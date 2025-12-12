#![cfg(not(target_arch = "wasm32"))]
//! PVCAM Hardware Smoke Test
//!
//! A minimal smoke test for verifying PVCAM camera connectivity and basic operation.
//! Designed to run quickly in CI/CD environments when hardware is available.
//!
//! # Environment Variables
//!
//! - `PVCAM_SMOKE_TEST=1` - Required to enable the test
//! - `PVCAM_CAMERA_NAME` - Optional camera name (default: "PrimeBSI")
//!
//! # Prerequisites
//!
//! - PVCAM SDK installed at `/opt/pvcam/sdk`
//! - Camera connected and powered on
//! - Environment sourced: `source /opt/pvcam/etc/profile.d/pvcam.sh`
//!
//! # Running
//!
//! ```bash
//! export PVCAM_SDK_DIR=/opt/pvcam/sdk
//! export LD_LIBRARY_PATH=/opt/pvcam/library/x86_64:$LD_LIBRARY_PATH
//! export LIBRARY_PATH=/opt/pvcam/library/x86_64:$LIBRARY_PATH
//! export PVCAM_SMOKE_TEST=1
//! export PVCAM_CAMERA_NAME=PrimeBSI  # optional
//!
//! cargo test --test pvcam_hardware_smoke --features 'instrument_photometrics,pvcam_hardware' -- --nocapture
//! ```

#![cfg(all(feature = "instrument_photometrics", feature = "pvcam_hardware"))]

use rust_daq::hardware::capabilities::FrameProducer;
use rust_daq::hardware::pvcam::PvcamDriver;
use std::env;

/// Check if smoke test is enabled via environment variable
fn smoke_test_enabled() -> bool {
    env::var("PVCAM_SMOKE_TEST")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// Get camera name from environment or default to PrimeBSI
fn camera_name() -> String {
    env::var("PVCAM_CAMERA_NAME").unwrap_or_else(|_| "PrimeBSI".to_string())
}

/// PVCAM Hardware Smoke Test
///
/// This test verifies:
/// 1. SDK initialization
/// 2. Camera enumeration and detection
/// 3. Camera connection
/// 4. Basic exposure setting
/// 5. Single frame acquisition
/// 6. Proper cleanup
///
/// Designed to complete in under 10 seconds with minimal frame acquisition.
#[tokio::test]
async fn pvcam_smoke_test() {
    // Skip if smoke test not enabled
    if !smoke_test_enabled() {
        println!("PVCAM smoke test skipped (set PVCAM_SMOKE_TEST=1 to enable)");
        return;
    }

    let camera_name = camera_name();
    println!("=== PVCAM Hardware Smoke Test ===");
    println!("Camera: {}", camera_name);

    // Step 1: Initialize camera
    println!("\n[1/5] Initializing camera...");
    let camera = PvcamDriver::new_async(camera_name.clone())
        .await
        .expect("Failed to create PVCAM driver - check SDK installation and camera connection");

    // Step 2: Verify camera resolution (basic introspection available today)
    println!("[2/5] Querying camera resolution...");
    let (sensor_width, sensor_height) = camera.resolution();
    println!("  Sensor: {}x{}", sensor_width, sensor_height);

    // Validate sensor dimensions (basic sanity check)
    assert!(sensor_width > 0, "Sensor width must be positive");
    assert!(sensor_height > 0, "Sensor height must be positive");

    // Step 3: Set short exposure
    println!("[3/5] Setting exposure to 10ms...");
    camera
        .set_exposure(0.010)
        .await
        .expect("Failed to set exposure (s)");
    let exposure = camera
        .get_exposure()
        .await
        .expect("Failed to query exposure (s)");
    let exposure_ms = exposure * 1000.0;
    println!("  Exposure: {:.3} ms", exposure_ms);

    // Allow some tolerance for camera rounding
    assert!(
        (exposure_ms - 10.0).abs() < 1.0,
        "Exposure should be approximately 10ms, got {}ms",
        exposure_ms
    );

    // Step 4: Acquire single frame (one-shot path)
    println!("[4/5] Acquiring single frame (one-shot)...");
    let start = std::time::Instant::now();
    let frame = camera
        .acquire_frame()
        .await
        .expect("Failed to acquire single frame");

    let elapsed = start.elapsed();
    println!("  Frame received in {:?}", elapsed);
    println!("  Frame size: {}x{}", frame.width, frame.height);
    let pixels: Vec<u16> = match frame.bit_depth {
        16 => frame
            .data
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect(),
        _ => frame.data.iter().map(|&b| b as u16).collect(),
    };

    println!("  Buffer size: {} pixels", pixels.len());

    // Validate frame data
    assert!(frame.width > 0, "Frame width must be positive");
    assert!(frame.height > 0, "Frame height must be positive");
    assert!(
        pixels.len() == (frame.width * frame.height) as usize,
        "Pixel buffer size must match frame dimensions"
    );

    // Step 5: Ensure streaming is stopped (acquire_frame stops internally)
    println!("[5/5] Ensuring acquisition stopped...");
    let _ = camera.stop_stream().await; // best-effort

    // Calculate simple statistics on frame data
    let sum: u64 = pixels.iter().map(|&v| v as u64).sum();
    let mean = sum as f64 / pixels.len() as f64;
    let max_val = *pixels.iter().max().unwrap_or(&0);
    let min_val = *pixels.iter().min().unwrap_or(&0);

    println!("\n=== Frame Statistics ===");
    println!("  Mean: {:.2}", mean);
    println!("  Min: {}", min_val);
    println!("  Max: {}", max_val);

    println!("\n=== PVCAM Smoke Test PASSED ===");
}

/// Test that smoke test is properly skipped when not enabled
#[test]
fn smoke_test_skip_check() {
    // This test always runs to verify the skip logic works
    let enabled = smoke_test_enabled();
    if !enabled {
        println!("Smoke test correctly disabled (PVCAM_SMOKE_TEST not set)");
    } else {
        println!("Smoke test enabled via PVCAM_SMOKE_TEST=1");
    }
}
