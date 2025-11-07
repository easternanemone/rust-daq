//! Regression Tests for bd-76: RollbackToVersion Error Handling
//!
//! This test suite validates that the RollbackToVersion command properly
//! propagates errors from VersionManager through the actor's command handling.
//! The error handling logic (src/app_actor.rs:413-425) is already correct,
//! but these tests prevent regression.
//!
//! ## Test Coverage
//!
//! 1. **Invalid Version ID**: Non-existent version file
//! 2. **Missing Snapshot File**: Deleted or corrupted snapshot directory
//! 3. **Corrupted TOML**: Invalid TOML syntax in snapshot
//! 4. **Successful Rollback**: Valid snapshot restores settings correctly
//!
//! ## Testing Strategy
//!
//! - Uses DaqApp's actor interface (command_tx/oneshot response)
//! - Standard #[test] (not #[tokio::test]) since DaqApp creates its own runtime
//! - Uses `.daq/config_versions` directory (same as production)
//! - Creates/deletes test snapshot files
//! - Verifies error messages contain useful context

use rust_daq::{
    app::DaqApp,
    config::{versioning::VersionId, Settings},
    data::registry::ProcessorRegistry,
    instrument::{InstrumentRegistry, InstrumentRegistryV2},
    log_capture::LogBuffer,
    measurement::InstrumentMeasurement,
    messages::DaqCommand,
    modules::ModuleRegistry,
};
use std::fs;
use std::sync::Arc;

/// Helper to create a test DaqApp with custom settings
fn create_test_app_with_settings(settings: Settings) -> DaqApp<InstrumentMeasurement> {
    let settings = Arc::new(settings);
    let instrument_registry = Arc::new(InstrumentRegistry::<InstrumentMeasurement>::new());
    let instrument_registry_v2 = Arc::new(InstrumentRegistryV2::new());
    let processor_registry = Arc::new(ProcessorRegistry::new());
    let module_registry = Arc::new(ModuleRegistry::<InstrumentMeasurement>::new());
    let log_buffer = LogBuffer::new();

    DaqApp::new_with_v2(
        settings.clone(),
        instrument_registry,
        instrument_registry_v2,
        processor_registry,
        module_registry,
        log_buffer,
    )
    .expect("Failed to create app")
}

/// Helper to ensure config versions directory exists
fn ensure_config_dir() {
    fs::create_dir_all(".daq/config_versions").expect("Failed to create config directory");
}

/// Helper to clean up test snapshot files
fn cleanup_test_snapshot(version_id: &VersionId) {
    let path = format!(".daq/config_versions/{}", version_id.0);
    let _ = fs::remove_file(path); // Ignore errors if file doesn't exist
}

#[test]
fn test_rollback_invalid_version_id() {
    //! Test bd-76: Verify that rollback with non-existent version ID returns error
    //!
    //! This test validates that when a version ID doesn't correspond to any
    //! snapshot file, the RollbackToVersion command returns a meaningful error
    //! instead of panicking or returning success.

    ensure_config_dir();

    let settings = Settings::new(None).expect("Failed to create settings");
    let app = create_test_app_with_settings(settings);

    // Attempt to rollback to non-existent version
    let invalid_version = VersionId("nonexistent_version_12345.toml".to_string());
    let (cmd, rx) = DaqCommand::rollback_to_version(invalid_version.clone());

    // Send command via blocking API (DaqApp creates its own runtime)
    app.command_tx
        .blocking_send(cmd)
        .expect("Command should be queued");

    // Receive response
    let result = rx.blocking_recv().expect("Should receive response");

    // Verify error is returned
    assert!(
        result.is_err(),
        "Rollback with invalid version ID should return error"
    );

    let error = result.unwrap_err();
    let error_msg = format!("{}", error);

    // Verify error message indicates file not found
    assert!(
        error_msg.contains("No such file") || error_msg.contains("not found"),
        "Error message should indicate file not found: {}",
        error_msg
    );

    app.shutdown().expect("Shutdown should succeed");
}

#[test]
fn test_rollback_missing_snapshot_file() {
    //! Test bd-76: Verify that rollback with deleted snapshot file returns error
    //!
    //! This test creates a valid version ID but ensures the snapshot file doesn't exist,
    //! simulating filesystem corruption or manual deletion.

    ensure_config_dir();

    let settings = Settings::new(None).expect("Failed to create settings");
    let app = create_test_app_with_settings(settings);

    // Create a valid-looking version ID but ensure no corresponding file exists
    let version_id = VersionId("valid_looking_version_67890.toml".to_string());
    cleanup_test_snapshot(&version_id); // Ensure it doesn't exist

    let snapshot_path = format!(".daq/config_versions/{}", version_id.0);
    assert!(
        !std::path::Path::new(&snapshot_path).exists(),
        "Snapshot file should not exist for this test"
    );

    // Attempt to rollback to version with missing file
    let (cmd, rx) = DaqCommand::rollback_to_version(version_id);

    app.command_tx
        .blocking_send(cmd)
        .expect("Command should be queued");

    let result = rx.blocking_recv().expect("Should receive response");

    // Verify error is returned
    assert!(
        result.is_err(),
        "Rollback with missing snapshot file should return error"
    );

    let error = result.unwrap_err();
    let error_msg = format!("{}", error);

    // Verify error message mentions file not found or I/O error
    assert!(
        error_msg.contains("No such file") || error_msg.contains("not found"),
        "Error message should indicate file not found: {}",
        error_msg
    );

    app.shutdown().expect("Shutdown should succeed");
}

#[test]
fn test_rollback_corrupted_toml() {
    //! Test bd-76: Verify that rollback with corrupted TOML returns error
    //!
    //! This test creates a snapshot file with invalid TOML syntax and verifies
    //! that the deserialization error is propagated correctly.

    ensure_config_dir();

    // Create a snapshot file with corrupted TOML
    let version_id = VersionId("corrupted_version_test.toml".to_string());
    let snapshot_path = format!(".daq/config_versions/{}", version_id.0);

    // Write invalid TOML (missing closing bracket, invalid syntax)
    let corrupted_toml = r#"
        [application]
        log_level = "info"
        broadcast_channel_capacity = [ # INVALID: Unclosed array
        command_channel_capacity = "not_a_number" # INVALID: Should be integer

        [storage
        default_format = "csv"
        # MISSING CLOSING BRACKET
    "#;

    fs::write(&snapshot_path, corrupted_toml).expect("Failed to write corrupted snapshot");

    let settings = Settings::new(None).expect("Failed to create settings");
    let app = create_test_app_with_settings(settings);

    // Attempt to rollback to corrupted snapshot
    let (cmd, rx) = DaqCommand::rollback_to_version(version_id.clone());

    app.command_tx
        .blocking_send(cmd)
        .expect("Command should be queued");

    let result = rx.blocking_recv().expect("Should receive response");

    // Verify error is returned
    assert!(
        result.is_err(),
        "Rollback with corrupted TOML should return error"
    );

    let error = result.unwrap_err();
    let error_msg = format!("{}", error);

    // Verify error message mentions parsing or deserialization
    assert!(
        error_msg.contains("TOML")
            || error_msg.contains("invalid")
            || error_msg.contains("expected")
            || error_msg.contains("missing"),
        "Error message should indicate TOML parsing failure: {}",
        error_msg
    );

    // Cleanup
    cleanup_test_snapshot(&version_id);

    app.shutdown().expect("Shutdown should succeed");
}

#[test]
fn test_rollback_successful() {
    //! Test bd-76: Verify that rollback with valid snapshot succeeds
    //!
    //! This test creates a valid snapshot file and verifies that the rollback
    //! operation succeeds and returns Ok(()).

    ensure_config_dir();

    // Create a valid snapshot file with proper TOML
    let version_id = VersionId("valid_version_test.toml".to_string());
    let snapshot_path = format!(".daq/config_versions/{}", version_id.0);

    // Write valid TOML representing a minimal Settings structure
    let valid_toml = r#"
log_level = "info"

[application]
broadcast_channel_capacity = 1024
command_channel_capacity = 32

[application.data_distributor]
subscriber_capacity = 512
warn_drop_rate_percent = 5.0
error_saturation_percent = 90.0
metrics_window_secs = 10

[storage]
default_path = "./data_test"
default_format = "csv"

[instruments]
# Empty instruments table for test
"#;

    fs::write(&snapshot_path, valid_toml).expect("Failed to write valid snapshot");

    let settings = Settings::new(None).expect("Failed to create settings");
    let app = create_test_app_with_settings(settings);

    // Attempt to rollback to valid snapshot
    let (cmd, rx) = DaqCommand::rollback_to_version(version_id.clone());

    app.command_tx
        .blocking_send(cmd)
        .expect("Command should be queued");

    let result = rx.blocking_recv().expect("Should receive response");

    // Verify success
    assert!(
        result.is_ok(),
        "Rollback with valid snapshot should succeed: {:?}",
        result
    );

    // Cleanup
    cleanup_test_snapshot(&version_id);

    app.shutdown().expect("Shutdown should succeed");
}

#[test]
fn test_rollback_multiple_errors_sequential() {
    //! Test bd-76: Verify that multiple rollback errors are handled independently
    //!
    //! This test sends multiple rollback commands with different invalid version IDs
    //! and verifies each returns an error with appropriate context.

    ensure_config_dir();

    let settings = Settings::new(None).expect("Failed to create settings");
    let app = create_test_app_with_settings(settings);

    // Test multiple invalid version IDs
    let test_cases = vec![
        "invalid_version_1.toml",
        "invalid_version_2.toml",
        "invalid_version_3.toml",
    ];

    for version_str in test_cases {
        let version_id = VersionId(version_str.to_string());
        let (cmd, rx) = DaqCommand::rollback_to_version(version_id.clone());

        app.command_tx
            .blocking_send(cmd)
            .expect("Command should be queued");

        let result = rx.blocking_recv().expect("Should receive response");

        assert!(
            result.is_err(),
            "Rollback with version '{}' should return error",
            version_str
        );

        // Each error should be returned properly
        let error = result.unwrap_err();
        let error_msg = format!("{}", error);

        // Verify error is a meaningful I/O error
        assert!(
            error_msg.contains("No such file") || error_msg.contains("not found"),
            "Error for version '{}' should indicate file not found: {}",
            version_str,
            error_msg
        );
    }

    app.shutdown().expect("Shutdown should succeed");
}

#[test]
fn test_error_propagation_preserves_type() {
    //! Test bd-76: Verify that error type is preserved through actor layers
    //!
    //! This test validates that the error handling in app_actor.rs:413-425
    //! properly preserves error type from VersionManager all the way to
    //! the caller (anyhow::Error).

    ensure_config_dir();

    let settings = Settings::new(None).expect("Failed to create settings");
    let app = create_test_app_with_settings(settings);

    let version_id = VersionId("error_type_test_version.toml".to_string());
    let (cmd, rx) = DaqCommand::rollback_to_version(version_id.clone());

    app.command_tx
        .blocking_send(cmd)
        .expect("Command should be queued");

    let result = rx.blocking_recv().expect("Should receive response");

    assert!(result.is_err(), "Should return error");

    let error = result.unwrap_err();

    // Verify error is anyhow::Error (preserves type from VersionManager)
    // The actual error should be std::io::Error wrapped in anyhow
    let error_msg = format!("{:#}", error); // {:#} formats full error chain

    // Should contain I/O error context
    assert!(
        error_msg.contains("No such file") || error_msg.contains("not found"),
        "Error chain should preserve I/O error: {}",
        error_msg
    );

    app.shutdown().expect("Shutdown should succeed");
}

#[test]
fn test_actor_shutdown_after_rollback_error() {
    //! Test bd-76: Verify app can shut down cleanly after rollback error
    //!
    //! This test ensures that rollback errors don't corrupt actor state
    //! or prevent clean shutdown.

    ensure_config_dir();

    let settings = Settings::new(None).expect("Failed to create settings");
    let app = create_test_app_with_settings(settings);

    // Send rollback command with invalid version
    let version_id = VersionId("shutdown_test_version.toml".to_string());
    let (cmd, rx) = DaqCommand::rollback_to_version(version_id);

    app.command_tx
        .blocking_send(cmd)
        .expect("Command should be queued");

    let result = rx.blocking_recv().expect("Should receive response");
    assert!(result.is_err(), "Should return error for invalid version");

    // Shutdown should still work cleanly
    let shutdown_result = app.shutdown();
    assert!(
        shutdown_result.is_ok(),
        "Shutdown should succeed after rollback error: {:?}",
        shutdown_result
    );
}
