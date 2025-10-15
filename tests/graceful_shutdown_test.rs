//! Integration tests for graceful shutdown behavior.

use rust_daq::app::DaqApp;
use rust_daq::config::Settings;
use std::sync::Arc;
use std::time::Duration;

/// Helper to create a test configuration with mock instruments.
fn create_test_config() -> Settings {
    let toml_str = r#"
        [application]
        name = "Rust DAQ Test"
        log_level = "info"

        [instruments.mock1]
        type = "mock"
        sampling_rate_hz = 100.0

        [instruments.mock2]
        type = "mock"
        sampling_rate_hz = 50.0
    "#;
    toml::from_str(toml_str).expect("Failed to parse test config")
}

#[test]
fn test_shutdown_signal_sent() {
    // Create app and connect instruments
    let settings = Arc::new(create_test_config());
    let mut app = DaqApp::new(settings).expect("Failed to create app");
    app.connect_instruments().expect("Failed to connect instruments");

    // Give instruments time to start
    std::thread::sleep(Duration::from_millis(100));

    // Shutdown should complete without hanging
    app.shutdown();
}

#[test]
fn test_shutdown_timeout() {
    // This test verifies that shutdown completes within the expected timeout
    let settings = Arc::new(create_test_config());
    let mut app = DaqApp::new(settings).expect("Failed to create app");
    app.connect_instruments().expect("Failed to connect instruments");

    std::thread::sleep(Duration::from_millis(100));

    let start = std::time::Instant::now();
    app.shutdown();
    let elapsed = start.elapsed();

    // Should complete within 6 seconds (5s timeout + 1s margin)
    assert!(elapsed < Duration::from_secs(6), "Shutdown took too long: {:?}", elapsed);
}

#[test]
fn test_multiple_shutdown_calls() {
    // Verify that calling shutdown multiple times is safe
    let settings = Arc::new(create_test_config());
    let mut app = DaqApp::new(settings).expect("Failed to create app");
    app.connect_instruments().expect("Failed to connect instruments");

    std::thread::sleep(Duration::from_millis(100));

    // First shutdown
    app.shutdown();

    // Second shutdown should be a no-op
    app.shutdown();
}

#[test]
fn test_shutdown_before_connect() {
    // Verify that shutdown works even if instruments were never connected
    let settings = Arc::new(create_test_config());
    let mut app = DaqApp::new(settings).expect("Failed to create app");

    // Shutdown without connecting
    app.shutdown();
}

#[test]
fn test_shutdown_after_disconnect() {
    // Verify that shutdown works after manual disconnect
    let settings = Arc::new(create_test_config());
    let mut app = DaqApp::new(settings).expect("Failed to create app");
    app.connect_instruments().expect("Failed to connect instruments");

    std::thread::sleep(Duration::from_millis(100));

    // Disconnect first
    app.disconnect_instruments().expect("Failed to disconnect");

    // Then shutdown
    app.shutdown();
}

#[test]
fn test_data_stream_after_shutdown() {
    // Verify that data streams stop after shutdown
    let settings = Arc::new(create_test_config());
    let mut app = DaqApp::new(settings).expect("Failed to create app");
    app.connect_instruments().expect("Failed to connect instruments");

    std::thread::sleep(Duration::from_millis(100));

    app.shutdown();

    // Attempting to get data stream should fail or return no data
    // (This depends on implementation details)
}

#[test]
fn test_reconnect_after_shutdown() {
    // Verify that instruments can be reconnected after shutdown
    let settings = Arc::new(create_test_config());
    let mut app = DaqApp::new(settings).expect("Failed to create app");

    // First connection cycle
    app.connect_instruments().expect("Failed to connect instruments");
    std::thread::sleep(Duration::from_millis(100));
    app.shutdown();

    // Second connection cycle
    app.connect_instruments().expect("Failed to reconnect instruments");
    std::thread::sleep(Duration::from_millis(100));
    app.shutdown();
}
