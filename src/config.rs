//! Configuration management.
use anyhow::{Context, Result};
use config::Config;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StorageSettings {
    pub default_path: String,
    pub default_format: String,
}

impl Settings {
    pub fn new(config_name: Option<&str>) -> Result<Self> {
        let config_path = format!("config/{}", config_name.unwrap_or("default"));
        let s = Config::builder()
            .add_source(config::File::with_name(&config_path))
            .build()
            .with_context(|| format!("Failed to load configuration from '{}'", config_path))?;

        s.try_deserialize()
            .context("Failed to deserialize configuration")
    }
}
