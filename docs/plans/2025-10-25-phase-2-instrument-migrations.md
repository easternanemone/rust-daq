# Phase 2: Instrument Migrations to V3 Architecture

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Migrate all remaining instruments from V1/V2 to unified V3 architecture using PVCAM V3 as the reference implementation.

**Architecture:** Direct async trait methods replacing actor model, single broadcast channel for data streaming, `Parameter<T>` abstraction for type-safe parameter management, SDK abstraction layers with RAII guards where applicable.

**Tech Stack:** Rust async/await, tokio runtime, Arc/RwLock for shared state, broadcast channels for data distribution, async_trait for trait async methods.

**Reference Implementation:** `src/instruments_v2/pvcam_v3.rs` (754 lines, 6/6 tests passing, validated by Gemini review)

**Success Criteria:**
- All V3 instruments implement `Instrument` + meta-trait (Camera/PowerMeter/Stage/Laser)
- 100% test coverage with passing tests
- No `block_on` calls in async contexts
- Proper RAII patterns for resource management
- Performance equal or better than V1/V2

---

## Migration Priority Order

1. **Newport 1830C** (PowerMeter) - Simplest, validates PowerMeter trait
2. **ESP300** (Stage) - Validates Stage trait, moderate complexity
3. **MaiTai** (Laser) - Validates Laser trait, simple API
4. **Elliptec** (Stage) - Second Stage implementation, validates trait reusability
5. **VISA/SCPI** (Generic) - Generic instrument pattern, validates extensibility

---

## Task 1: Newport 1830C V3 (PowerMeter)

**Estimated Time:** 2-3 hours
**Complexity:** Low (simple scalar measurements)

**Files:**
- Create: `src/instruments_v2/newport_1830c_v3.rs` (~400 lines)
- Modify: `src/instruments_v2/mod.rs` (add export)
- Reference: `src/instruments_v2/newport_1830c.rs` (V2, 487 lines)
- Test: Create 6 tests matching PVCAM V3 pattern

### Step 1: Analyze V2 implementation

**Read and document:**
```bash
# Read existing V2 implementation
cat src/instruments_v2/newport_1830c.rs | head -100
```

**Document findings:**
- Connection type: Serial (via serialport crate)
- Commands: "PM:P?" (read power), "*IDN?" (identify)
- Measurement type: Scalar power readings
- Parameters: Wavelength, range, units
- No SDK abstraction needed (simple serial protocol)

**Commit:**
```bash
# In docs/plans/ or as comments in code
git add docs/plans/newport-1830c-v3-analysis.md
git commit -m "docs: analyze Newport 1830C V2 for migration"
```

### Step 2: Write failing test - initialization

**File:** `src/instruments_v2/newport_1830c_v3.rs`

**Add test module:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_newport_1830c_v3_initialization() {
        // Mock serial port will be created in struct
        let mut power_meter = Newport1830CV3::new("test_pm", "/dev/tty.mock");
        assert_eq!(power_meter.state(), InstrumentState::Uninitialized);

        power_meter.initialize().await.unwrap();
        assert_eq!(power_meter.state(), InstrumentState::Idle);
    }
}
```

**Run test to verify it fails:**
```bash
cargo test --lib newport_1830c_v3::tests::test_newport_1830c_v3_initialization
# Expected: compilation error - Newport1830CV3 not defined
```

### Step 3: Create struct skeleton

**File:** `src/instruments_v2/newport_1830c_v3.rs`

**Write minimal implementation:**
```rust
//! Newport 1830C Power Meter V3
//!
//! Unified architecture implementation using PowerMeter trait.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::core_v3::{
    Instrument, InstrumentState, Measurement, ParameterBase, PowerMeter, Response, Command,
};
use crate::parameter::{Parameter, ParameterBuilder};

/// Newport 1830C Power Meter V3 implementation
pub struct Newport1830CV3 {
    id: String,
    state: InstrumentState,
    data_tx: broadcast::Sender<Measurement>,
    parameters: HashMap<String, Box<dyn ParameterBase>>,

    // Serial connection
    port_name: String,
    serial_port: Option<Box<dyn serialport::SerialPort>>,

    // Parameters
    wavelength_nm: Arc<RwLock<Parameter<f64>>>,
    range: Arc<RwLock<Parameter<String>>>,
}

impl Newport1830CV3 {
    pub fn new(id: impl Into<String>, port_name: impl Into<String>) -> Self {
        let id = id.into();
        let (data_tx, _) = broadcast::channel(1024);

        let wavelength_nm = Arc::new(RwLock::new(
            ParameterBuilder::new("wavelength_nm", 532.0)
                .description("Laser wavelength for calibration")
                .unit("nm")
                .range(200.0, 1100.0)
                .build(),
        ));

        let range = Arc::new(RwLock::new(
            ParameterBuilder::new("range", "auto".to_string())
                .description("Power measurement range")
                .choices(vec!["auto".to_string(), "1mW".to_string(), "10mW".to_string()])
                .build(),
        ));

        Self {
            id,
            state: InstrumentState::Uninitialized,
            data_tx,
            parameters: HashMap::new(),
            port_name: port_name.into(),
            serial_port: None,
            wavelength_nm,
            range,
        }
    }
}

#[async_trait]
impl Instrument for Newport1830CV3 {
    fn id(&self) -> &str {
        &self.id
    }

    fn state(&self) -> InstrumentState {
        self.state
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state = InstrumentState::Idle;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state = InstrumentState::ShuttingDown;
        Ok(())
    }

    fn data_channel(&self) -> broadcast::Receiver<Measurement> {
        self.data_tx.subscribe()
    }

    async fn execute(&mut self, _cmd: Command) -> Result<Response> {
        Ok(Response::Ok)
    }

    fn parameters(&self) -> &HashMap<String, Box<dyn ParameterBase>> {
        &self.parameters
    }
}

#[async_trait]
impl PowerMeter for Newport1830CV3 {
    // Note: read_power() is NOT in PowerMeter trait - data flows via broadcast channel
    // PowerMeter trait only has control methods: set_wavelength, set_range, zero
    // Implementations may add convenience methods like read_power() as needed
    
    async fn set_wavelength(&mut self, nm: f64) -> Result<()> {
        self.wavelength_nm.write().await.set(nm).await
    }

    async fn set_range(&mut self, watts: f64) -> Result<()> {
        // TODO: Implement
        Ok(())
    }
    
    async fn zero(&mut self) -> Result<()> {
        // TODO: Implement
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_newport_1830c_v3_initialization() {
        let mut power_meter = Newport1830CV3::new("test_pm", "/dev/tty.mock");
        assert_eq!(power_meter.state(), InstrumentState::Uninitialized);

        power_meter.initialize().await.unwrap();
        assert_eq!(power_meter.state(), InstrumentState::Idle);
    }
}
```

**Run test to verify it passes:**
```bash
cargo test --lib newport_1830c_v3::tests::test_newport_1830c_v3_initialization
# Expected: PASS
```

**Commit:**
```bash
git add src/instruments_v2/newport_1830c_v3.rs
git commit -m "feat(newport): add V3 skeleton with initialization test"
```

### Step 4: Add PowerMeter trait to core_v3 (if not exists)

**Check if PowerMeter trait exists:**
```bash
grep -n "pub trait PowerMeter" src/core_v3.rs
```

**If not found, add to `src/core_v3.rs`:**
```rust
/// Power meter meta-instrument trait
///
/// V3 Design: Control methods only. Data flows via Instrument::data_channel().
/// Implementations may add convenience methods (e.g., read_power()) that trigger
/// measurements and broadcast via data_channel(), but these are not required by trait.
#[async_trait]
pub trait PowerMeter: Instrument {
    /// Set wavelength calibration in nanometers
    async fn set_wavelength(&mut self, nm: f64) -> Result<()>;
    
    /// Set measurement range in watts
    async fn set_range(&mut self, watts: f64) -> Result<()>;

    /// Zero the power meter (dark calibration)
    async fn zero(&mut self) -> Result<()>;
}
```

**Run tests:**
```bash
cargo test --lib newport_1830c_v3
# Expected: All tests pass
```

**Commit:**
```bash
git add src/core_v3.rs
git commit -m "feat(core): add PowerMeter meta-instrument trait"
```

### Step 5: Write failing test - power measurement

**File:** `src/instruments_v2/newport_1830c_v3.rs`

**Add test:**
```rust
#[tokio::test]
async fn test_newport_1830c_v3_power_reading() {
    let mut power_meter = Newport1830CV3::new("test_pm", "/dev/tty.mock");
    power_meter.initialize().await.unwrap();

    // Read power via PowerMeter trait
    let power = power_meter.read_power().await.unwrap();
    assert!(power >= 0.0); // Power should be non-negative

    // Check that measurement was broadcast
    let mut rx = power_meter.data_channel();
    tokio::select! {
        result = rx.recv() => {
            let measurement = result.unwrap();
            match measurement {
                Measurement::Scalar { value, .. } => {
                    assert!(value >= 0.0);
                }
                _ => panic!("Expected Scalar measurement"),
            }
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
            panic!("No measurement received");
        }
    }
}
```

**Run test to verify it fails:**
```bash
cargo test --lib newport_1830c_v3::tests::test_newport_1830c_v3_power_reading
# Expected: FAIL - no measurement broadcast
```

### Step 6: Implement serial communication and power reading

**File:** `src/instruments_v2/newport_1830c_v3.rs`

**Update initialize() method:**
```rust
async fn initialize(&mut self) -> Result<()> {
    if self.state != InstrumentState::Uninitialized {
        return Err(anyhow!("Already initialized"));
    }

    // Open serial port
    let port = serialport::new(&self.port_name, 9600)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|e| anyhow!("Failed to open {}: {}", self.port_name, e))?;

    self.serial_port = Some(port);

    // Query identification
    self.send_command("*IDN?").await?;
    let id_response = self.read_response().await?;
    log::info!("Newport 1830C '{}' identified: {}", self.id, id_response);

    self.state = InstrumentState::Idle;
    Ok(())
}
```

**Add helper methods:**
```rust
impl Newport1830CV3 {
    /// Send command to power meter
    async fn send_command(&mut self, cmd: &str) -> Result<()> {
        if let Some(port) = &mut self.serial_port {
            let command = format!("{}\r\n", cmd);
            port.write_all(command.as_bytes())?;
            Ok(())
        } else {
            Err(anyhow!("Serial port not open"))
        }
    }

    /// Read response from power meter
    async fn read_response(&mut self) -> Result<String> {
        if let Some(port) = &mut self.serial_port {
            let mut buffer = vec![0; 128];
            let n = port.read(&mut buffer)?;
            let response = String::from_utf8_lossy(&buffer[..n])
                .trim()
                .to_string();
            Ok(response)
        } else {
            Err(anyhow!("Serial port not open"))
        }
    }
}
```

**Implement read_power():**
```rust
async fn read_power(&mut self) -> Result<f64> {
    // Send power query command
    self.send_command("PM:P?").await?;

    // Read response
    let response = self.read_response().await?;

    // Parse power value
    let power: f64 = response
        .trim()
        .parse()
        .map_err(|e| anyhow!("Failed to parse power '{}': {}", response, e))?;

    // Broadcast measurement
    let measurement = Measurement::Scalar {
        name: format!("{}_power", self.id),
        value: power,
        unit: "W".to_string(),
        timestamp: chrono::Utc::now(),
    };
    let _ = self.data_tx.send(measurement);

    Ok(power)
}
```

**Run test to verify it passes:**
```bash
cargo test --lib newport_1830c_v3::tests::test_newport_1830c_v3_power_reading
# Expected: PASS (with mock serial port)
```

**Note:** For testing, you may need to add a mock serial port implementation or use feature flags.

**Commit:**
```bash
git add src/instruments_v2/newport_1830c_v3.rs
git commit -m "feat(newport): implement power reading with serial communication"
```

### Step 7: Add remaining tests

**File:** `src/instruments_v2/newport_1830c_v3.rs`

**Add tests matching PVCAM V3 pattern:**
```rust
#[tokio::test]
async fn test_newport_1830c_v3_wavelength_setting() {
    let mut power_meter = Newport1830CV3::new("test_pm", "/dev/tty.mock");
    power_meter.initialize().await.unwrap();

    power_meter.set_wavelength(633.0).await.unwrap();
    assert_eq!(power_meter.wavelength().await, 633.0);
}

#[tokio::test]
async fn test_newport_1830c_v3_zero_calibration() {
    let mut power_meter = Newport1830CV3::new("test_pm", "/dev/tty.mock");
    power_meter.initialize().await.unwrap();

    // Zero should succeed
    power_meter.zero().await.unwrap();
}

#[tokio::test]
async fn test_newport_1830c_v3_parameter_validation() {
    let mut power_meter = Newport1830CV3::new("test_pm", "/dev/tty.mock");
    power_meter.initialize().await.unwrap();

    // Invalid wavelength should fail
    assert!(power_meter.set_wavelength(100.0).await.is_err()); // Below 200nm

    // Valid wavelength should work
    power_meter.set_wavelength(800.0).await.unwrap();
}

#[tokio::test]
async fn test_newport_1830c_v3_shutdown() {
    let mut power_meter = Newport1830CV3::new("test_pm", "/dev/tty.mock");
    power_meter.initialize().await.unwrap();

    power_meter.shutdown().await.unwrap();
    assert_eq!(power_meter.state(), InstrumentState::ShuttingDown);
}
```

**Run tests:**
```bash
cargo test --lib newport_1830c_v3
# Expected: 6/6 tests passing
```

**Commit:**
```bash
git add src/instruments_v2/newport_1830c_v3.rs
git commit -m "test(newport): add comprehensive test coverage"
```

### Step 8: Add module export

**File:** `src/instruments_v2/mod.rs`

**Add export:**
```rust
pub mod newport_1830c_v3;

pub use newport_1830c_v3::Newport1830CV3;
```

**Verify:**
```bash
cargo check --lib
# Expected: Success, no errors
```

**Commit:**
```bash
git add src/instruments_v2/mod.rs
git commit -m "feat(newport): export Newport1830CV3 from instruments_v2"
```

### Step 9: Create completion document

**File:** `docs/NEWPORT_1830C_V3_COMPLETION.md`

**Content:**
```markdown
# Newport 1830C V3 Implementation - Completion Report

**Date**: 2025-10-25
**Status**: ✅ COMPLETE
**Test Results**: 6/6 tests passing (100%)

## Summary

Successfully migrated Newport 1830C power meter from V2 to V3, validating the PowerMeter meta-instrument trait.

## Key Metrics

- **Lines of Code**: ~400 (V3) vs 487 (V2) = 18% reduction
- **Test Coverage**: 6/6 tests passing
- **Traits Implemented**: `Instrument` + `PowerMeter`

## Lessons Learned

1. Serial protocol is simpler than SDK abstraction
2. PowerMeter trait works well for scalar measurements
3. Pattern from PVCAM V3 transfers cleanly

## Next: ESP300 V3 (Stage)
```

**Commit:**
```bash
git add docs/NEWPORT_1830C_V3_COMPLETION.md
git commit -m "docs(newport): add V3 completion report"
```

---

## Task 2: ESP300 V3 (Stage Controller)

**Estimated Time:** 3-4 hours
**Complexity:** Medium (motion control, positioning)

**Files:**
- Create: `src/instruments_v2/esp300_v3.rs` (~600 lines)
- Modify: `src/instruments_v2/mod.rs` (add export)
- Reference: `src/instruments_v2/esp300.rs` (V2, ~800 lines)

### Step 1: Add Stage trait to core_v3

**File:** `src/core_v3.rs`

**Add trait:**
```rust
/// Stage/motion controller meta-instrument trait
///
/// Instruments that control motorized stages should implement this trait.
#[async_trait]
pub trait Stage: Instrument {
    /// Move to absolute position in mm
    async fn move_absolute(&mut self, position_mm: f64) -> Result<()>;

    /// Move relative to current position in mm
    async fn move_relative(&mut self, distance_mm: f64) -> Result<()>;

    /// Get current position in mm
    async fn position(&self) -> Result<f64>;

    /// Stop motion immediately
    async fn stop_motion(&mut self) -> Result<()>;

    /// Check if stage is moving
    async fn is_moving(&self) -> Result<bool>;

    /// Home the stage (find reference position)
    async fn home(&mut self) -> Result<()>;

    /// Set velocity in mm/s
    async fn set_velocity(&mut self, mm_per_sec: f64) -> Result<()>;
}
```

**Commit:**
```bash
git add src/core_v3.rs
git commit -m "feat(core): add Stage meta-instrument trait"
```

### Step 2: Write failing test - initialization

**File:** `src/instruments_v2/esp300_v3.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_esp300_v3_initialization() {
        let mut stage = ESP300V3::new("test_stage", "/dev/tty.mock");
        assert_eq!(stage.state(), InstrumentState::Uninitialized);

        stage.initialize().await.unwrap();
        assert_eq!(stage.state(), InstrumentState::Idle);
    }
}
```

**Run to verify failure:**
```bash
cargo test --lib esp300_v3::tests::test_esp300_v3_initialization
# Expected: compilation error
```

### Step 3: Create struct skeleton (similar to Newport pattern)

**File:** `src/instruments_v2/esp300_v3.rs`

**Implementation structure:**
```rust
pub struct ESP300V3 {
    id: String,
    state: InstrumentState,
    data_tx: broadcast::Sender<Measurement>,
    parameters: HashMap<String, Box<dyn ParameterBase>>,

    // Serial connection
    port_name: String,
    serial_port: Option<Box<dyn serialport::SerialPort>>,

    // Stage parameters
    velocity_mm_s: Arc<RwLock<Parameter<f64>>>,
    acceleration_mm_s2: Arc<RwLock<Parameter<f64>>>,
    position_mm: Arc<RwLock<Parameter<f64>>>,

    // Motion state
    is_homed: bool,
}

// Implement Instrument + Stage traits
// Follow PVCAM V3 pattern
```

**Commit pattern:** Follow same incremental commit strategy as Newport

### Step 4-9: Follow same pattern as Newport 1830C

- Implement serial communication helpers
- Implement Stage trait methods (move_absolute, position, etc.)
- Add comprehensive tests (6-8 tests)
- Export from mod.rs
- Create completion document

**Expected Duration:** 3-4 hours following established pattern

---

## Task 3: MaiTai V3 (Laser)

**Estimated Time:** 2-3 hours
**Complexity:** Low-Medium (wavelength tuning, power control)

**Pattern:** Follow Newport + ESP300 patterns

**New Trait:**
```rust
#[async_trait]
pub trait Laser: Instrument {
    async fn set_wavelength(&mut self, nm: f64) -> Result<()>;
    async fn wavelength(&self) -> Result<f64>;
    async fn set_power(&mut self, watts: f64) -> Result<()>;
    async fn power(&self) -> Result<f64>;
    async fn enable_shutter(&mut self) -> Result<()>;
    async fn disable_shutter(&mut self) -> Result<()>;
    async fn is_enabled(&self) -> Result<bool>;
}
```

---

## Task 4: Elliptec V3 (Second Stage Implementation)

**Estimated Time:** 2-3 hours
**Complexity:** Medium (validates Stage trait reusability)

**Key Validation:** Demonstrates that Stage trait works for different hardware (Newport Elliptec vs ESP300)

---

## Task 5: VISA/SCPI V3 (Generic Instruments)

**Estimated Time:** 3-4 hours
**Complexity:** Medium-High (generic pattern)

**Pattern:** Generic instrument that doesn't fit specific meta-trait

---

## Phase 2 Completion Checklist

### Per-Instrument Checklist
- [ ] V3 implementation created (~400-600 lines)
- [ ] All trait methods implemented
- [ ] 6+ tests passing (100%)
- [ ] No `block_on` calls in async contexts
- [ ] Proper error handling with Result types
- [ ] Documentation comments on public APIs
- [ ] Exported from `instruments_v2/mod.rs`
- [ ] Completion document created
- [ ] Gemini review performed (optional but recommended)

### Phase 2 Overall
- [ ] Newport 1830C V3 (PowerMeter) ✅
- [ ] ESP300 V3 (Stage)
- [ ] MaiTai V3 (Laser)
- [ ] Elliptec V3 (Stage)
- [ ] VISA/SCPI V3 (Generic)
- [ ] All meta-traits validated
- [ ] Performance benchmarks vs V1/V2
- [ ] Migration guide document
- [ ] Phase 2 completion report

---

## Skills Referenced

- @superpowers:test-driven-development - Follow RED-GREEN-REFACTOR for each method
- @superpowers:testing-anti-patterns - Avoid mocking SDK, test real behavior
- @superpowers:verification-before-completion - Run tests before marking complete
- @superpowers:systematic-debugging - If tests fail, use structured debugging

---

## Estimated Timeline

| Task | Instrument | Duration | Dependencies |
|------|-----------|----------|--------------|
| 1 | Newport 1830C | 2-3h | PowerMeter trait |
| 2 | ESP300 | 3-4h | Stage trait |
| 3 | MaiTai | 2-3h | Laser trait |
| 4 | Elliptec | 2-3h | Stage trait (reuse) |
| 5 | VISA/SCPI | 3-4h | Generic pattern |
| **Total** | **5 instruments** | **12-17h** | Sequential or parallel |

**Parallel Execution:** Tasks 1-3 can be done in parallel (different traits)
**Sequential Execution:** More learning, better pattern refinement

---

## Success Metrics

### Code Quality
- Zero `block_on` calls
- All async patterns correct
- Proper RAII resource management
- 100% test pass rate per instrument

### Performance
- Latency equal or better than V1/V2
- No additional memory overhead
- Clean shutdown in all cases

### Architecture
- Trait polymorphism validated
- Multiple implementations per trait work
- Generic instruments supported
- Pattern scales to future instruments

---

## Next Steps After Phase 2

1. **Phase 3**: Remove actor model (Weeks 5-7)
2. **Benchmarking**: V3 vs V2 performance comparison
3. **Documentation**: Migration guide for contributors
4. **Deprecation**: Mark V1/V2 as deprecated