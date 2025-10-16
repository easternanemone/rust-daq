//! Comprehensive tests for type-safe configuration validation.

use rust_daq::instrument::config::MockInstrumentConfig;

#[test]
fn test_negative_sample_rate_rejected() {
    let config = MockInstrumentConfig {
        sample_rate_hz: -100.0,
        num_samples: 1000,
    };
    let result = config.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("sample_rate_hz must be positive and finite"));
}

#[test]
fn test_nan_sample_rate_rejected() {
    let config = MockInstrumentConfig {
        sample_rate_hz: f64::NAN,
        num_samples: 1000,
    };
    let result = config.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("sample_rate_hz must be positive and finite"));
}

#[test]
fn test_positive_infinity_sample_rate_rejected() {
    let config = MockInstrumentConfig {
        sample_rate_hz: f64::INFINITY,
        num_samples: 1000,
    };
    let result = config.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("sample_rate_hz must be positive and finite"));
}

#[test]
fn test_negative_infinity_sample_rate_rejected() {
    let config = MockInstrumentConfig {
        sample_rate_hz: f64::NEG_INFINITY,
        num_samples: 1000,
    };
    let result = config.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("sample_rate_hz must be positive and finite"));
}

#[test]
fn test_very_large_num_samples_accepted() {
    // usize::MAX should be accepted (though impractical)
    let config = MockInstrumentConfig {
        sample_rate_hz: 1000.0,
        num_samples: usize::MAX,
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_boundary_sample_rate_accepted() {
    // Very small positive value should be accepted
    let config = MockInstrumentConfig {
        sample_rate_hz: f64::MIN_POSITIVE,
        num_samples: 1,
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_minimum_num_samples_accepted() {
    let config = MockInstrumentConfig {
        sample_rate_hz: 1000.0,
        num_samples: 1,
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_from_toml_with_wrong_type_for_sample_rate() {
    let toml_str = r#"
        sample_rate_hz = "not_a_number"
        num_samples = 1000
    "#;
    let value: toml::Value = toml::from_str(toml_str).unwrap();
    let result = MockInstrumentConfig::from_toml(&value);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to parse mock instrument configuration"));
}

#[test]
fn test_from_toml_with_wrong_type_for_num_samples() {
    let toml_str = r#"
        sample_rate_hz = 1000.0
        num_samples = "not_a_number"
    "#;
    let value: toml::Value = toml::from_str(toml_str).unwrap();
    let result = MockInstrumentConfig::from_toml(&value);
    assert!(result.is_err());
}

#[test]
fn test_from_toml_with_missing_sample_rate() {
    let toml_str = r#"
        num_samples = 1000
    "#;
    let value: toml::Value = toml::from_str(toml_str).unwrap();
    let result = MockInstrumentConfig::from_toml(&value);
    assert!(result.is_err());
}

#[test]
fn test_from_toml_with_missing_num_samples() {
    let toml_str = r#"
        sample_rate_hz = 1000.0
    "#;
    let value: toml::Value = toml::from_str(toml_str).unwrap();
    let result = MockInstrumentConfig::from_toml(&value);
    assert!(result.is_err());
}

#[test]
fn test_from_toml_with_extra_fields() {
    // Extra fields should be ignored by serde (default behavior)
    let toml_str = r#"
        sample_rate_hz = 1000.0
        num_samples = 1000
        extra_field = "ignored"
        another_extra = 42
    "#;
    let value: toml::Value = toml::from_str(toml_str).unwrap();
    let result = MockInstrumentConfig::from_toml(&value);
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.sample_rate_hz, 1000.0);
    assert_eq!(config.num_samples, 1000);
}

#[test]
fn test_serialization_round_trip() {
    let original = MockInstrumentConfig {
        sample_rate_hz: 1234.5,
        num_samples: 9876,
    };

    // Serialize to TOML string
    let toml_str = toml::to_string(&original).unwrap();

    // Deserialize back
    let toml_value: toml::Value = toml::from_str(&toml_str).unwrap();
    let restored = MockInstrumentConfig::from_toml(&toml_value).unwrap();

    // Should match original
    assert_eq!(original.sample_rate_hz, restored.sample_rate_hz);
    assert_eq!(original.num_samples, restored.num_samples);
}

#[test]
fn test_from_toml_validated_rejects_invalid_after_parse() {
    let toml_str = r#"
        sample_rate_hz = -500.0
        num_samples = 1000
    "#;
    let value: toml::Value = toml::from_str(toml_str).unwrap();
    let result = MockInstrumentConfig::from_toml_validated(&value);

    // Should fail at validation step, not parsing
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("sample_rate_hz must be positive"));
}

#[test]
fn test_default_config_passes_validation() {
    let config = MockInstrumentConfig::default();
    assert!(config.validate().is_ok());
    assert_eq!(config.sample_rate_hz, 1000.0);
    assert_eq!(config.num_samples, 10000);
}

#[test]
fn test_clone_preserves_values() {
    let original = MockInstrumentConfig {
        sample_rate_hz: 2000.0,
        num_samples: 5000,
    };
    let cloned = original.clone();
    assert_eq!(original.sample_rate_hz, cloned.sample_rate_hz);
    assert_eq!(original.num_samples, cloned.num_samples);
}
