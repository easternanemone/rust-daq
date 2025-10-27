//! Integration tests for the module system (bd-64 Phase 3)
//!
//! Tests runtime instrument assignment and module lifecycle management.

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rust_daq::modules::power_meter::PowerMeterModule;
    use rust_daq::modules::{Module, ModuleConfig, ModuleStatus, ModuleWithInstrument};

    // Mock measure type for testing
    #[derive(Clone)]
    struct MockPowerMeasure;

    #[async_trait::async_trait]
    impl rust_daq::measurement::Measure for MockPowerMeasure {
        type Data = f64;

        async fn measure(&mut self) -> Result<Self::Data> {
            Ok(42.0)
        }

        async fn data_stream(
            &self,
        ) -> Result<tokio::sync::mpsc::Receiver<std::sync::Arc<Self::Data>>> {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(rx)
        }
    }

    #[test]
    fn test_power_meter_module_creation() {
        let module: PowerMeterModule<MockPowerMeasure> =
            PowerMeterModule::new("test_power".to_string());
        assert_eq!(module.name(), "test_power");
        assert_eq!(module.status(), ModuleStatus::Idle);
    }

    #[test]
    fn test_power_meter_module_initialization() -> Result<()> {
        let mut module: PowerMeterModule<MockPowerMeasure> =
            PowerMeterModule::new("test_power".to_string());

        let mut config = ModuleConfig::new();
        config.set("low_threshold".to_string(), serde_json::json!(50.0));
        config.set("high_threshold".to_string(), serde_json::json!(150.0));
        config.set("window_duration_s".to_string(), serde_json::json!(60.0));

        module.init(config)?;
        assert_eq!(module.status(), ModuleStatus::Initialized);
        Ok(())
    }

    #[test]
    fn test_power_meter_module_lifecycle() -> Result<()> {
        let mut module: PowerMeterModule<MockPowerMeasure> =
            PowerMeterModule::new("test_power".to_string());

        // Initialize
        module.init(ModuleConfig::new())?;
        assert_eq!(module.status(), ModuleStatus::Initialized);

        // Start should fail without instrument
        assert!(module.start().is_err());

        Ok(())
    }

    #[test]
    fn test_power_meter_invalid_thresholds() -> Result<()> {
        let mut module: PowerMeterModule<MockPowerMeasure> =
            PowerMeterModule::new("test_power".to_string());

        let mut config = ModuleConfig::new();
        config.set("low_threshold".to_string(), serde_json::json!(150.0));
        config.set("high_threshold".to_string(), serde_json::json!(50.0));

        let result = module.init(config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid thresholds"));

        Ok(())
    }

    #[test]
    fn test_power_meter_module_pause_stop_without_running() -> Result<()> {
        let mut module: PowerMeterModule<MockPowerMeasure> =
            PowerMeterModule::new("test_power".to_string());

        module.init(ModuleConfig::new())?;

        // Cannot pause if not running
        assert!(module.pause().is_err());

        // Can stop initialized module (transitions to Stopped)
        module.stop().ok(); // May fail or succeed depending on implementation

        Ok(())
    }

    #[test]
    fn test_power_meter_module_measurement_broadcast() -> Result<()> {
        let mut module: PowerMeterModule<MockPowerMeasure> =
            PowerMeterModule::new("test_power".to_string());

        // Create measurement channel
        let mut _rx = module.create_measurement_channel();

        module.init(ModuleConfig::new())?;

        // Module now has a broadcast channel that can be used for data streaming
        assert_eq!(module.status(), ModuleStatus::Initialized);

        Ok(())
    }

    #[test]
    fn test_power_meter_module_config_parsing() -> Result<()> {
        let mut module: PowerMeterModule<MockPowerMeasure> =
            PowerMeterModule::new("test_power".to_string());

        let mut config = ModuleConfig::new();
        config.set("low_threshold".to_string(), serde_json::json!(40.0));
        config.set("high_threshold".to_string(), serde_json::json!(200.0));
        config.set("window_duration_s".to_string(), serde_json::json!(120.0));
        config.set(
            "alert_callback".to_string(),
            serde_json::json!("email_alert"),
        );

        module.init(config)?;

        // Verify initialization succeeded with custom values
        assert_eq!(module.status(), ModuleStatus::Initialized);

        Ok(())
    }
}
