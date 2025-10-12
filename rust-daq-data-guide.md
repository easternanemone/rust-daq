# Data Management and Storage Guide

## Overview

This guide covers data management strategies for Rust DAQ applications, including real-time buffering, data persistence, formats, and analysis pipelines optimized for performance and reliability.

## Data Architecture

### Core Data Types
```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: SystemTime,
    pub value: f64,
    pub channel: String,
    pub unit: Option<String>,
    pub quality: DataQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSet {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: SystemTime,
    pub metadata: HashMap<String, serde_json::Value>,
    pub channels: Vec<Channel>,
    pub data: Vec<DataPoint>,
    pub sample_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub name: String,
    pub unit: String,
    pub range: Option<(f64, f64)>,
    pub calibration: Option<Calibration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calibration {
    pub slope: f64,
    pub offset: f64,
    pub reference_date: SystemTime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DataQuality {
    Good,
    Suspect,
    Bad,
    Interpolated,
}
```

### Real-Time Buffering System
```rust
use ringbuf::{HeapRb, Rb, SharedRb};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::watch;

pub struct RealTimeBuffer<T> {
    buffer: Arc<RwLock<HeapRb<T>>>,
    capacity: usize,
    sample_rate: f64,
    overflow_count: Arc<RwLock<u64>>,
    data_notify: watch::Sender<bool>,
}

impl<T: Clone + Send + Sync + 'static> RealTimeBuffer<T> {
    pub fn new(capacity: usize, sample_rate: f64) -> Self {
        let (data_notify, _) = watch::channel(false);
        
        Self {
            buffer: Arc::new(RwLock::new(HeapRb::<T>::new(capacity))),
            capacity,
            sample_rate,
            overflow_count: Arc::new(RwLock::new(0)),
            data_notify,
        }
    }

    pub fn push(&self, item: T) -> Result<(), BufferError> {
        let mut buffer = self.buffer.write();
        
        if buffer.is_full() {
            // Handle overflow by dropping oldest data
            buffer.pop();
            *self.overflow_count.write() += 1;
        }
        
        buffer.push(item).map_err(|_| BufferError::Full)?;
        
        // Notify subscribers of new data
        let _ = self.data_notify.send(true);
        
        Ok(())
    }

    pub fn get_latest(&self, count: usize) -> Vec<T> {
        let buffer = self.buffer.read();
        buffer.iter().rev().take(count).cloned().collect()
    }

    pub fn get_range(&self, start_idx: usize, count: usize) -> Vec<T> {
        let buffer = self.buffer.read();
        buffer.iter()
            .skip(start_idx)
            .take(count)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.buffer.read().len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn overflow_count(&self) -> u64 {
        *self.overflow_count.read()
    }

    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.data_notify.subscribe()
    }

    pub fn clear(&self) {
        let mut buffer = self.buffer.write();
        buffer.clear();
        *self.overflow_count.write() = 0;
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BufferError {
    #[error("Buffer is full")]
    Full,
    
    #[error("Buffer is empty")]
    Empty,
    
    #[error("Invalid index: {0}")]
    InvalidIndex(usize),
}
```

### Multi-Channel Data Manager
```rust
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;

pub struct DataManager {
    channels: HashMap<String, RealTimeBuffer<DataPoint>>,
    datasets: Arc<RwLock<HashMap<Uuid, DataSet>>>,
    storage: Arc<dyn DataStorage>,
    config: DataManagerConfig,
    data_receiver: mpsc::Receiver<(String, DataPoint)>,
    subscribers: Vec<mpsc::Sender<DataEvent>>,
}

#[derive(Debug, Clone)]
pub struct DataManagerConfig {
    pub buffer_size: usize,
    pub auto_save_interval: std::time::Duration,
    pub compression_enabled: bool,
    pub max_memory_usage_mb: usize,
}

#[derive(Debug, Clone)]
pub enum DataEvent {
    NewData { channel: String, point: DataPoint },
    DatasetCreated { id: Uuid, name: String },
    DatasetSaved { id: Uuid, path: String },
    BufferOverflow { channel: String, count: u64 },
}

impl DataManager {
    pub fn new(
        config: DataManagerConfig,
        storage: Arc<dyn DataStorage>,
        data_receiver: mpsc::Receiver<(String, DataPoint)>,
    ) -> Self {
        Self {
            channels: HashMap::new(),
            datasets: Arc::new(RwLock::new(HashMap::new())),
            storage,
            config,
            data_receiver,
            subscribers: Vec::new(),
        }
    }

    pub async fn run(&mut self) -> Result<(), DataError> {
        let mut save_timer = tokio::time::interval(self.config.auto_save_interval);

        loop {
            tokio::select! {
                // Handle incoming data
                Some((channel, data_point)) = self.data_receiver.recv() => {
                    self.handle_data_point(channel, data_point).await?;
                }
                
                // Periodic auto-save
                _ = save_timer.tick() => {
                    self.auto_save_datasets().await?;
                }
                
                // Handle shutdown signal
                _ = tokio::signal::ctrl_c() => {
                    self.shutdown().await?;
                    break;
                }
            }
        }
        
        Ok(())
    }

    async fn handle_data_point(&mut self, channel: String, data_point: DataPoint) -> Result<(), DataError> {
        // Get or create buffer for channel
        let buffer = self.channels.entry(channel.clone())
            .or_insert_with(|| RealTimeBuffer::new(self.config.buffer_size, 1000.0));

        // Add data to buffer
        buffer.push(data_point.clone())?;

        // Notify subscribers
        let event = DataEvent::NewData { channel: channel.clone(), point: data_point };
        self.notify_subscribers(event).await;

        // Check for buffer overflow
        let overflow_count = buffer.overflow_count();
        if overflow_count > 0 {
            let event = DataEvent::BufferOverflow { channel, count: overflow_count };
            self.notify_subscribers(event).await;
        }

        Ok(())
    }

    pub async fn create_dataset(&self, name: String, channels: Vec<String>) -> Result<Uuid, DataError> {
        let id = Uuid::new_v4();
        let mut channel_configs = Vec::new();

        for channel_name in channels {
            channel_configs.push(Channel {
                name: channel_name.clone(),
                unit: "V".to_string(), // Default unit
                range: None,
                calibration: None,
            });
        }

        let dataset = DataSet {
            id,
            name: name.clone(),
            description: None,
            created_at: SystemTime::now(),
            metadata: HashMap::new(),
            channels: channel_configs,
            data: Vec::new(),
            sample_rate: 1000.0,
        };

        self.datasets.write().await.insert(id, dataset);

        let event = DataEvent::DatasetCreated { id, name };
        self.notify_subscribers(event).await;

        Ok(id)
    }

    pub async fn export_dataset(&self, dataset_id: Uuid, format: ExportFormat) -> Result<String, DataError> {
        let datasets = self.datasets.read().await;
        let dataset = datasets.get(&dataset_id)
            .ok_or(DataError::DatasetNotFound(dataset_id))?;

        // Collect current data from buffers
        let mut full_dataset = dataset.clone();
        for channel in &dataset.channels {
            if let Some(buffer) = self.channels.get(&channel.name) {
                let data = buffer.get_latest(buffer.len());
                full_dataset.data.extend(data);
            }
        }

        // Save using appropriate format
        let path = self.storage.save_dataset(&full_dataset, format).await?;

        let event = DataEvent::DatasetSaved { id: dataset_id, path: path.clone() };
        self.notify_subscribers(event).await;

        Ok(path)
    }

    async fn auto_save_datasets(&self) -> Result<(), DataError> {
        let datasets = self.datasets.read().await;
        
        for (id, dataset) in datasets.iter() {
            // Check if dataset needs saving (implement your logic here)
            if self.should_auto_save(dataset) {
                drop(datasets); // Release read lock
                self.export_dataset(*id, ExportFormat::HDF5).await?;
                break; // Re-acquire lock on next iteration
            }
        }
        
        Ok(())
    }

    fn should_auto_save(&self, dataset: &DataSet) -> bool {
        // Implement auto-save logic based on time, data size, etc.
        let age = dataset.created_at.elapsed().unwrap_or_default();
        age > self.config.auto_save_interval
    }

    pub async fn subscribe(&mut self) -> mpsc::Receiver<DataEvent> {
        let (sender, receiver) = mpsc::channel(1000);
        self.subscribers.push(sender);
        receiver
    }

    async fn notify_subscribers(&self, event: DataEvent) {
        for subscriber in &self.subscribers {
            let _ = subscriber.try_send(event.clone());
        }
    }

    async fn shutdown(&self) -> Result<(), DataError> {
        // Save all active datasets
        let datasets = self.datasets.read().await;
        for (id, _) in datasets.iter() {
            drop(datasets);
            self.export_dataset(*id, ExportFormat::HDF5).await?;
            break;
        }
        
        Ok(())
    }
}
```

### Data Storage Backends

#### HDF5 Storage Implementation
```rust
use hdf5::{File, Group, Dataset as H5Dataset};
use ndarray::{Array1, Array2};

pub struct HDF5Storage {
    base_path: std::path::PathBuf,
}

#[async_trait::async_trait]
impl DataStorage for HDF5Storage {
    async fn save_dataset(&self, dataset: &DataSet, _format: ExportFormat) -> Result<String, DataError> {
        let filename = format!("{}_{}.h5", 
            dataset.name.replace(' ', "_"), 
            dataset.id
        );
        let path = self.base_path.join(&filename);

        // Create HDF5 file
        let file = File::create(&path)?;
        
        // Create groups
        let data_group = file.create_group("data")?;
        let metadata_group = file.create_group("metadata")?;

        // Save metadata
        self.save_metadata(&metadata_group, dataset)?;

        // Organize data by channel
        let mut channel_data: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
        
        for point in &dataset.data {
            let timestamp = point.timestamp
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            
            channel_data.entry(point.channel.clone())
                .or_insert_with(Vec::new)
                .push((timestamp, point.value));
        }

        // Save each channel as a separate dataset
        for (channel_name, data) in channel_data {
            let timestamps: Vec<f64> = data.iter().map(|(t, _)| *t).collect();
            let values: Vec<f64> = data.iter().map(|(_, v)| *v).collect();

            let channel_group = data_group.create_group(&channel_name)?;
            
            let timestamp_array = Array1::from_vec(timestamps);
            let values_array = Array1::from_vec(values);

            channel_group.new_dataset::<f64>()
                .with_data(&timestamp_array)
                .create("timestamps")?;
                
            channel_group.new_dataset::<f64>()
                .with_data(&values_array)
                .create("values")?;
        }

        Ok(path.to_string_lossy().to_string())
    }

    async fn load_dataset(&self, path: &str) -> Result<DataSet, DataError> {
        let file = File::open(path)?;
        
        // Load metadata
        let metadata_group = file.group("metadata")?;
        let dataset_info = self.load_metadata(&metadata_group)?;

        // Load data
        let data_group = file.group("data")?;
        let mut all_data = Vec::new();

        for group_name in data_group.member_names()? {
            let channel_group = data_group.group(&group_name)?;
            
            let timestamps: Array1<f64> = channel_group.dataset("timestamps")?.read()?;
            let values: Array1<f64> = channel_group.dataset("values")?.read()?;

            for (timestamp, value) in timestamps.iter().zip(values.iter()) {
                let system_time = std::time::UNIX_EPOCH + 
                    std::time::Duration::from_secs_f64(*timestamp);
                
                all_data.push(DataPoint {
                    timestamp: system_time,
                    value: *value,
                    channel: group_name.clone(),
                    unit: Some("V".to_string()),
                    quality: DataQuality::Good,
                });
            }
        }

        Ok(DataSet {
            id: dataset_info.id,
            name: dataset_info.name,
            description: dataset_info.description,
            created_at: dataset_info.created_at,
            metadata: dataset_info.metadata,
            channels: dataset_info.channels,
            data: all_data,
            sample_rate: dataset_info.sample_rate,
        })
    }
}
```

#### CSV Storage Implementation
```rust
use polars::prelude::*;
use std::path::Path;

pub struct CSVStorage {
    base_path: std::path::PathBuf,
}

#[async_trait::async_trait]
impl DataStorage for CSVStorage {
    async fn save_dataset(&self, dataset: &DataSet, _format: ExportFormat) -> Result<String, DataError> {
        let filename = format!("{}_{}.csv", 
            dataset.name.replace(' ', "_"), 
            dataset.id
        );
        let path = self.base_path.join(&filename);

        // Convert data to DataFrame
        let mut timestamps = Vec::new();
        let mut values = Vec::new();
        let mut channels = Vec::new();
        let mut qualities = Vec::new();

        for point in &dataset.data {
            let timestamp_ms = point.timestamp
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            
            timestamps.push(timestamp_ms);
            values.push(point.value);
            channels.push(point.channel.clone());
            qualities.push(format!("{:?}", point.quality));
        }

        let df = DataFrame::new(vec![
            Column::new("timestamp_ms", timestamps),
            Column::new("value", values),
            Column::new("channel", channels),
            Column::new("quality", qualities),
        ])?;

        // Write to CSV
        let mut file = std::fs::File::create(&path)?;
        CsvWriter::new(&mut file)
            .has_header(true)
            .finish(&mut df.clone())?;

        Ok(path.to_string_lossy().to_string())
    }

    async fn load_dataset(&self, path: &str) -> Result<DataSet, DataError> {
        let df = LazyFrame::scan_csv(path, ScanArgsCSV::default())?
            .collect()?;

        let mut data_points = Vec::new();
        
        let timestamps = df.column("timestamp_ms")?.i64()?;
        let values = df.column("value")?.f64()?;
        let channels = df.column("channel")?.utf8()?;
        let qualities = df.column("quality")?.utf8()?;

        for i in 0..df.height() {
            let timestamp_ms = timestamps.get(i).unwrap_or(0);
            let timestamp = std::time::UNIX_EPOCH + 
                std::time::Duration::from_millis(timestamp_ms as u64);

            let quality = match qualities.get(i).unwrap_or("Good") {
                "Good" => DataQuality::Good,
                "Suspect" => DataQuality::Suspect,
                "Bad" => DataQuality::Bad,
                "Interpolated" => DataQuality::Interpolated,
                _ => DataQuality::Good,
            };

            data_points.push(DataPoint {
                timestamp,
                value: values.get(i).unwrap_or(0.0),
                channel: channels.get(i).unwrap_or("").to_string(),
                unit: Some("V".to_string()),
                quality,
            });
        }

        // Create basic dataset info (metadata would need to be loaded separately)
        Ok(DataSet {
            id: Uuid::new_v4(),
            name: Path::new(path).file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            description: None,
            created_at: SystemTime::now(),
            metadata: HashMap::new(),
            channels: Vec::new(), // Would need to be inferred or loaded separately
            data: data_points,
            sample_rate: 1000.0, // Default value
        })
    }
}
```

### Data Processing Pipeline
```rust
use async_trait::async_trait;

#[async_trait]
pub trait DataProcessor: Send + Sync {
    type Input: Send + Clone;
    type Output: Send + Clone;
    type Error: std::error::Error + Send + Sync + 'static;

    async fn process(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
    fn name(&self) -> &str;
}

pub struct FilterProcessor {
    filter_type: FilterType,
    parameters: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
    MovingAverage,
}

#[async_trait]
impl DataProcessor for FilterProcessor {
    type Input = Vec<DataPoint>;
    type Output = Vec<DataPoint>;
    type Error = ProcessingError;

    async fn process(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        match self.filter_type {
            FilterType::MovingAverage => {
                let window_size = self.parameters.get("window_size")
                    .copied()
                    .unwrap_or(5.0) as usize;
                
                self.apply_moving_average(input, window_size)
            }
            // Implement other filter types
            _ => Ok(input),
        }
    }

    fn name(&self) -> &str {
        "FilterProcessor"
    }
}

impl FilterProcessor {
    fn apply_moving_average(&self, mut data: Vec<DataPoint>, window_size: usize) -> Result<Vec<DataPoint>, ProcessingError> {
        if window_size == 0 || data.len() < window_size {
            return Ok(data);
        }

        for i in window_size..data.len() {
            let window_sum: f64 = data[i-window_size..i]
                .iter()
                .map(|p| p.value)
                .sum();
            
            data[i].value = window_sum / window_size as f64;
        }

        Ok(data)
    }
}
```

This data management guide provides a comprehensive framework for handling scientific data with real-time requirements, multiple storage formats, and processing capabilities while maintaining performance and reliability.