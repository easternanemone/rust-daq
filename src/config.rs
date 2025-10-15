//! Configuration management.
use crate::error::DaqError;
use crate::validation::{is_in_range, is_not_empty, is_valid_ip, is_valid_path, is_valid_port};
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
    pub fn new(config_name: Option<&str>) -> Result<Self, DaqError> {
        let config_path = format!("config/{}", config_name.unwrap_or("default"));
        let s = Config::builder()
            .add_source(config::File::with_name(&config_path))
            .build()
            .map_err(DaqError::Config)?;

        let settings: Settings = s.try_deserialize().map_err(DaqError::Config)?;
        settings.validate()?;
        Ok(settings)
    }
}

impl Settings {
    fn validate(&self) -> Result<(), DaqError> {
        is_not_empty(&self.log_level).map_err(|e| DaqError::Configuration(e.to_string()))?;
        let valid_log_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_log_levels.contains(&self.log_level.to_lowercase().as_str()) {
            return Err(DaqError::Configuration(format!(
                "Invalid log level: {}",
                self.log_level
            )));
        }

        is_valid_path(&self.storage.default_path)
            .map_err(|e| DaqError::Configuration(e.to_string()))?;
        is_not_empty(&self.storage.default_format)
            .map_err(|e| DaqError::Configuration(e.to_string()))?;

        for (name, instrument) in &self.instruments {
            self.validate_instrument(name, instrument)?;
        }

        Ok(())
    }

    fn validate_instrument(&self, name: &str, instrument: &toml::Value) -> Result<(), DaqError> {
        if let Some(resource_string) = instrument.get("resource_string").and_then(|v| v.as_str()) {
            if resource_string.starts_with("TCPIP") {
                let parts: Vec<&str> = resource_string.split("::").collect();
                if parts.len() >= 2 {
                    let ip_address = parts[1];
                    is_valid_ip(ip_address).map_err(|e| DaqError::Configuration(format!("Invalid IP address for {}: {}", name, e)))?;
                }
            }
        }

        if let Some(sample_rate) = instrument.get("sample_rate_hz").and_then(|v| v.as_float()) {
            is_in_range(sample_rate, 0.1..=1_000_000.0).map_err(|e| DaqError::Configuration(format!("Invalid sample_rate_hz for {}: {}", name, e)))?;
        }

        if let Some(num_samples) = instrument.get("num_samples").and_then(|v| v.as_integer()) {
            is_in_range(num_samples, 1..=1_000_000).map_err(|e| DaqError::Configuration(format!("Invalid num_samples for {}: {}", name, e)))?;
        }

        if let Some(address) = instrument.get("address").and_then(|v| v.as_str()) {
            is_valid_ip(address).map_err(|e| DaqError::Configuration(format!("Invalid IP address for {}: {}", name, e)))?;
        }

        if let Some(port) = instrument.get("port").and_then(|v| v.as_integer()) {
            is_valid_port(port as u16).map_err(|e| DaqError::Configuration(format!("Invalid port for {}: {}", name, e)))?;
        }

        Ok(())
    }
}
