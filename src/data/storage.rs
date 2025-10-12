//! Example data storage writers.
use crate::{
    config::Settings,
    core::{DataPoint, StorageWriter},
    error::DaqError,
};
use async_trait::async_trait;
use std::sync::Arc;

/// A writer for CSV files.
pub struct CsvWriter {
    // csv::Writer is not async, so we'd need to wrap it or use a blocking task.
    // This is a placeholder for the actual implementation.
}

impl CsvWriter {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl StorageWriter for CsvWriter {
    async fn init(&mut self, _settings: &Arc<Settings>) -> Result<(), DaqError> {
        #[cfg(not(feature = "storage_csv"))]
        return Err(DaqError::FeatureNotEnabled("storage_csv".to_string()));

        #[cfg(feature = "storage_csv")]
        {
            // TODO: Create and open the CSV file, write header.
            log::info!("CSV Writer initialized (placeholder).");
            Ok(())
        }
    }

    async fn write(&mut self, _data: &[DataPoint]) -> Result<(), DaqError> {
        #[cfg(feature = "storage_csv")]
        {
            // TODO: Write data points to the CSV file.
            Ok(())
        }
        #[cfg(not(feature = "storage_csv"))]
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), DaqError> {
        #[cfg(feature = "storage_csv")]
        {
            log::info!("CSV Writer shut down (placeholder).");
            Ok(())
        }
        #[cfg(not(feature = "storage_csv"))]
        Ok(())
    }
}

// Skeletons for other writers
pub struct Hdf5Writer;
#[async_trait]
impl StorageWriter for Hdf5Writer {
    async fn init(&mut self, _settings: &Arc<Settings>) -> Result<(), DaqError> {
        Err(DaqError::FeatureNotEnabled("storage_hdf5".to_string()))
    }
    async fn write(&mut self, _data: &[DataPoint]) -> Result<(), DaqError> {
        Ok(())
    }
    async fn shutdown(&mut self) -> Result<(), DaqError> {
        Ok(())
    }
}
