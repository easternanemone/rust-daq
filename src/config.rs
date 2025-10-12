//! Configuration management.
use crate::error::DaqError;
use config::Config;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub log_level: String,
    pub storage: StorageSettings,
    pub instruments: HashMap<String, toml::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StorageSettings {
    pub default_path: String,
    pub default_format: String,
}

impl Settings {
    pub fn new() -> Result<Self, DaqError> {
        let s = Config::builder()
            .add_source(config::File::with_name("config/default"))
            .build()
            .map_err(DaqError::Config)?;

        s.try_deserialize().map_err(DaqError::Config)
    }
}
