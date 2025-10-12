# Performance Optimization and Testing Guide

## Overview

This guide covers performance optimization strategies, benchmarking, profiling, and comprehensive testing approaches for scientific data acquisition applications in Rust, ensuring optimal real-time performance and reliability.

## Performance Optimization Strategies

### Memory Management Optimization

#### Zero-Copy Data Handling
```rust
use std::sync::Arc;
use bytes::{Bytes, BytesMut};
use memmap2::MmapOptions;

// Use Arc for shared ownership without copying
pub struct SharedData {
    data: Arc<[f64]>,
    metadata: Arc<DataMetadata>,
}

impl SharedData {
    pub fn new(data: Vec<f64>, metadata: DataMetadata) -> Self {
        Self {
            data: Arc::from(data.into_boxed_slice()),
            metadata: Arc::new(metadata),
        }
    }

    pub fn clone_handle(&self) -> Self {
        Self {
            data: self.data.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

// Memory-mapped files for large datasets
pub struct MmapDataset {
    mmap: memmap2::Mmap,
    len: usize,
}

impl MmapDataset {
    pub fn open(path: &std::path::Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let len = mmap.len() / std::mem::size_of::<f64>();
        
        Ok(Self { mmap, len })
    }

    pub fn as_slice(&self) -> &[f64] {
        unsafe {
            std::slice::from_raw_parts(
                self.mmap.as_ptr() as *const f64,
                self.len,
            )
        }
    }
}
```

#### Pool-Based Allocation
```rust
use object_pool::{Pool, Reusable};
use std::sync::Arc;

pub struct BufferPool {
    pool: Arc<Pool<Vec<f64>>>,
}

impl BufferPool {
    pub fn new(capacity: usize, buffer_size: usize) -> Self {
        let pool = Pool::new(capacity, || Vec::with_capacity(buffer_size));
        Self {
            pool: Arc::new(pool),
        }
    }

    pub fn get_buffer(&self) -> Reusable<Vec<f64>> {
        let mut buffer = self.pool.try_pull().unwrap_or_else(|| {
            self.pool.attach(Vec::new())
        });
        buffer.clear();
        buffer
    }

    pub fn return_buffer(&self, mut buffer: Reusable<Vec<f64>>) {
        buffer.clear();
        // Buffer automatically returns to pool when dropped
    }
}

// Usage in data acquisition
pub struct OptimizedAcquisition {
    buffer_pool: BufferPool,
    processing_pool: Arc<rayon::ThreadPool>,
}

impl OptimizedAcquisition {
    pub async fn process_data_batch(&self, raw_data: &[u8]) -> Result<ProcessedData, AcquisitionError> {
        let buffer = self.buffer_pool.get_buffer();
        
        // Parse data into reused buffer
        self.parse_into_buffer(raw_data, &mut *buffer)?;
        
        // Process on thread pool
        let processed = self.processing_pool.install(|| {
            self.apply_processing(&buffer)
        })?;
        
        Ok(processed)
    }
}
```

### SIMD Optimization for Signal Processing
```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use wide::f64x4;

pub struct SIMDProcessor;

impl SIMDProcessor {
    pub fn apply_filter_simd(input: &[f64], coefficients: &[f64]) -> Vec<f64> {
        let mut output = vec![0.0; input.len()];
        
        // Process 4 elements at a time using SIMD
        let chunks = input.chunks_exact(4);
        let remainder = chunks.remainder();
        
        for (i, chunk) in chunks.enumerate() {
            let data = f64x4::from(chunk);
            let filtered = Self::apply_filter_chunk_simd(data, coefficients);
            output[i * 4..(i + 1) * 4].copy_from_slice(&filtered.to_array());
        }
        
        // Handle remainder
        for (i, &value) in remainder.iter().enumerate() {
            output[input.len() - remainder.len() + i] = 
                Self::apply_filter_scalar(value, coefficients);
        }
        
        output
    }

    #[inline]
    fn apply_filter_chunk_simd(data: f64x4, coefficients: &[f64]) -> f64x4 {
        // Implement SIMD filtering logic
        let mut result = f64x4::splat(0.0);
        
        for &coeff in coefficients {
            result = result + data * f64x4::splat(coeff);
        }
        
        result
    }

    fn apply_filter_scalar(value: f64, coefficients: &[f64]) -> f64 {
        coefficients.iter().sum::<f64>() * value
    }

    // Fast Fourier Transform using SIMD
    pub fn fft_simd(input: &[f64]) -> Vec<std::num::Complex<f64>> {
        // Implement optimized FFT using SIMD instructions
        // This is a simplified example - use a proper FFT library like rustfft
        rustfft::FftPlanner::new()
            .plan_fft_forward(input.len())
            .process(&mut input.iter().map(|&x| std::num::Complex::new(x, 0.0)).collect::<Vec<_>>())
    }
}
```

### Async Performance Optimization
```rust
use tokio::sync::{mpsc, oneshot};
use futures::{stream::StreamExt, sink::SinkExt};
use std::time::Duration;

pub struct OptimizedDataPipeline {
    batch_size: usize,
    processing_timeout: Duration,
    worker_pool: Vec<tokio::task::JoinHandle<()>>,
}

impl OptimizedDataPipeline {
    pub fn new(num_workers: usize, batch_size: usize) -> Self {
        let mut worker_pool = Vec::new();
        
        for worker_id in 0..num_workers {
            let handle = tokio::spawn(async move {
                Self::worker_loop(worker_id).await;
            });
            worker_pool.push(handle);
        }
        
        Self {
            batch_size,
            processing_timeout: Duration::from_millis(10),
            worker_pool,
        }
    }

    async fn worker_loop(worker_id: usize) {
        let mut interval = tokio::time::interval(Duration::from_micros(100));
        
        loop {
            interval.tick().await;
            
            // Process batched data
            if let Some(batch) = Self::get_work_batch(worker_id).await {
                Self::process_batch(batch).await;
            }
        }
    }

    // Batched processing to reduce context switching
    pub async fn process_stream<T>(&self, mut input: mpsc::Receiver<T>) -> mpsc::Receiver<ProcessedData>
    where
        T: Send + 'static,
    {
        let (output_tx, output_rx) = mpsc::channel(1000);
        let mut batch = Vec::with_capacity(self.batch_size);
        
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Collect items into batches
                    item = input.recv() => {
                        match item {
                            Some(data) => {
                                batch.push(data);
                                
                                if batch.len() >= self.batch_size {
                                    let processing_batch = std::mem::take(&mut batch);
                                    Self::process_and_send_batch(processing_batch, &output_tx).await;
                                }
                            }
                            None => break,
                        }
                    }
                    
                    // Timeout to process partial batches
                    _ = tokio::time::sleep(self.processing_timeout) => {
                        if !batch.is_empty() {
                            let processing_batch = std::mem::take(&mut batch);
                            Self::process_and_send_batch(processing_batch, &output_tx).await;
                        }
                    }
                }
            }
        });
        
        output_rx
    }
}
```

## Benchmarking and Profiling

### Comprehensive Benchmarking Suite
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;

fn benchmark_data_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_processing");
    
    // Test different data sizes
    for size in [1000, 10000, 100000, 1000000].iter() {
        let data: Vec<f64> = (0..*size).map(|i| i as f64 * 0.001).collect();
        
        group.bench_with_input(
            BenchmarkId::new("scalar_processing", size),
            &data,
            |b, data| {
                b.iter(|| {
                    black_box(scalar_process(black_box(data)))
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("simd_processing", size),
            &data,
            |b, data| {
                b.iter(|| {
                    black_box(SIMDProcessor::apply_filter_simd(black_box(data), &[0.1, 0.2, 0.3]))
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("parallel_processing", size),
            &data,
            |b, data| {
                b.iter(|| {
                    black_box(parallel_process(black_box(data)))
                })
            },
        );
    }
    
    group.finish();
}

fn benchmark_buffer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_operations");
    group.measurement_time(Duration::from_secs(10));
    
    for buffer_size in [1000, 10000, 100000].iter() {
        let buffer = RealTimeBuffer::<f64>::new(*buffer_size, 1000.0);
        
        group.bench_with_input(
            BenchmarkId::new("push_operation", buffer_size),
            buffer_size,
            |b, _| {
                b.iter(|| {
                    for i in 0..1000 {
                        buffer.push(black_box(i as f64)).unwrap();
                    }
                })
            },
        );
        
        // Fill buffer first
        for i in 0..*buffer_size {
            buffer.push(i as f64).unwrap();
        }
        
        group.bench_with_input(
            BenchmarkId::new("get_latest", buffer_size),
            buffer_size,
            |b, _| {
                b.iter(|| {
                    black_box(buffer.get_latest(black_box(1000)))
                })
            },
        );
    }
    
    group.finish();
}

fn benchmark_async_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("async_data_pipeline", |b| {
        b.to_async(&rt).iter(|| async {
            let (tx, rx) = tokio::sync::mpsc::channel(1000);
            let pipeline = OptimizedDataPipeline::new(4, 100);
            
            // Send test data
            for i in 0..1000 {
                tx.send(black_box(i as f64)).await.unwrap();
            }
            
            // Process data
            let mut processed_rx = pipeline.process_stream(rx).await;
            let mut count = 0;
            
            while let Some(_) = processed_rx.recv().await {
                count += 1;
                if count >= 1000 {
                    break;
                }
            }
            
            black_box(count)
        })
    });
}

criterion_group!(
    benches,
    benchmark_data_processing,
    benchmark_buffer_operations,
    benchmark_async_performance
);
criterion_main!(benches);
```

### Memory Usage Profiling
```rust
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct ProfilingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for ProfilingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc(layout);
        if !ret.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        DEALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static GLOBAL: ProfilingAllocator = ProfilingAllocator;

pub fn get_memory_stats() -> (usize, usize, usize) {
    let allocated = ALLOCATED.load(Ordering::SeqCst);
    let deallocated = DEALLOCATED.load(Ordering::SeqCst);
    let current = allocated.saturating_sub(deallocated);
    (allocated, deallocated, current)
}

#[cfg(test)]
mod memory_tests {
    use super::*;
    
    #[test]
    fn test_memory_usage() {
        let (initial_alloc, initial_dealloc, initial_current) = get_memory_stats();
        
        {
            let _data: Vec<f64> = vec![0.0; 1000000];
            let (mid_alloc, mid_dealloc, mid_current) = get_memory_stats();
            
            assert!(mid_current > initial_current);
            println!("Memory usage increased by {} bytes", mid_current - initial_current);
        }
        
        // Force garbage collection if needed
        std::hint::black_box(());
        
        let (final_alloc, final_dealloc, final_current) = get_memory_stats();
        println!("Final memory usage: {} bytes", final_current);
    }
}
```

## Comprehensive Testing Strategy

### Unit Testing with Mock Instruments
```rust
use mockall::{automock, predicate::*};
use tokio_test;

#[automock]
pub trait MockableInstrument {
    async fn read_data(&mut self) -> Result<Vec<f64>, InstrumentError>;
    async fn send_command(&mut self, cmd: &str) -> Result<String, InstrumentError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_data_acquisition_flow() {
        let mut mock_instrument = MockMockableInstrument::new();
        
        // Setup expectations
        mock_instrument
            .expect_read_data()
            .times(3)
            .returning(|| Ok(vec![1.0, 2.0, 3.0, 4.0]));
        
        mock_instrument
            .expect_send_command()
            .with(eq("*IDN?"))
            .returning(|_| Ok("Mock Instrument".to_string()));
        
        // Test the acquisition system
        let mut acquisition = DataAcquisition::new(Box::new(mock_instrument));
        
        let identification = acquisition.identify().await.unwrap();
        assert_eq!(identification, "Mock Instrument");
        
        for _ in 0..3 {
            let data = acquisition.read_data().await.unwrap();
            assert_eq!(data.len(), 4);
            assert_eq!(data[0], 1.0);
        }
    }
    
    #[tokio::test]
    async fn test_buffer_overflow_handling() {
        let buffer = RealTimeBuffer::<f64>::new(10, 1000.0);
        
        // Fill buffer beyond capacity
        for i in 0..20 {
            buffer.push(i as f64).unwrap();
        }
        
        // Check overflow handling
        assert_eq!(buffer.len(), 10);
        assert!(buffer.overflow_count() > 0);
        
        // Verify latest data is preserved
        let latest = buffer.get_latest(5);
        assert_eq!(latest.len(), 5);
        assert_eq!(latest[0], 19.0); // Most recent
    }
}
```

### Integration Testing
```rust
use tempfile::TempDir;
use std::time::Duration;

#[tokio::test]
async fn test_end_to_end_data_flow() {
    let temp_dir = TempDir::new().unwrap();
    let config = DataManagerConfig {
        buffer_size: 1000,
        auto_save_interval: Duration::from_millis(100),
        compression_enabled: false,
        max_memory_usage_mb: 100,
    };
    
    let storage = Arc::new(CSVStorage::new(temp_dir.path().to_path_buf()));
    let (data_tx, data_rx) = mpsc::channel(1000);
    let mut data_manager = DataManager::new(config, storage, data_rx);
    
    // Start data manager in background
    let manager_task = tokio::spawn(async move {
        data_manager.run().await
    });
    
    // Create dataset
    let dataset_id = data_manager.create_dataset(
        "test_dataset".to_string(),
        vec!["channel1".to_string(), "channel2".to_string()]
    ).await.unwrap();
    
    // Send test data
    for i in 0..100 {
        let point = DataPoint {
            timestamp: SystemTime::now(),
            value: i as f64,
            channel: "channel1".to_string(),
            unit: Some("V".to_string()),
            quality: DataQuality::Good,
        };
        
        data_tx.send(("channel1".to_string(), point)).await.unwrap();
    }
    
    // Wait for processing
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Export and verify
    let export_path = data_manager.export_dataset(dataset_id, ExportFormat::CSV).await.unwrap();
    assert!(std::path::Path::new(&export_path).exists());
    
    // Clean up
    drop(data_tx);
    manager_task.abort();
}
```

### Performance Testing
```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn test_high_throughput_performance() {
    let throughput_counter = Arc::new(AtomicU64::new(0));
    let error_counter = Arc::new(AtomicU64::new(0));
    
    let (tx, mut rx) = mpsc::channel(10000);
    
    // Producer task - high rate data generation
    let producer_counter = throughput_counter.clone();
    let producer_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_micros(10)); // 100kHz
        let mut sample_count = 0u64;
        
        loop {
            interval.tick().await;
            
            let data_point = DataPoint {
                timestamp: SystemTime::now(),
                value: (sample_count as f64 * 0.001).sin(),
                channel: "high_speed_channel".to_string(),
                unit: Some("V".to_string()),
                quality: DataQuality::Good,
            };
            
            match tx.try_send(data_point) {
                Ok(_) => {
                    producer_counter.fetch_add(1, Ordering::Relaxed);
                    sample_count += 1;
                }
                Err(_) => break, // Channel full or closed
            }
            
            if sample_count >= 100000 {
                break;
            }
        }
    });
    
    // Consumer task - processing
    let consumer_counter = throughput_counter.clone();
    let consumer_errors = error_counter.clone();
    let consumer_task = tokio::spawn(async move {
        let mut processed_count = 0u64;
        
        while let Some(data_point) = rx.recv().await {
            // Simulate processing time
            tokio::task::yield_now().await;
            
            // Validate data
            if data_point.value.is_finite() {
                consumer_counter.fetch_add(1, Ordering::Relaxed);
                processed_count += 1;
            } else {
                consumer_errors.fetch_add(1, Ordering::Relaxed);
            }
            
            if processed_count >= 100000 {
                break;
            }
        }
    });
    
    // Run test for limited time
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(10);
    
    tokio::select! {
        _ = producer_task => {},
        _ = consumer_task => {},
        _ = tokio::time::sleep(timeout) => {
            println!("Test timed out after {:?}", timeout);
        }
    }
    
    let elapsed = start_time.elapsed();
    let total_throughput = throughput_counter.load(Ordering::Relaxed);
    let total_errors = error_counter.load(Ordering::Relaxed);
    
    println!("Performance Results:");
    println!("  Duration: {:?}", elapsed);
    println!("  Total samples: {}", total_throughput);
    println!("  Throughput: {:.2} samples/sec", total_throughput as f64 / elapsed.as_secs_f64());
    println!("  Error rate: {:.4}%", (total_errors as f64 / total_throughput as f64) * 100.0);
    
    // Performance assertions
    let samples_per_second = total_throughput as f64 / elapsed.as_secs_f64();
    assert!(samples_per_second > 50000.0, "Throughput below 50kHz: {:.2}", samples_per_second);
    assert!(total_errors == 0, "Errors detected: {}", total_errors);
}
```

### Load Testing
```rust
#[tokio::test]
async fn test_system_under_load() {
    let num_concurrent_instruments = 10;
    let samples_per_instrument = 10000;
    
    let mut tasks = Vec::new();
    let (global_tx, mut global_rx) = mpsc::channel(100000);
    
    // Spawn multiple concurrent instrument simulators
    for instrument_id in 0..num_concurrent_instruments {
        let tx = global_tx.clone();
        
        let task = tokio::spawn(async move {
            let mut instrument = MockInstrument::new(MockConfig {
                channels: 4,
                sample_rate: 1000.0,
                amplitude: 5.0,
                frequency: 50.0,
                noise_level: 0.1,
            });
            
            instrument.initialize(instrument.config.clone()).await.unwrap();
            instrument.start_acquisition().await.unwrap();
            
            for sample_id in 0..samples_per_instrument {
                match instrument.read_data().await {
                    Ok(data) => {
                        let msg = format!("instrument_{}_sample_{}", instrument_id, sample_id);
                        if tx.send(msg).await.is_err() {
                            break; // Receiver dropped
                        }
                    }
                    Err(e) => {
                        eprintln!("Instrument {} error: {}", instrument_id, e);
                        break;
                    }
                }
                
                // Simulate realistic timing
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });
        
        tasks.push(task);
    }
    
    // Process received data
    let mut received_count = 0;
    let expected_total = num_concurrent_instruments * samples_per_instrument;
    
    drop(global_tx); // Close sender so receiver will eventually finish
    
    while let Some(_message) = global_rx.recv().await {
        received_count += 1;
        
        if received_count % 10000 == 0 {
            println!("Processed {} / {} messages", received_count, expected_total);
        }
    }
    
    // Wait for all tasks to complete
    for task in tasks {
        task.await.unwrap();
    }
    
    println!("Load test completed:");
    println!("  Expected messages: {}", expected_total);
    println!("  Received messages: {}", received_count);
    println!("  Success rate: {:.2}%", (received_count as f64 / expected_total as f64) * 100.0);
    
    assert_eq!(received_count, expected_total, "Message loss detected");
}
```

This performance and testing guide provides comprehensive strategies for optimizing and validating your scientific data acquisition application, ensuring it meets the demanding requirements of real-time scientific instrumentation.