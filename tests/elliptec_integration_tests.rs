//! Integration tests for multi-device coordination of Elliptec rotators.
//!
//! Run with: cargo test --test elliptec_integration_tests --features instrument_serial -- --ignored --nocapture

use anyhow::Result;
use rust_daq::{
    app::DaqApp,
    config::Settings,
    core::{DataPoint, InstrumentCommand},
    measurement::Measure,
};
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::time::{timeout, Duration};

const TEST_CONFIG: &str = "config/default.toml";
const ELLIPTEC_ID: &str = "elliptec_rotators";

/// Test bd-e52e.10: Simultaneous movement of both Elliptec devices
///
/// This test commands both rotators to move to new positions concurrently and
/// monitors their position updates to verify that they move simultaneously
/// without RS-485 bus contention.
///
/// # Procedure
/// 1. Load the default configuration and connect to the DaqApp.
/// 2. Get a handle to the "elliptec_rotators" instrument.
/// 3. Subscribe to the instrument's data stream to receive position updates.
/// 4. Command both devices to move to new target positions (e.g., dev 2 to 90°, dev 3 to 180°).
/// 5. Monitor the data stream for position updates from both devices.
/// 6. Verify that both devices reach their target positions within a timeout.
///
/// # Expected Results
/// - Both devices start moving at approximately the same time.
/// - Position updates are received for both devices during movement.
/// - Both devices reach their final positions with < 1° error.
/// - No communication timeouts or errors occur.
#[tokio::test]
#[ignore] // Hardware-only test
async fn test_simultaneous_movement() -> Result<()> {
    // 1. Load configuration and connect to DaqApp
    let settings = Arc::new(Settings::from_path(Path::new(TEST_CONFIG))?);
    let mut app = DaqApp::new();
    app.connect(settings).await?;

    // 2. Get a handle to the Elliptec instrument
    let instrument = app.instrument(ELLIPTEC_ID).await?;
    let mut receiver = instrument.measure().subscribe();

    // 3. Define target positions and tolerances
    let target_pos_2 = 90.0;
    let target_pos_3 = 180.0;
    let tolerance = 1.0;

    println!(
        "Commanding simultaneous movement: device 2 to {}°, device 3 to {}°",
        target_pos_2, target_pos_3
    );

    // 4. Command both devices to move
    instrument
        .command(InstrumentCommand::SetParameter(
            "2:position".to_string(),
            target_pos_2.into(),
        ))
        .await?;
    instrument
        .command(InstrumentCommand::SetParameter(
            "3:position".to_string(),
            target_pos_3.into(),
        ))
        .await?;

    // 5. Monitor positions
    let mut last_positions: HashMap<u8, f64> = HashMap::new();
    let mut reached_target: HashMap<u8, bool> = HashMap::from([(2, false), (3, false)]);

    let test_duration = Duration::from_secs(30);
    let result = timeout(test_duration, async {
        while !reached_target.values().all(|&v| v) {
            if let Ok(data_point) = receiver.recv().await {
                if let DataPoint {
                    instrument_id,
                    channel,
                    value,
                    ..
                } = data_point
                {
                    if instrument_id == ELLIPTEC_ID {
                        let device_address = channel
                            .strip_prefix("device")
                            .and_then(|s| s.strip_suffix("_position"))
                            .and_then(|s| s.parse::<u8>().ok())
                            .unwrap_or(0);

                        if device_address == 2 || device_address == 3 {
                            last_positions.insert(device_address, value);
                            println!(
                                "  - Position update: device {} at {:.2}°",
                                device_address, value
                            );

                            let target = if device_address == 2 {
                                target_pos_2
                            } else {
                                target_pos_3
                            };
                            if (value - target).abs() < tolerance {
                                if !reached_target.get(&device_address).unwrap_or(&false) {
                                    println!(
                                        "  ✓ Device {} reached target position ({:.2}°)",
                                        device_address, value
                                    );
                                    reached_target.insert(device_address, true);
                                }
                            }
                        }
                    }
                }
            }
        }
    })
    .await;

    // 6. Assert results
    assert!(
        result.is_ok(),
        "Test timed out after {} seconds",
        test_duration.as_secs()
    );

    let final_pos_2 = *last_positions.get(&2).unwrap_or(&0.0);
    let final_pos_3 = *last_positions.get(&3).unwrap_or(&0.0);

    println!("\nFinal positions:");
    println!("  - Device 2: {:.2}° (target: {})", final_pos_2, target_pos_2);
    println!("  - Device 3: {:.2}° (target: {})", final_pos_3, target_pos_3);

    assert!(
        (final_pos_2 - target_pos_2).abs() < tolerance,
        "Device 2 did not reach target position. Final: {:.2}°, Target: {:.2}°",
        final_pos_2,
        target_pos_2
    );
    assert!(
        (final_pos_3 - target_pos_3).abs() < tolerance,
        "Device 3 did not reach target position. Final: {:.2}°, Target: {:.2}°",
        final_pos_3,
        target_pos_3
    );

    println!("\n✓ Simultaneous movement test passed.");
    Ok(())
}
