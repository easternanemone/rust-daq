# daq-driver-mock

Mock hardware drivers for rust-daq testing and development. Provides simulated devices that mimic real hardware behavior without requiring physical equipment.

## Overview

This crate provides three mock device implementations:

1. **MockStage** - Simulated motorized motion stage
2. **MockCamera** - Simulated scientific camera with frame streaming
3. **MockPowerMeter** - Simulated optical power meter

All mock devices use async-safe operations (`tokio::time::sleep`) and can be used in integration tests, UI development, and demos.

## Quick Start

### Basic Usage

```rust
use daq_driver_mock::{MockStageFactory, MockCameraFactory, MockPowerMeterFactory};
use daq_core::driver::DriverFactory;

// Create factory instances
let stage_factory = MockStageFactory;
let camera_factory = MockCameraFactory;
let meter_factory = MockPowerMeterFactory;

// Register with device registry
registry.register_factory(Box::new(stage_factory));
registry.register_factory(Box::new(camera_factory));
registry.register_factory(Box::new(meter_factory));

// Create devices from TOML config
let stage_config = toml::toml! {
    // MockStage accepts empty config or custom parameters
};
let stage = stage_factory.build(stage_config.into()).await?;
```

### Configuration Examples

#### MockStage

```toml
[[devices]]
id = "mock_stage"
type = "mock_stage"
enabled = true

[devices.config]
# No required options; all are defaults
# Optional: Add custom travel limits or speed profiles
```

#### MockCamera

```toml
[[devices]]
id = "mock_camera"
type = "mock_camera"
enabled = true

[devices.config]
# Configuration options (all optional):
# width = 640
# height = 480
# fps = 30
```

#### MockPowerMeter

```toml
[[devices]]
id = "mock_power_meter"
type = "mock_power_meter"
enabled = true

[devices.config]
# Configuration options (all optional):
# min_power = 0.0
# max_power = 5.0
# noise_level = 0.01  # 1% noise
```

## Device Specifications

### MockStage - Simulated Motion Stage

**Capabilities:** Movable, Parameterized

**Characteristics:**
- Travel range: 0-25 mm (default)
- Motion speed: 10 mm/sec (realistic scanning speed)
- Settling time: 50 ms after reaching target
- Acceleration: Smooth ramp profile
- Backlash: Simulated for realism

**Implementation:**
- Uses `tokio::time::sleep` for motion delay (async-safe)
- Tracks position, velocity, and acceleration
- Simulates realistic mechanical behavior

**Usage:**

```rust
use daq_driver_mock::MockStageFactory;

let stage_factory = MockStageFactory;
let config = toml::toml! {};
let components = stage_factory.build(config.into()).await?;

let movable = components.movable.unwrap();

// Move to absolute position
movable.move_abs(10.0).await?;  // Move to 10mm

// Get current position
let pos = movable.get_position().await?;
println!("Position: {} mm", pos);

// Move relative
movable.move_rel(2.0).await?;   // Move +2mm
```

### MockCamera - Simulated Scientific Camera

**Capabilities:** FrameProducer, Triggerable, Parameterized

**Characteristics:**
- Resolution: 640×480 pixels (default, configurable)
- Frame rate: 30 FPS (33 ms per frame)
- Bit depth: 16-bit grayscale
- Exposure control: 1-100 ms
- Trigger modes: Software and external simulation
- Pattern generation: Gradient, noise, and checkerboard

**Pattern Types:**
- **Gradient:** Intensity increases left-to-right (for focus testing)
- **Gaussian:** Gaussian blob (useful for centroid algorithms)
- **Noise:** Random noise (for noise analysis)
- **Checkerboard:** Regular pattern (for calibration)

**Implementation:**
- Uses `tokio::time::sleep` for frame timing (async-safe)
- Generates synthetic frames on-demand (no file I/O)
- Supports configurable exposure and gain
- Realistic pixel values in 16-bit range

**Usage:**

```rust
use daq_driver_mock::MockCameraFactory;

let camera_factory = MockCameraFactory;
let config = toml::toml! {
    width = 640
    height = 480
};
let components = camera_factory.build(config.into()).await?;

let frame_producer = components.frame_producer.unwrap();

// Start streaming
frame_producer.start_streaming().await?;

// Grab frame
let frame = frame_producer.grab_frame().await?;
println!("Frame: {}×{}, {} bytes", frame.width, frame.height, frame.data.len());

// Set exposure
frame_producer.set_parameter("exposure_ms", 10.0).await?;

// Stop streaming
frame_producer.stop_streaming().await?;
```

### MockPowerMeter - Simulated Power Sensor

**Capabilities:** Readable, WavelengthTunable, Parameterized

**Characteristics:**
- Measurement range: 0-5 W (default, configurable)
- Noise level: ~1% (realistic uncertainty)
- Reading frequency: 1 Hz (1 second per measurement)
- Wavelength sensitivity: Simulated wavelength-dependent response
- Response time: 100 ms

**Noise Characteristics:**
- Gaussian noise with configurable sigma
- Realistic power fluctuations
- Useful for testing noise-handling algorithms

**Implementation:**
- Uses `tokio::time::sleep` for measurement delay
- Generates realistic noisy readings
- Simulates wavelength-dependent responsivity
- Never returns negative values (physical constraint)

**Usage:**

```rust
use daq_driver_mock::MockPowerMeterFactory;

let meter_factory = MockPowerMeterFactory;
let config = toml::toml! {
    min_power = 0.0
    max_power = 5.0
};
let components = meter_factory.build(config.into()).await?;

let readable = components.readable.unwrap();

// Read power
let power_w = readable.read_value().await?;
println!("Power: {:.3} W", power_w);

// Wavelength tuning affects response
let tunable = components.wavelength_tunable.unwrap();
tunable.set_wavelength(850.0).await?;
let power_at_850 = readable.read_value().await?;
```

## Feature Flags

All mock drivers use the `default` feature set:

```toml
# Cargo.toml
[features]
default = ["mock_stage", "mock_camera", "mock_power_meter"]

mock_stage = []
mock_camera = []
mock_power_meter = []
```

To disable specific drivers:

```toml
# Reduce compiled code size
daq-driver-mock = { path = "...", default-features = false, features = ["mock_camera"] }
```

## Testing Patterns

### Unit Test Template

```rust
#[tokio::test]
async fn test_mock_stage_movement() -> anyhow::Result<()> {
    use daq_driver_mock::MockStageFactory;

    let factory = MockStageFactory;
    let config = toml::toml! {};
    let components = factory.build(config.into()).await?;

    let movable = components.movable.unwrap();

    // Test absolute movement
    movable.move_abs(5.0).await?;
    let pos = movable.get_position().await?;
    assert!((pos - 5.0).abs() < 0.01);  // Within 0.01mm

    // Test relative movement
    movable.move_rel(2.0).await?;
    let pos = movable.get_position().await?;
    assert!((pos - 7.0).abs() < 0.01);

    Ok(())
}
```

### Integration Test Template

```rust
#[tokio::test]
async fn test_multi_device_orchestration() -> anyhow::Result<()> {
    use daq_driver_mock::{MockStageFactory, MockCameraFactory};

    // Create devices
    let stage = MockStageFactory
        .build(toml::toml! {}.into())
        .await?;
    let camera = MockCameraFactory
        .build(toml::toml! {}.into())
        .await?;

    let movable = stage.movable.unwrap();
    let frame_producer = camera.frame_producer.unwrap();

    // Coordinated movement and imaging
    movable.move_abs(0.0).await?;
    frame_producer.start_streaming().await?;

    for i in 0..10 {
        movable.move_abs(i as f64 * 2.0).await?;
        let frame = frame_producer.grab_frame().await?;
        println!("Position: {}, Frame: {}", i * 2, frame.width);
    }

    frame_producer.stop_streaming().await?;
    Ok(())
}
```

## Performance Characteristics

All timings are intentionally conservative to be realistic:

| Device | Operation | Time | Notes |
|--------|-----------|------|-------|
| MockStage | Move 10mm | ~1 sec | 10mm/sec + 50ms settle |
| MockStage | Set position | <1ms | Instant, no I/O |
| MockCamera | Grab frame | 33ms | ~30 FPS |
| MockCamera | Start stream | <1ms | Immediate |
| MockPowerMeter | Read value | ~100ms | Measurement delay |
| MockPowerMeter | Read cached | <1ms | Local variable |

## Synthetic Data Generation

### Camera Patterns

The `generate_test_pattern()` function creates synthetic image patterns:

```rust
use daq_driver_mock::generate_test_pattern;

// Generate gradient pattern (for focus testing)
let gradient = generate_test_pattern("gradient", 640, 480)?;

// Generate Gaussian blob (for centroid testing)
let gaussian = generate_test_pattern("gaussian", 640, 480)?;

// Generate random noise
let noise = generate_test_pattern("noise", 640, 480)?;

// Generate checkerboard (for calibration)
let checker = generate_test_pattern("checkerboard", 640, 480)?;
```

### Power Meter Noise

Realistic power readings include Gaussian noise:

```rust
// Base power reading
let base_power = 2.5;  // 2.5W

// Add noise (sigma=0.01 for 1% noise)
let noisy_power = base_power + random_normal(0.0, 0.01);
println!("Power: {:.3} W", noisy_power);
```

## Convenience Functions

### Register All Factories

```rust
use daq_driver_mock::register_all;

// Register all mock drivers with the registry
register_all(&registry);
```

### Link Function (for Linker)

```rust
use daq_driver_mock::link;

// Call from main() to prevent linker optimization
fn main() {
    daq_driver_mock::link();
    // Rest of initialization...
}
```

## Use Cases

### Scenario 1: UI Development Without Hardware

```rust
// No real hardware required
let config = toml::toml! {
    id = "mock_camera"
    type = "mock_camera"
    enabled = true
};

// UI can display frames in development
let components = MockCameraFactory.build(config.into()).await?;
let frames = components.frame_producer.unwrap();
```

### Scenario 2: Integration Tests

```rust
// Fast, deterministic testing without hardware
#[tokio::test]
async fn test_wavelength_scan() -> anyhow::Result<()> {
    let camera = MockCameraFactory.build(toml::toml! {}.into()).await?;
    let laser = MockPowerMeterFactory.build(toml::toml! {}.into()).await?;

    // Scan wavelength and verify camera responds
    // No actual hardware, no timing issues
    Ok(())
}
```

### Scenario 3: Demos and Tutorials

```rhai
// Rhai script using mock devices
let stage = create_mock_stage();
let camera = create_mock_camera();

for i in range(0, 10) {
    stage.move_to(i * 1.0);
    let frame = camera.grab_frame();
    print(`Frame ${i}: ${frame.width}x${frame.height}`);
}
```

## Troubleshooting

### Device Not Registering

```
Error: "No factory found for type 'mock_stage'"
```

**Solution:** Ensure `link()` is called or factory is explicitly registered:

```rust
use daq_driver_mock::MockStageFactory;

// Option 1: Call link()
daq_driver_mock::link();

// Option 2: Register directly
registry.register_factory(Box::new(MockStageFactory));
```

### Unexpected Delays

```
Operation taking longer than expected
```

**Solution:** Mock timings are intentionally realistic. Reduce expectations:

```rust
// Mock operations have realistic delays
// MockCamera: 33ms per frame
// MockStage: ~1 sec for 10mm movement
// Adjust test timeouts accordingly
```

### Configuration Not Applied

```
Mock device ignoring configuration values
```

**Solution:** Check configuration format is valid TOML:

```toml
[devices.config]
width = 640      # Valid: number
height = 480
fps = 30

# Also valid: string configuration
[devices.config]
pattern = "gradient"  # For camera
mode = "gradient"     # For pattern selection
```

## Dependencies

- `tokio` - Async runtime for sleep operations
- `anyhow` - Error handling
- `serde` - TOML configuration

## See Also

- [Hardware Drivers Guide](../../docs/guides/hardware-drivers.md) - General driver patterns
- [Testing Guide](../../docs/guides/testing.md) - Testing with real and mock hardware
- [DEMO.md](../../DEMO.md) - Quick start guide using mock devices

## Mock vs Real Hardware

| Aspect | Mock | Real |
|--------|------|------|
| **Setup** | None | Hardware + cables |
| **Speed** | Fast (simulated delays) | Slower (actual timings) |
| **Reliability** | 100% (deterministic) | 95% (equipment variance) |
| **Cost** | Free | Hardware cost |
| **Learning** | Excellent (no equipment risk) | Required for deployment |
| **Debugging** | Easy (synthetic data) | Harder (physical constraints) |

## Contributing

To add a new mock device:

1. Create new module in `src/mock_*.rs`
2. Implement `DriverFactory` trait
3. Implement capability traits (e.g., `Movable`, `Readable`)
4. Add to `lib.rs` exports and `link()` function
5. Add tests to validate behavior
6. Document in this README

---

**Note:** Mock devices are useful for development but should NOT be used in production. Always test with real hardware before deployment.
