//! The core application state and logic.
use crate::{
    config::Settings,
    core::{DataPoint, DataProcessor, InstrumentHandle},
    data::registry::ProcessorRegistry,
    error::DaqError,
    instrument::InstrumentRegistry,
    log_capture::LogBuffer,
};
use anyhow::Result;
use log::{error, info};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::{runtime::Runtime, sync::broadcast, task::JoinHandle};

/// The main application struct that holds all state.
#[derive(Clone)]
pub struct DaqApp {
    inner: Arc<Mutex<DaqAppInner>>,
}

/// Inner state of the DAQ application, protected by a Mutex.
pub struct DaqAppInner {
    pub settings: Arc<Settings>,
    pub instrument_registry: Arc<InstrumentRegistry>,
    pub processor_registry: Arc<ProcessorRegistry>,
    pub instruments: HashMap<String, InstrumentHandle>,
    pub data_sender: broadcast::Sender<DataPoint>,
    pub log_buffer: LogBuffer,
    runtime: Arc<Runtime>,
    shutdown_flag: bool,
}

impl DaqApp {
    /// Creates a new `DaqApp`.
    pub fn new(
        settings: Arc<Settings>,
        instrument_registry: Arc<InstrumentRegistry>,
        processor_registry: Arc<ProcessorRegistry>,
        log_buffer: LogBuffer,
    ) -> Result<Self> {
        let runtime = Arc::new(Runtime::new().map_err(DaqError::Tokio)?);
        let (data_sender, _) = broadcast::channel(1024);

        let mut inner = DaqAppInner {
            settings: settings.clone(),
            instrument_registry,
            processor_registry,
            instruments: HashMap::new(),
            data_sender,
            log_buffer,
            runtime,
            shutdown_flag: false,
        };

        for (id, _instrument_config) in &settings.instruments {
            if let Err(e) = inner.spawn_instrument(id) {
                error!("Failed to spawn instrument '{}': {}", id, e);
            }
        }

        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    /// Provides safe access to the inner application state.
    pub fn with_inner<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut DaqAppInner) -> R,
    {
        let mut inner = self.inner.lock().unwrap();
        f(&mut *inner)
    }

    /// Returns a clone of the application's Tokio runtime handle.
    pub fn get_runtime(&self) -> Arc<Runtime> {
        self.with_inner(|inner| inner.runtime.clone())
    }

    /// Shuts down the application, stopping all instruments and the Tokio runtime.
    pub fn shutdown(&self) {
        self.with_inner(|inner| {
            if inner.shutdown_flag {
                return;
            }
            info!("Shutting down application runtime...");
            inner.shutdown_flag = true;
            // Stop all instruments
            for (id, handle) in inner.instruments.drain() {
                info!("Stopping instrument: {}", id);
                handle.task.abort();
            }
        });
    }
}

impl DaqAppInner {
    /// Spawns an instrument to run on the Tokio runtime.
    pub fn spawn_instrument(&mut self, id: &str) -> Result<(), DaqError> {
        if self.instruments.contains_key(id) {
            return Err(DaqError::Instrument(format!(
                "Instrument '{}' is already running.",
                id
            )));
        }

        let instrument_config = self.settings.instruments.get(id)
            .ok_or_else(|| DaqError::Config(config::ConfigError::NotFound("instrument".to_string())))?;
        let instrument_type = instrument_config.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DaqError::Config(config::ConfigError::NotFound("type".to_string())))?;

        let mut instrument = self
            .instrument_registry
            .create(instrument_type, id)
            .ok_or_else(|| DaqError::Instrument(format!("Instrument type '{}' not found.", instrument_type)))?;

        // Create processor chain for this instrument
        let mut processors: Vec<Box<dyn DataProcessor>> = Vec::new();
        if let Some(processor_configs) = self.settings.processors.as_ref().and_then(|p| p.get(id)) {
            for config in processor_configs {
                let processor = self.processor_registry.create(&config.r#type, &config.config)?;
                processors.push(processor);
            }
        }

        let data_sender = self.data_sender.clone();
        let settings = self.settings.clone();
        let id_clone = id.to_string();

        let task: JoinHandle<Result<()>> = self.runtime.spawn(async move {
            instrument.connect(&settings).await?;
            info!("Instrument '{}' connected.", id_clone);

            let mut stream = instrument.data_stream().await?;
            loop {
                tokio::select! {
                    data_point_result = stream.recv() => {
                        match data_point_result {
                            Ok(dp) => {
                                let mut data_points = vec![dp];
                                for processor in &mut processors {
                                    data_points = processor.process(&data_points);
                                }

                                for processed_dp in data_points {
                                    if let Err(e) = data_sender.send(processed_dp) {
                                        error!("Failed to broadcast data point: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Stream receive error: {}", e);
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                        log::trace!("Instrument stream for {} is idle.", id_clone);
                    }
                }
            }
            Ok(())
        });

        let handle = InstrumentHandle { task };
        self.instruments.insert(id.to_string(), handle);
        Ok(())
    }

    /// Stops a running instrument.
    pub fn stop_instrument(&mut self, id: &str) {
        if let Some(handle) = self.instruments.remove(id) {
            handle.task.abort();
            info!("Instrument '{}' stopped.", id);
        }
    }

    /// Returns a list of available channel names.
    pub fn get_available_channels(&self) -> Vec<String> {
        self.instrument_registry.list().collect()
    }
}
