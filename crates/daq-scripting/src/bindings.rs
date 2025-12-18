//! Hardware Bindings for Rhai Scripts
//!
//! This module provides the bridge between async Rust hardware traits and
//! synchronous Rhai scripts. It exposes hardware capabilities (Movable, Triggerable,
//! FrameProducer) as Rhai-compatible types and methods.
//!
//! # Architecture
//!
//! - `StageHandle` - Wraps devices implementing `Movable`
//! - `CameraHandle` - Wraps devices implementing `Triggerable + FrameProducer`
//! - `register_hardware()` - Registers all types and methods with Rhai engine
//!
//! # Async→Sync Bridge
//!
//! Uses a guarded `block_in_place` helper that only runs on Tokio's multi-thread
//! scheduler (current-thread would deadlock). This allows scripts to call
//! `stage.move_abs(10.0)` without dealing with async/await while keeping runtime
//! safety explicit.
//!
//! # Example Usage
//!
//! ```rust,ignore
//! let mut engine = Engine::new();
//! register_hardware(&mut engine);
//!
//! let mut scope = Scope::new();
//! scope.push("stage", StageHandle { driver: Arc::new(mock_stage) });
//!
//! let script = r#"
//!     stage.move_abs(10.0);
//!     let pos = stage.position();
//!     print("Position: " + pos);
//! "#;
//!
//! engine.eval_with_scope(&mut scope, script)?;
//! ```

use chrono::Utc;
use rhai::{Dynamic, Engine, EvalAltResult, Position};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::{Handle, RuntimeFlavor};
use tokio::sync::broadcast;
use tokio::task::block_in_place;

use daq_core::core::Measurement;
use daq_hardware::capabilities::{Camera, Movable};

// Helper to execute async hardware calls from synchronous Rhai functions without
// risking deadlock on a current-thread runtime. Returns a Rhai-compatible error
// with a clear message when the runtime flavor is unsupported.
fn run_blocking<Fut, T, E>(label: &str, fut: Fut) -> Result<T, Box<EvalAltResult>>
where
    Fut: std::future::Future<Output = Result<T, E>> + Send,
    T: Send,
    E: std::fmt::Display,
{
    let handle = Handle::try_current().map_err(|e| {
        Box::new(EvalAltResult::ErrorRuntime(
            format!("{}: missing Tokio runtime ({})", label, e).into(),
            Position::NONE,
        ))
    })?;

    if handle.runtime_flavor() == RuntimeFlavor::CurrentThread {
        return Err(Box::new(EvalAltResult::ErrorRuntime(
            format!(
                "{}: Tokio current-thread runtime cannot run blocking hardware calls. \
                 Use the multi-thread runtime (#[tokio::main(flavor = \"multi_thread\")]).",
                label
            )
            .into(),
            Position::NONE,
        )));
    }

    block_in_place(|| handle.block_on(fut)).map_err(|e| {
        Box::new(EvalAltResult::ErrorRuntime(
            format!("{}: {}", label, e).into(),
            Position::NONE,
        ))
    })
}

// =============================================================================
// Handle Types - Rhai-Compatible Wrappers
// =============================================================================

/// Soft position limits for stage safety (bd-jnfu.4)
///
/// These limits are enforced by the scripting layer BEFORE commands
/// are sent to hardware, preventing scripts from driving stages to
/// unsafe positions.
#[derive(Clone, Debug)]
pub struct SoftLimits {
    /// Minimum allowed position (None = no limit)
    pub min: Option<f64>,
    /// Maximum allowed position (None = no limit)
    pub max: Option<f64>,
}

impl SoftLimits {
    /// Create unlimited soft limits (no restrictions)
    pub fn unlimited() -> Self {
        Self {
            min: None,
            max: None,
        }
    }

    /// Create soft limits with min and max bounds
    pub fn new(min: f64, max: f64) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
        }
    }

    /// Check if a position is within soft limits
    pub fn validate(&self, position: f64) -> Result<(), String> {
        if let Some(min) = self.min {
            if position < min {
                return Err(format!(
                    "Position {} is below soft limit minimum {}",
                    position, min
                ));
            }
        }
        if let Some(max) = self.max {
            if position > max {
                return Err(format!(
                    "Position {} exceeds soft limit maximum {}",
                    position, max
                ));
            }
        }
        Ok(())
    }
}

impl Default for SoftLimits {
    fn default() -> Self {
        Self::unlimited()
    }
}

/// Handle to a stage device that can be used in Rhai scripts
///
/// Wraps any device implementing `Movable` trait (stages, actuators, goniometers).
/// Provides synchronous methods that scripts can call directly.
///
/// # Safety (bd-jnfu.4)
/// Soft limits can be configured to prevent scripts from commanding
/// hardware to unsafe positions. These are checked BEFORE any hardware
/// command is issued.
///
/// # Script Example
/// ```rhai
/// stage.move_abs(10.0);
/// stage.wait_settled();
/// let pos = stage.position();
/// print("Current position: " + pos + "mm");
/// ```
#[derive(Clone)]
pub struct StageHandle {
    /// Hardware driver implementing the Movable trait.
    ///
    /// This is typically an Arc-wrapped driver (e.g., `Arc<MockStage>`, `Arc<Esp300>`)
    /// that provides position control methods. The Arc enables sharing the driver
    /// across multiple script handles and async tasks.
    pub driver: Arc<dyn Movable>,
    /// Optional data sender for broadcasting measurements to RingBuffer/gRPC clients
    pub data_tx: Option<Arc<broadcast::Sender<Measurement>>>,
    /// Soft position limits (bd-jnfu.4) - validated before hardware commands
    pub soft_limits: SoftLimits,
}

/// Handle to a camera device that can be used in Rhai scripts
///
/// Wraps any device implementing `Camera` trait (which combines Triggerable + FrameProducer).
/// Provides synchronous methods for camera control.
///
/// # Script Example
/// ```rhai
/// camera.arm();
/// camera.trigger();
/// let res = camera.resolution();
/// print("Resolution: " + res[0] + "x" + res[1]);
/// ```
#[derive(Clone)]
pub struct CameraHandle {
    /// Hardware driver implementing the Camera trait.
    ///
    /// This is typically an Arc-wrapped driver (e.g., `Arc<MockCamera>`, `Arc<PvcamDriver>`)
    /// that provides camera control and frame acquisition methods. The Arc enables sharing
    /// the driver across multiple script handles and async tasks.
    pub driver: Arc<dyn Camera>,
    /// Optional data sender for broadcasting measurements to RingBuffer/gRPC clients
    pub data_tx: Option<Arc<broadcast::Sender<Measurement>>>,
}

// =============================================================================
// Hardware Registration
// =============================================================================

/// Register all hardware bindings with the Rhai engine
///
/// This function registers:
/// - Custom types: `Stage`, `Camera`
/// - Stage methods: `move_abs`, `move_rel`, `position`, `wait_settled`
/// - Camera methods: `arm`, `trigger`, `resolution`
/// - Utility functions: `sleep`
///
/// # Arguments
/// * `engine` - Mutable reference to Rhai engine
///
/// # Example
/// ```rust,ignore
/// let mut engine = Engine::new();
/// register_hardware(&mut engine);
/// ```
pub fn register_hardware(engine: &mut Engine) {
    // Register custom types with human-readable names
    engine.register_type_with_name::<StageHandle>("Stage");
    engine.register_type_with_name::<CameraHandle>("Camera");

    // =========================================================================
    // Stage Methods - Motion Control
    // =========================================================================

    // stage.move_abs(10.0) - Move to absolute position
    // SAFETY (bd-jnfu.4): Validates against soft limits BEFORE hardware command
    engine.register_fn(
        "move_abs",
        move |stage: &mut StageHandle, pos: f64| -> Result<Dynamic, Box<EvalAltResult>> {
            // Validate soft limits BEFORE issuing hardware command (bd-jnfu.4)
            if let Err(e) = stage.soft_limits.validate(pos) {
                return Err(Box::new(EvalAltResult::ErrorRuntime(
                    format!("Stage move_abs: soft limit violation: {}", e).into(),
                    Position::NONE,
                )));
            }

            run_blocking("Stage move_abs", stage.driver.move_abs(pos))?;

            // Send measurement to broadcast channel if sender available
            if let Some(ref tx) = stage.data_tx {
                let measurement = Measurement::Scalar {
                    name: "stage_position".to_string(),
                    value: pos,
                    unit: "mm".to_string(),
                    timestamp: Utc::now(),
                };

                // Ignore errors if no receivers (non-critical)
                let _ = tx.send(measurement);
            }

            Ok(Dynamic::UNIT)
        },
    );

    // stage.move_rel(5.0) - Move relative distance
    // SAFETY (bd-jnfu.4): Validates resulting position against soft limits
    // FIX (bd-jnfu.17): Convert to atomic move_abs to prevent TOCTOU race condition
    // If position changes between read and move, we still land at a validated position
    engine.register_fn(
        "move_rel",
        move |stage: &mut StageHandle, dist: f64| -> Result<Dynamic, Box<EvalAltResult>> {
            // Get current position to calculate target, then validate soft limits (bd-jnfu.4)
            let current_pos = run_blocking("Stage position", stage.driver.position())?;
            let target_pos = current_pos + dist;

            if let Err(e) = stage.soft_limits.validate(target_pos) {
                return Err(Box::new(EvalAltResult::ErrorRuntime(
                    format!(
                        "Stage move_rel: soft limit violation (current: {}, relative: {}, target: {}): {}",
                        current_pos, dist, target_pos, e
                    ).into(),
                    Position::NONE,
                )));
            }

            // Use move_abs instead of move_rel to eliminate TOCTOU race (bd-jnfu.17)
            // This ensures we move to the validated target position regardless of
            // any intermediate position changes (e.g., from concurrent controllers)
            run_blocking("Stage move_abs (atomic from move_rel)", stage.driver.move_abs(target_pos))?;
            Ok(Dynamic::UNIT)
        },
    );

    // let pos = stage.position() - Get current position
    engine.register_fn(
        "position",
        move |stage: &mut StageHandle| -> Result<f64, Box<EvalAltResult>> {
            run_blocking("Stage position", stage.driver.position())
        },
    );

    // stage.wait_settled() - Wait for motion to complete
    engine.register_fn(
        "wait_settled",
        move |stage: &mut StageHandle| -> Result<Dynamic, Box<EvalAltResult>> {
            run_blocking("Stage wait_settled", stage.driver.wait_settled())?;
            Ok(Dynamic::UNIT)
        },
    );

    // =========================================================================
    // Camera Methods - Acquisition Control
    // =========================================================================

    // camera.arm() - Prepare camera for trigger
    engine.register_fn(
        "arm",
        move |camera: &mut CameraHandle| -> Result<Dynamic, Box<EvalAltResult>> {
            run_blocking("Camera arm", camera.driver.arm())?;
            Ok(Dynamic::UNIT)
        },
    );

    // camera.trigger() - Capture frame
    engine.register_fn(
        "trigger",
        move |camera: &mut CameraHandle| -> Result<Dynamic, Box<EvalAltResult>> {
            run_blocking("Camera trigger", camera.driver.trigger())?;

            // Send measurement to broadcast channel if sender available
            if let Some(ref tx) = camera.data_tx {
                let measurement = Measurement::Scalar {
                    name: "camera_trigger".to_string(),
                    value: 1.0, // Trigger event indicator
                    unit: "event".to_string(),
                    timestamp: Utc::now(),
                };

                // Ignore errors if no receivers (non-critical)
                let _ = tx.send(measurement);
            }

            Ok(Dynamic::UNIT)
        },
    );

    // let res = camera.resolution() - Get [width, height] array
    engine.register_fn("resolution", move |camera: &mut CameraHandle| -> Dynamic {
        let (width, height) = camera.driver.resolution();
        Dynamic::from(vec![
            Dynamic::from(width as i64),
            Dynamic::from(height as i64),
        ])
    });

    // =========================================================================
    // Utility Functions
    // =========================================================================

    // sleep(0.5) - Async-aware sleep using Tokio (avoids blocking the runtime)
    engine.register_fn("sleep", |seconds: f64| {
        if let Ok(handle) = Handle::try_current() {
            if handle.runtime_flavor() == RuntimeFlavor::CurrentThread {
                // Avoid deadlock: fall back to std::thread::sleep on current-thread runtime
                std::thread::sleep(Duration::from_secs_f64(seconds));
                return;
            }

            let _ = block_in_place(|| {
                handle.block_on(tokio::time::sleep(Duration::from_secs_f64(seconds)))
            });
        } else {
            // No runtime available; use blocking sleep as a safe fallback
            std::thread::sleep(Duration::from_secs_f64(seconds));
        }
    });

    // =========================================================================
    // Mock Hardware Factories - For script testing and demos
    // =========================================================================

    // create_mock_stage() - Create a mock stage for testing (unlimited soft limits)
    engine.register_fn("create_mock_stage", || -> StageHandle {
        use daq_hardware::drivers::mock::MockStage;
        StageHandle {
            driver: Arc::new(MockStage::new()),
            data_tx: None,
            soft_limits: SoftLimits::unlimited(),
        }
    });

    // create_mock_stage_limited(min, max) - Create a mock stage with soft limits (bd-jnfu.4)
    engine.register_fn(
        "create_mock_stage_limited",
        |min: f64, max: f64| -> StageHandle {
            use daq_hardware::drivers::mock::MockStage;
            StageHandle {
                driver: Arc::new(MockStage::new()),
                data_tx: None,
                soft_limits: SoftLimits::new(min, max),
            }
        },
    );

    // create_mock_camera(width, height) - Create a mock camera for testing
    engine.register_fn(
        "create_mock_camera",
        |width: i64, height: i64| -> CameraHandle {
            use daq_hardware::drivers::mock::MockCamera;
            CameraHandle {
                driver: Arc::new(MockCamera::new(width as u32, height as u32)),
                data_tx: None,
            }
        },
    );

    // create_mock_power_meter(base_power) - Create a mock power meter for testing
    // Returns a StageHandle (using Readable trait exposed as stage for simplicity)
    engine.register_fn(
        "create_mock_power_meter",
        |_base_power: f64| -> StageHandle {
            // Mock power meter uses MockStage for now (no dedicated mock)
            // Real scripts should use actual Newport 1830-C
            use daq_hardware::drivers::mock::MockStage;
            StageHandle {
                driver: Arc::new(MockStage::new()),
                data_tx: None,
                soft_limits: SoftLimits::unlimited(),
            }
        },
    );

    // =========================================================================
    // Soft Limit Methods - Query and configure limits from scripts (bd-jnfu.4)
    // =========================================================================

    // stage.get_soft_limits() - Returns [min, max] or [] if unlimited
    engine.register_fn("get_soft_limits", |stage: &mut StageHandle| -> Dynamic {
        let limits = &stage.soft_limits;
        match (limits.min, limits.max) {
            (Some(min), Some(max)) => Dynamic::from(vec![
                Dynamic::from(min),
                Dynamic::from(max),
            ]),
            (Some(min), None) => Dynamic::from(vec![
                Dynamic::from(min),
                Dynamic::from(f64::INFINITY),
            ]),
            (None, Some(max)) => Dynamic::from(vec![
                Dynamic::from(f64::NEG_INFINITY),
                Dynamic::from(max),
            ]),
            (None, None) => Dynamic::from(Vec::<Dynamic>::new()),
        }
    });
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use daq_hardware::drivers::mock::{MockCamera, MockStage};
    use rhai::Scope;

    #[test]
    fn test_register_hardware_succeeds() {
        let mut engine = Engine::new();
        register_hardware(&mut engine); // Should not panic
    }

    #[test]
    fn test_stage_handle_clone() {
        let stage = Arc::new(MockStage::new());
        let handle1 = StageHandle {
            driver: stage.clone(),
            data_tx: None,
            soft_limits: SoftLimits::unlimited(),
        };
        let handle2 = handle1.clone();

        // Both handles should point to same underlying driver
        assert!(Arc::ptr_eq(&handle1.driver, &handle2.driver));
    }

    #[test]
    fn test_camera_handle_clone() {
        let camera = Arc::new(MockCamera::new(1920, 1080));
        let handle1 = CameraHandle {
            driver: camera.clone(),
            data_tx: None,
        };
        let handle2 = handle1.clone();

        assert!(Arc::ptr_eq(&handle1.driver, &handle2.driver));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_stage_methods_available() {
        let mut engine = Engine::new();
        register_hardware(&mut engine);

        let stage = Arc::new(MockStage::new());
        let mut scope = Scope::new();
        scope.push(
            "stage",
            StageHandle {
                driver: stage,
                data_tx: None,
                soft_limits: SoftLimits::unlimited(),
            },
        );

        // Test that all stage methods are registered and callable
        let script = r#"
            stage.move_abs(5.0);
            stage.move_rel(2.0);
            let pos = stage.position();
            stage.wait_settled();
            pos
        "#;

        let result = engine.eval_with_scope::<f64>(&mut scope, script).unwrap();
        assert_eq!(result, 7.0); // 5.0 + 2.0
    }

    /// Test that soft limits prevent out-of-range moves (bd-jnfu.4)
    #[tokio::test(flavor = "multi_thread")]
    async fn test_soft_limits_enforcement() {
        let mut engine = Engine::new();
        register_hardware(&mut engine);

        let stage = Arc::new(MockStage::new());
        let mut scope = Scope::new();
        scope.push(
            "stage",
            StageHandle {
                driver: stage,
                data_tx: None,
                soft_limits: SoftLimits::new(0.0, 100.0), // Limit: 0 to 100
            },
        );

        // Valid move should succeed
        let result = engine.eval_with_scope::<()>(&mut scope, "stage.move_abs(50.0);");
        assert!(result.is_ok(), "Move within limits should succeed");

        // Move above max should fail
        let result = engine.eval_with_scope::<()>(&mut scope, "stage.move_abs(150.0);");
        assert!(result.is_err(), "Move above max should fail");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("soft limit"), "Error should mention soft limit: {}", err);

        // Move below min should fail
        let result = engine.eval_with_scope::<()>(&mut scope, "stage.move_abs(-10.0);");
        assert!(result.is_err(), "Move below min should fail");

        // Relative move that would exceed limits should fail
        let result = engine.eval_with_scope::<()>(&mut scope, "stage.move_rel(60.0);"); // 50 + 60 = 110 > 100
        assert!(result.is_err(), "Relative move exceeding limits should fail");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_camera_methods_available() {
        let mut engine = Engine::new();
        register_hardware(&mut engine);

        let camera = Arc::new(MockCamera::new(1920, 1080));
        let mut scope = Scope::new();
        scope.push(
            "camera",
            CameraHandle {
                driver: camera,
                data_tx: None,
            },
        );

        // Test camera methods
        let script = r#"
            camera.arm();
            camera.trigger();
            let res = camera.resolution();
            res[0]
        "#;

        let result = engine.eval_with_scope::<i64>(&mut scope, script).unwrap();
        assert_eq!(result, 1920);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_sleep_function() {
        let mut engine = Engine::new();
        register_hardware(&mut engine);

        let start = std::time::Instant::now();
        // We need to run this in a spawn_blocking or similar if calling from async test directly,
        // but rhai_fn is registered with block_in_place.
        // However, calling engine.eval from async context is tricky if the fn calls block_in_place.
        // Actually, block_in_place works inside a multithreaded runtime.

        // Note: engine.eval is synchronous.
        engine.eval::<()>("sleep(0.1)").unwrap();

        let elapsed = start.elapsed();

        // Should sleep for ~100ms (allow some tolerance)
        assert!(elapsed.as_millis() >= 95);
        assert!(elapsed.as_millis() <= 150);
    }
}
