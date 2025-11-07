// DESIGN REFERENCE: Test cases for TimeoutSettings validation
// These tests should be added to src/config.rs when implementing bd-ltd3

#[cfg(test)]
mod timeout_settings_tests {
    use super::*;
    
    // ========================================================================
    // Validation Tests - Each timeout field tested independently
    // ========================================================================
    
    #[test]
    fn test_serial_read_timeout_too_short() {
        let mut settings = TimeoutSettings::default();
        settings.serial_read_timeout_ms = 50; // Below 100ms minimum
        
        let result = settings.validate();
        assert!(result.is_err());
        
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("serial_read_timeout_ms"));
        assert!(err_msg.contains("50ms"));
        assert!(err_msg.contains("100ms - 30000ms"));
    }
    
    #[test]
    fn test_serial_read_timeout_too_long() {
        let mut settings = TimeoutSettings::default();
        settings.serial_read_timeout_ms = 40_000; // Above 30s maximum
        
        let result = settings.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("serial_read_timeout_ms"));
    }
    
    #[test]
    fn test_serial_read_timeout_valid_range() {
        let mut settings = TimeoutSettings::default();
        
        // Test minimum valid value
        settings.serial_read_timeout_ms = 100;
        assert!(settings.validate().is_ok());
        
        // Test maximum valid value
        settings.serial_read_timeout_ms = 30_000;
        assert!(settings.validate().is_ok());
        
        // Test typical value
        settings.serial_read_timeout_ms = 5_000;
        assert!(settings.validate().is_ok());
    }
    
    #[test]
    fn test_scpi_command_timeout_too_short() {
        let mut settings = TimeoutSettings::default();
        settings.scpi_command_timeout_ms = 400; // Below 500ms minimum
        
        let result = settings.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("scpi_command_timeout_ms"));
    }
    
    #[test]
    fn test_scpi_command_timeout_valid_range() {
        let mut settings = TimeoutSettings::default();
        
        // Test minimum
        settings.scpi_command_timeout_ms = 500;
        assert!(settings.validate().is_ok());
        
        // Test maximum
        settings.scpi_command_timeout_ms = 60_000;
        assert!(settings.validate().is_ok());
    }
    
    #[test]
    fn test_network_timeouts_valid_range() {
        let mut settings = TimeoutSettings::default();
        
        // Test minimum valid values
        settings.network_client_timeout_ms = 1_000;
        settings.network_cleanup_timeout_ms = 1_000;
        assert!(settings.validate().is_ok());
        
        // Test maximum valid values
        settings.network_client_timeout_ms = 120_000;
        settings.network_cleanup_timeout_ms = 120_000;
        assert!(settings.validate().is_ok());
    }
    
    #[test]
    fn test_instrument_lifecycle_timeouts_valid_range() {
        let mut settings = TimeoutSettings::default();
        
        // Test all lifecycle timeouts at minimum
        settings.instrument_connect_timeout_ms = 1_000;
        settings.instrument_shutdown_timeout_ms = 1_000;
        settings.instrument_measurement_timeout_ms = 1_000;
        assert!(settings.validate().is_ok());
        
        // Test all at maximum
        settings.instrument_connect_timeout_ms = 60_000;
        settings.instrument_shutdown_timeout_ms = 60_000;
        settings.instrument_measurement_timeout_ms = 60_000;
        assert!(settings.validate().is_ok());
    }
    
    #[test]
    fn test_default_timeouts_are_valid() {
        // Critical: Default values MUST pass validation
        let settings = TimeoutSettings::default();
        assert!(settings.validate().is_ok());
    }
    
    // ========================================================================
    // Backward Compatibility Tests
    // ========================================================================
    
    #[test]
    fn test_missing_timeout_section_uses_defaults() {
        // Config without [application.timeouts] should load successfully
        let toml_content = r#"
            log_level = "info"
            
            [application]
            broadcast_channel_capacity = 1024
            command_channel_capacity = 32
            
            [storage]
            default_path = "./data"
            default_format = "csv"
            
            [instruments]
        "#;
        
        let settings: Settings = toml::from_str(toml_content).unwrap();
        
        // Should use default timeout values
        assert_eq!(settings.application.timeouts.serial_read_timeout_ms, 1000);
        assert_eq!(settings.application.timeouts.scpi_command_timeout_ms, 2000);
        assert_eq!(settings.application.timeouts.network_client_timeout_ms, 5000);
        assert_eq!(settings.application.timeouts.instrument_connect_timeout_ms, 5000);
        
        // Should pass validation
        assert!(settings.validate().is_ok());
    }
    
    #[test]
    fn test_partial_timeout_section() {
        // Config with only some timeout fields specified
        let toml_content = r#"
            log_level = "info"
            
            [application]
            broadcast_channel_capacity = 1024
            command_channel_capacity = 32
            
            [application.timeouts]
            serial_read_timeout_ms = 5000
            # Other fields not specified - should use defaults
            
            [storage]
            default_path = "./data"
            default_format = "csv"
            
            [instruments]
        "#;
        
        let settings: Settings = toml::from_str(toml_content).unwrap();
        
        // Specified field uses custom value
        assert_eq!(settings.application.timeouts.serial_read_timeout_ms, 5000);
        
        // Missing fields use defaults
        assert_eq!(settings.application.timeouts.serial_write_timeout_ms, 1000);
        assert_eq!(settings.application.timeouts.scpi_command_timeout_ms, 2000);
        
        assert!(settings.validate().is_ok());
    }
    
    #[test]
    fn test_empty_timeout_section_uses_defaults() {
        // Config with empty [application.timeouts] section
        let toml_content = r#"
            log_level = "info"
            
            [application]
            broadcast_channel_capacity = 1024
            
            [application.timeouts]
            # Empty section - all fields should use defaults
            
            [storage]
            default_path = "./data"
            default_format = "csv"
            
            [instruments]
        "#;
        
        let settings: Settings = toml::from_str(toml_content).unwrap();
        
        // All fields should use defaults
        let defaults = TimeoutSettings::default();
        assert_eq!(
            settings.application.timeouts.serial_read_timeout_ms,
            defaults.serial_read_timeout_ms
        );
        assert_eq!(
            settings.application.timeouts.scpi_command_timeout_ms,
            defaults.scpi_command_timeout_ms
        );
    }
    
    // ========================================================================
    // Config Loading Tests
    // ========================================================================
    
    #[test]
    fn test_custom_timeouts_load_correctly() {
        let toml_content = r#"
            log_level = "info"
            
            [application]
            broadcast_channel_capacity = 1024
            
            [application.timeouts]
            serial_read_timeout_ms = 3000
            serial_write_timeout_ms = 2500
            scpi_command_timeout_ms = 8000
            network_client_timeout_ms = 15000
            network_cleanup_timeout_ms = 20000
            instrument_connect_timeout_ms = 10000
            instrument_shutdown_timeout_ms = 12000
            instrument_measurement_timeout_ms = 25000
            
            [storage]
            default_path = "./data"
            default_format = "csv"
            
            [instruments]
        "#;
        
        let settings: Settings = toml::from_str(toml_content).unwrap();
        
        // Verify all custom values loaded correctly
        assert_eq!(settings.application.timeouts.serial_read_timeout_ms, 3000);
        assert_eq!(settings.application.timeouts.serial_write_timeout_ms, 2500);
        assert_eq!(settings.application.timeouts.scpi_command_timeout_ms, 8000);
        assert_eq!(settings.application.timeouts.network_client_timeout_ms, 15000);
        assert_eq!(settings.application.timeouts.network_cleanup_timeout_ms, 20000);
        assert_eq!(settings.application.timeouts.instrument_connect_timeout_ms, 10000);
        assert_eq!(settings.application.timeouts.instrument_shutdown_timeout_ms, 12000);
        assert_eq!(settings.application.timeouts.instrument_measurement_timeout_ms, 25000);
        
        assert!(settings.validate().is_ok());
    }
    
    #[test]
    fn test_invalid_timeout_fails_config_load() {
        let toml_content = r#"
            log_level = "info"
            
            [application]
            broadcast_channel_capacity = 1024
            
            [application.timeouts]
            serial_read_timeout_ms = 50  # Too short - below 100ms minimum
            
            [storage]
            default_path = "./data"
            default_format = "csv"
            
            [instruments]
        "#;
        
        let settings: Settings = toml::from_str(toml_content).unwrap();
        
        // Config deserializes successfully, but validation fails
        let result = settings.validate();
        assert!(result.is_err());
        
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("serial_read_timeout_ms"));
        assert!(err_msg.contains("50ms"));
        assert!(err_msg.contains("100ms - 30000ms"));
    }
    
    // ========================================================================
    // Integration Tests - Usage in actual code paths
    // ========================================================================
    
    #[test]
    fn test_timeout_conversion_to_duration() {
        use std::time::Duration;
        
        let settings = TimeoutSettings::default();
        
        // Verify conversion from milliseconds to Duration works correctly
        let serial_timeout = Duration::from_millis(settings.serial_read_timeout_ms);
        assert_eq!(serial_timeout, Duration::from_secs(1));
        
        let scpi_timeout = Duration::from_millis(settings.scpi_command_timeout_ms);
        assert_eq!(scpi_timeout, Duration::from_secs(2));
        
        let connect_timeout = Duration::from_millis(settings.instrument_connect_timeout_ms);
        assert_eq!(connect_timeout, Duration::from_secs(5));
    }
    
    #[test]
    fn test_settings_new_with_valid_config() {
        // This test requires config/default.toml to exist with valid timeouts
        // Should be run as part of integration test suite
        
        // Load from actual config file
        let settings = Settings::new(Some("default"));
        
        // Should load successfully if config/default.toml is valid
        assert!(settings.is_ok());
        
        if let Ok(settings) = settings {
            // Timeouts should be present and valid
            assert!(settings.application.timeouts.serial_read_timeout_ms >= 100);
            assert!(settings.application.timeouts.serial_read_timeout_ms <= 30_000);
        }
    }
    
    // ========================================================================
    // Edge Case Tests
    // ========================================================================
    
    #[test]
    fn test_timeout_at_exact_boundaries() {
        let mut settings = TimeoutSettings::default();
        
        // Test exact minimum boundaries (should pass)
        settings.serial_read_timeout_ms = 100;
        settings.scpi_command_timeout_ms = 500;
        settings.network_client_timeout_ms = 1_000;
        settings.instrument_connect_timeout_ms = 1_000;
        assert!(settings.validate().is_ok());
        
        // Test exact maximum boundaries (should pass)
        settings.serial_read_timeout_ms = 30_000;
        settings.scpi_command_timeout_ms = 60_000;
        settings.network_client_timeout_ms = 120_000;
        settings.instrument_connect_timeout_ms = 60_000;
        assert!(settings.validate().is_ok());
        
        // Test one below minimum (should fail)
        settings.serial_read_timeout_ms = 99;
        assert!(settings.validate().is_err());
        
        // Test one above maximum (should fail)
        settings.serial_read_timeout_ms = 30_001;
        assert!(settings.validate().is_err());
    }
    
    #[test]
    fn test_multiple_invalid_timeouts() {
        let mut settings = TimeoutSettings::default();
        
        // Set multiple timeouts to invalid values
        settings.serial_read_timeout_ms = 50;    // Too short
        settings.scpi_command_timeout_ms = 400;  // Too short
        settings.network_client_timeout_ms = 500; // Too short
        
        // Should fail on first invalid timeout encountered
        let result = settings.validate();
        assert!(result.is_err());
        
        // Error message should mention at least one invalid timeout
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("serial_read_timeout_ms") ||
            err_msg.contains("scpi_command_timeout_ms") ||
            err_msg.contains("network_client_timeout_ms")
        );
    }
    
    #[test]
    fn test_zero_timeout_fails_validation() {
        let mut settings = TimeoutSettings::default();
        
        // Zero timeout should fail (u64 prevents negative, but zero is invalid)
        settings.serial_read_timeout_ms = 0;
        assert!(settings.validate().is_err());
    }
    
    // ========================================================================
    // Realistic Use Case Tests
    // ========================================================================
    
    #[test]
    fn test_slow_spectrometer_config() {
        // Realistic scenario: Spectrometer with 30s integration time
        let toml_content = r#"
            log_level = "info"
            
            [application]
            broadcast_channel_capacity = 1024
            
            [application.timeouts]
            instrument_measurement_timeout_ms = 35000  # 35s for long integrations
            scpi_command_timeout_ms = 10000             # 10s for complex commands
            
            [storage]
            default_path = "./data"
            default_format = "csv"
            
            [instruments]
        "#;
        
        let settings: Settings = toml::from_str(toml_content).unwrap();
        
        assert_eq!(settings.application.timeouts.instrument_measurement_timeout_ms, 35_000);
        assert_eq!(settings.application.timeouts.scpi_command_timeout_ms, 10_000);
        
        // Other timeouts should use defaults
        assert_eq!(settings.application.timeouts.serial_read_timeout_ms, 1000);
        
        assert!(settings.validate().is_ok());
    }
    
    #[test]
    fn test_debug_mode_long_timeouts() {
        // Realistic scenario: Debug session with breakpoints
        let toml_content = r#"
            log_level = "debug"
            
            [application]
            broadcast_channel_capacity = 1024
            
            [application.timeouts]
            # All timeouts increased to 60s to prevent timeout during debugging
            serial_read_timeout_ms = 60000
            serial_write_timeout_ms = 60000
            scpi_command_timeout_ms = 60000
            network_client_timeout_ms = 60000
            instrument_connect_timeout_ms = 60000
            instrument_shutdown_timeout_ms = 60000
            instrument_measurement_timeout_ms = 60000
            
            [storage]
            default_path = "./data"
            default_format = "csv"
            
            [instruments]
        "#;
        
        let settings: Settings = toml::from_str(toml_content).unwrap();
        
        // All timeouts set to maximum for debug mode
        assert_eq!(settings.application.timeouts.serial_read_timeout_ms, 60_000);
        assert_eq!(settings.application.timeouts.instrument_connect_timeout_ms, 60_000);
        
        assert!(settings.validate().is_ok());
    }
    
    #[test]
    fn test_fast_mock_instruments_config() {
        // Realistic scenario: Mock instruments for testing with short timeouts
        let toml_content = r#"
            log_level = "info"
            
            [application]
            broadcast_channel_capacity = 1024
            
            [application.timeouts]
            # Short timeouts for fast mock instruments
            serial_read_timeout_ms = 200
            instrument_connect_timeout_ms = 1000
            instrument_measurement_timeout_ms = 1000
            
            [storage]
            default_path = "./data"
            default_format = "csv"
            
            [instruments]
        "#;
        
        let settings: Settings = toml::from_str(toml_content).unwrap();
        
        assert_eq!(settings.application.timeouts.serial_read_timeout_ms, 200);
        assert_eq!(settings.application.timeouts.instrument_connect_timeout_ms, 1000);
        
        assert!(settings.validate().is_ok());
    }
}
