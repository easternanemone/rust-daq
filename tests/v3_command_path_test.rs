//! V3 Command Path Integration Test
//!
//! Verifies end-to-end command routing for V3 instruments:
//! - `InstrumentManagerV3::execute_command`
//! - Per-instrument command channels
//! - `MockPowerMeterV3` command handling (Start, Stop, Configure)

use anyhow::Result;
use rust_daq::config::InstrumentConfigV3;
use rust_daq::core::ParameterValue;
use rust_daq::core_v3::{Command, Response};
use rust_daq::instrument_manager_v3::InstrumentManagerV3;
use rust_daq::instruments_v2::MockPowerMeterV3;
use std::collections::HashMap;

#[tokio::test]
async fn test_v3_command_path() -> Result<()> {
    // 1. Create manager and register factory
    let mut manager = InstrumentManagerV3::new();
    manager.register_factory("MockPowerMeterV3", MockPowerMeterV3::from_config);

    // 2. Load mock instrument
    let cfg = InstrumentConfigV3 {
        id: "power_meter_1".to_string(),
        type_name: "MockPowerMeterV3".to_string(),
        settings: serde_json::json!({
            "sampling_rate": 10.0,
            "wavelength_nm": 532.0
        }),
    };
    manager.load_from_config(&[cfg]).await?;

    let instrument_id = "power_meter_1";

    // 3. Subscribe to measurements
    let mut rx = manager.subscribe_measurements(instrument_id).await?;

    // 4. Send Start and verify data flow
    let response = manager
        .execute_command(instrument_id, Command::Start)
        .await?;
    assert!(matches!(response, Response::Ok));

    tokio::select! {
        result = rx.recv() => {
            assert!(result.is_ok(), "Should receive measurement after Start");
        }
        _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
            panic!("No measurement received after Start command");
        }
    }

    // 5. Send Stop and verify data flow stops
    let response = manager
        .execute_command(instrument_id, Command::Stop)
        .await?;
    assert!(matches!(response, Response::Ok));

    tokio::select! {
        recv_result = rx.recv() => {
            if let Err(err) = recv_result {
                panic!("Error receiving measurement after Stop: {err}");
            }
            // This might receive a value that was already in the buffer.
            // We'll try receiving again with a short timeout to confirm it has stopped.
            tokio::select! {
                res2 = rx.recv() => {
                    panic!("Should not have received a second measurement after Stop, but got {:?}", res2);
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                    // This is the expected outcome
                }
            }
        }
        _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
            // This is the expected outcome if the channel was already empty.
        }
    }

    // 6. Send Configure to change sampling rate
    let mut params = HashMap::new();
    params.insert("sampling_rate".to_string(), ParameterValue::Float(20.0));
    let response = manager
        .execute_command(instrument_id, Command::Configure { params })
        .await?;
    assert!(matches!(response, Response::Ok));

    // 7. Send Start again and verify data flow resumes
    let response = manager
        .execute_command(instrument_id, Command::Start)
        .await?;
    assert!(matches!(response, Response::Ok));

    let mut timestamps = Vec::new();
    for _ in 0..5 {
        tokio::select! {
            result = rx.recv() => {
                let measurement = result?;
                timestamps.push(measurement.timestamp());
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                panic!("Timed out waiting for measurement after re-Start");
            }
        }
    }

    // 8. Verify new sampling rate (20 Hz -> 50ms interval)
    assert_eq!(timestamps.len(), 5);
    for window in timestamps.windows(2) {
        let duration = window[1].signed_duration_since(window[0]);
        let duration_ms = duration.num_milliseconds() as f64;
        // Allow for some timing variance
        assert!(
            (40.0..=60.0).contains(&duration_ms),
            "Interval should be ~50ms, but was {}ms",
            duration_ms
        );
    }

    // 9. Shutdown
    manager.shutdown_all().await?;

    Ok(())
}
