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
    pub processors: Option<HashMap<String, Vec<ProcessorConfig>>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProcessorConfig {
    pub r#type: String,
    #[serde(flatten)]
    pub config: toml::Value,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StorageSettings {
    pub default_path: String,
    pub default_format: String,
}

impl Settings {
    pub fn new(config_name: Option<&str>) -> Result<Self, DaqError> {
        let config_path = format!("config/{}", config_name.unwrap_or("default"));
        let s = Config::builder()
            .add_source(config::File::with_name(&config_path))
            .build()
            .map_err(DaqError::Config)?;

        s.try_deserialize().map_err(DaqError::Config)
    }
}
