// DESIGN REFERENCE: TimeoutSettings struct for bd-ltd3 implementation
// This file demonstrates the struct definition and validation logic.
// Copy to src/config.rs when implementing bd-ltd3.

use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Timeout configuration for all system operations
///
/// All timeouts are in milliseconds for consistency with existing config patterns
/// (similar to metrics_window_secs in DataDistributorSettings).
///
/// # Validation Ranges
///
/// - Serial I/O: 100ms - 30s (prevent too-short hangs, too-long freezes)
/// - Protocol: 500ms - 60s (commands need reasonable time)
/// - Network: 1s - 120s (network operations can be slow)
/// - Instrument lifecycle: 1s - 60s (hardware init can take time)
///
/// # Usage
///
/// ```rust
/// use std::time::Duration;
///
/// let settings = Settings::new(Some("default"))?;
/// let timeout = Duration::from_millis(
///     settings.application.timeouts.serial_read_timeout_ms
/// );
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct TimeoutSettings {
    // ========================================================================
    // Serial I/O Timeouts
    // ========================================================================
    
    /// Timeout for serial port read operations (milliseconds)
    ///
    /// Applied to all serial instrument reads via SerialAdapter.
    /// Default: 1000ms (1 second)
    /// Valid range: 100-30000ms
    ///
    /// Increase for:
    /// - Slow instruments that take >1s to respond
    /// - High-latency serial-over-network configurations
    /// - Debug sessions where breakpoints may delay responses
    pub serial_read_timeout_ms: u64,
    
    /// Timeout for serial port write operations (milliseconds)
    ///
    /// Applied to all serial instrument writes via SerialAdapter.
    /// Default: 1000ms (1 second)
    /// Valid range: 100-30000ms
    pub serial_write_timeout_ms: u64,
    
    // ========================================================================
    // Protocol Timeouts
    // ========================================================================
    
    /// Timeout for SCPI command/response cycles (milliseconds)
    ///
    /// Applied to SCPI instruments (Keithley, Newport, etc.).
    /// Covers full command execution including instrument processing time.
    /// Default: 2000ms (2 seconds)
    /// Valid range: 500-60000ms
    ///
    /// Increase for:
    /// - Complex SCPI commands with long processing times
    /// - Instruments performing internal calculations or calibrations
    /// - Commands that trigger mechanical movements
    pub scpi_command_timeout_ms: u64,
    
    // ========================================================================
    // Network Timeouts
    // ========================================================================
    
    /// Timeout for network client request handling (milliseconds)
    ///
    /// Applied to individual client requests in network server.
    /// Default: 5000ms (5 seconds)
    /// Valid range: 1000-120000ms
    ///
    /// Increase for:
    /// - High-latency network deployments
    /// - VPN or WAN connections
    /// - Large data transfers over network
    pub network_client_timeout_ms: u64,
    
    /// Timeout for network cleanup operations (milliseconds)
    ///
    /// Applied to connection cleanup and resource deallocation.
    /// Default: 10000ms (10 seconds)
    /// Valid range: 1000-120000ms
    pub network_cleanup_timeout_ms: u64,
    
    // ========================================================================
    // Instrument Lifecycle Timeouts
    // ========================================================================
    
    /// Timeout for instrument connection/initialization (milliseconds)
    ///
    /// Applied to connect() calls in InstrumentManagerV3.
    /// Covers hardware initialization, self-test, and calibration.
    /// Default: 5000ms (5 seconds)
    /// Valid range: 1000-60000ms
    ///
    /// Increase for:
    /// - Instruments with slow initialization (cameras, spectrometers)
    /// - Devices performing self-test on startup
    /// - Hardware requiring warmup time
    pub instrument_connect_timeout_ms: u64,
    
    /// Timeout for graceful instrument shutdown (milliseconds)
    ///
    /// Applied to disconnect() calls and shutdown sequences.
    /// Allows instruments to finish current operations and clean up.
    /// Default: 6000ms (6 seconds)
    /// Valid range: 1000-60000ms
    ///
    /// Increase for:
    /// - Instruments with complex shutdown procedures
    /// - Devices that need to complete in-progress operations
    /// - Hardware requiring cooldown time
    pub instrument_shutdown_timeout_ms: u64,
    
    /// Timeout for waiting for measurement data (milliseconds)
    ///
    /// Applied to measurement channel receive operations.
    /// Default: 5000ms (5 seconds)
    /// Valid range: 1000-60000ms
    ///
    /// Increase for:
    /// - Long integration times (spectrometers, CCDs)
    /// - Slow data acquisition cycles
    /// - Instruments with variable measurement times
    pub instrument_measurement_timeout_ms: u64,
}

impl Default for TimeoutSettings {
    fn default() -> Self {
        Self {
            // Serial I/O (1s default - matches current hardcoded values)
            serial_read_timeout_ms: 1000,
            serial_write_timeout_ms: 1000,
            
            // Protocol (2s default - matches current hardcoded values)
            scpi_command_timeout_ms: 2000,
            
            // Network (5-10s defaults - matches current hardcoded values)
            network_client_timeout_ms: 5000,
            network_cleanup_timeout_ms: 10000,
            
            // Instrument lifecycle (5-6s defaults - matches current hardcoded values)
            instrument_connect_timeout_ms: 5000,
            instrument_shutdown_timeout_ms: 6000,
            instrument_measurement_timeout_ms: 5000,
        }
    }
}

impl TimeoutSettings {
    /// Validate all timeout values are within acceptable ranges
    ///
    /// This prevents common configuration errors:
    /// - Too-short timeouts that cause spurious failures
    /// - Absurdly long timeouts that hang the application
    /// - Zero or negative timeouts (u64 prevents negative, but catches zero)
    ///
    /// # Errors
    ///
    /// Returns `Err` if any timeout is outside its valid range.
    /// Error messages include the field name, actual value, and valid range.
    pub fn validate(&self) -> Result<()> {
        // Serial I/O: 100ms - 30s
        // Reasoning: Serial reads need >100ms for slow devices, <30s to prevent UI hangs
        validate_timeout_range(
            self.serial_read_timeout_ms,
            100,
            30_000,
            "serial_read_timeout_ms"
        )?;
        validate_timeout_range(
            self.serial_write_timeout_ms,
            100,
            30_000,
            "serial_write_timeout_ms"
        )?;
        
        // Protocol: 500ms - 60s
        // Reasoning: SCPI commands need >500ms for processing, <60s to prevent deadlocks
        validate_timeout_range(
            self.scpi_command_timeout_ms,
            500,
            60_000,
            "scpi_command_timeout_ms"
        )?;
        
        // Network: 1s - 120s
        // Reasoning: Network ops need >1s for latency, <120s to prevent client hangs
        validate_timeout_range(
            self.network_client_timeout_ms,
            1_000,
            120_000,
            "network_client_timeout_ms"
        )?;
        validate_timeout_range(
            self.network_cleanup_timeout_ms,
            1_000,
            120_000,
            "network_cleanup_timeout_ms"
        )?;
        
        // Instrument lifecycle: 1s - 60s
        // Reasoning: Hardware init needs >1s for slow devices, <60s to prevent startup hangs
        validate_timeout_range(
            self.instrument_connect_timeout_ms,
            1_000,
            60_000,
            "instrument_connect_timeout_ms"
        )?;
        validate_timeout_range(
            self.instrument_shutdown_timeout_ms,
            1_000,
            60_000,
            "instrument_shutdown_timeout_ms"
        )?;
        validate_timeout_range(
            self.instrument_measurement_timeout_ms,
            1_000,
            60_000,
            "instrument_measurement_timeout_ms"
        )?;
        
        Ok(())
    }
}

/// Helper function to validate a timeout is within a valid range
///
/// # Arguments
///
/// * `value` - The timeout value to validate (milliseconds)
/// * `min` - Minimum allowed value (inclusive)
/// * `max` - Maximum allowed value (inclusive)
/// * `name` - Field name for error messages
///
/// # Errors
///
/// Returns `Err` with descriptive message if value is outside [min, max] range.
fn validate_timeout_range(value: u64, min: u64, max: u64, name: &str) -> Result<()> {
    if value < min || value > max {
        anyhow::bail!(
            "Timeout '{}' = {}ms is out of valid range ({}ms - {}ms). \
             Check [application.timeouts] in config.toml.",
            name,
            value,
            min,
            max
        );
    }
    Ok(())
}

// ============================================================================
// Integration with ApplicationSettings
// ============================================================================

/// Example of how TimeoutSettings integrates into ApplicationSettings
///
/// This should be added to the existing ApplicationSettings struct in src/config.rs:
///
/// ```rust
/// #[derive(Debug, Serialize, Deserialize, Clone)]
/// pub struct ApplicationSettings {
///     #[serde(default = "default_broadcast_capacity")]
///     pub broadcast_channel_capacity: usize,
///     
///     #[serde(default = "default_command_capacity")]
///     pub command_channel_capacity: usize,
///     
///     #[serde(default)]
///     pub data_distributor: DataDistributorSettings,
///     
///     // NEW: Add this field
///     #[serde(default)]
///     pub timeouts: TimeoutSettings,
/// }
/// ```
///
/// The `#[serde(default)]` attribute ensures backward compatibility:
/// - If [application.timeouts] section is missing, uses TimeoutSettings::default()
/// - If only some fields are specified, missing fields use defaults
/// - Existing config files without timeout section continue to work

// ============================================================================
// Usage Examples
// ============================================================================

#[cfg(test)]
mod examples {
    use super::*;
    use std::time::Duration;
    
    /// Example: Using timeout settings in SerialAdapter
    #[test]
    fn example_serial_adapter_usage() {
        let settings = TimeoutSettings::default();
        
        // Convert milliseconds to Duration
        let read_timeout = Duration::from_millis(settings.serial_read_timeout_ms);
        let write_timeout = Duration::from_millis(settings.serial_write_timeout_ms);
        
        // Use in SerialAdapter constructor
        // let adapter = SerialAdapter::new(port, read_timeout, write_timeout);
        
        assert_eq!(read_timeout, Duration::from_secs(1));
        assert_eq!(write_timeout, Duration::from_secs(1));
    }
    
    /// Example: Using timeout settings in InstrumentManager
    #[test]
    fn example_instrument_manager_usage() {
        let settings = TimeoutSettings::default();
        
        let connect_timeout = Duration::from_millis(settings.instrument_connect_timeout_ms);
        let shutdown_timeout = Duration::from_millis(settings.instrument_shutdown_timeout_ms);
        
        // Use in timeout() calls
        // tokio::time::timeout(connect_timeout, instrument.connect()).await?;
        // tokio::time::timeout(shutdown_timeout, instrument.shutdown()).await?;
        
        assert_eq!(connect_timeout, Duration::from_secs(5));
        assert_eq!(shutdown_timeout, Duration::from_secs(6));
    }
    
    /// Example: Validation catches invalid values
    #[test]
    fn example_validation_catches_errors() {
        let mut settings = TimeoutSettings::default();
        
        // Too short - should fail
        settings.serial_read_timeout_ms = 50;
        assert!(settings.validate().is_err());
        
        // Too long - should fail
        settings.serial_read_timeout_ms = 100_000;
        assert!(settings.validate().is_err());
        
        // Just right - should pass
        settings.serial_read_timeout_ms = 1000;
        assert!(settings.validate().is_ok());
    }
    
    /// Example: Partial config with defaults
    #[test]
    fn example_partial_config() {
        // This TOML would only specify one timeout:
        // [application.timeouts]
        // serial_read_timeout_ms = 5000
        //
        // Serde will use default() for missing fields
        
        let toml_content = r#"
            serial_read_timeout_ms = 5000
        "#;
        
        let mut settings: TimeoutSettings = toml::from_str(toml_content).unwrap();
        
        // Specified field uses custom value
        assert_eq!(settings.serial_read_timeout_ms, 5000);
        
        // Missing fields use defaults
        assert_eq!(settings.serial_write_timeout_ms, 1000);
        assert_eq!(settings.scpi_command_timeout_ms, 2000);
        
        // But this would fail validation (5000ms is within valid range)
        assert!(settings.validate().is_ok());
        
        // Now make it fail validation
        settings.serial_read_timeout_ms = 50;
        assert!(settings.validate().is_err());
    }
}

// ============================================================================
// Migration Checklist for bd-ltd3 Implementation
// ============================================================================

/// IMPLEMENTATION CHECKLIST:
///
/// 1. [ ] Copy TimeoutSettings struct to src/config.rs
/// 2. [ ] Add `pub timeouts: TimeoutSettings` field to ApplicationSettings
/// 3. [ ] Add `#[serde(default)]` attribute to timeouts field
/// 4. [ ] Call `settings.application.timeouts.validate()` in Settings::validate()
/// 5. [ ] Update config/default.toml with [application.timeouts] section (DONE)
/// 6. [ ] Replace hardcoded timeouts in:
///    - [ ] src/adapters/serial_adapter.rs (2 instances)
///    - [ ] src/instrument/scpi_common.rs (3 instances)
///    - [ ] src/network/server_actor.rs (10 instances)
///    - [ ] src/instrument_manager_v3.rs (4 instances)
///    - [ ] src/experiment/primitives.rs (2 instances if applicable)
/// 7. [ ] Add unit tests to src/config.rs (validation, defaults, partial config)
/// 8. [ ] Add integration tests (timeout behavior, backward compatibility)
/// 9. [ ] Update CLAUDE.md with timeout configuration section
/// 10. [ ] Run `cargo test` to verify all tests pass
/// 11. [ ] Run `cargo clippy` to verify no new warnings
/// 12. [ ] Test with existing config.toml (should use defaults)
/// 13. [ ] Test with custom timeouts (verify values applied)
/// 14. [ ] Test with invalid timeouts (verify validation errors)
///
/// Estimated time: 2-3 hours
