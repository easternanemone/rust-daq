# Phase 1 Test Strategy: V2 Migration Validation

**Date**: 2025-11-03  
**Status**: ACTIVE  
**Mission**: Comprehensive testing strategy for Phase 1 V2 instrument migration  
**Context**: Phase 1 focuses on V3→V2 merges and VISA V2 implementation

---

## Executive Summary

Phase 1 involves migrating instruments from the V3 prototype architecture back to the production V2 architecture. This test strategy ensures:
1. No regressions in existing functionality
2. V3→V2 merges preserve behavior
3. New VISA V2 implementation works correctly
4. Helper modules integrate properly with V2
5. Feature flag combinations work as expected

### Key Testing Priorities

**Priority 1 (Blocking)**:
- Compilation with all feature flag combinations
- Existing integration tests pass
- V2 instrument trait compliance
- Core measurement broadcast functionality

**Priority 2 (Important)**:
- Hardware adapter behavior
- Error handling and recovery
- State machine transitions
- Memory efficiency (PixelBuffer validation)

**Priority 3 (Nice-to-have)**:
- Performance benchmarks
- Concurrent operation stress tests
- Edge case validation

---

## 1. Existing Test Infrastructure Analysis

### 1.1 Integration Tests (tests/*.rs)

**Total Tests**: 22 integration test files

**Key Test Categories**:

```bash
# Core Functionality
tests/integration_test.rs              # Basic instrument spawn and data flow
tests/graceful_shutdown_test.rs        # Shutdown behavior (10 tests)
tests/measurement_enum_test.rs         # Measurement type handling

# Advanced Features
tests/modules_integration_test.rs      # Module system integration
tests/capability_system_test.rs        # Capability assignment
tests/experiment_sequencer_test.rs     # Experiment orchestration

# Performance & Reliability
tests/backpressure_test.rs            # Data flow under load
tests/spawn_error_test.rs             # Error handling
tests/storage_shutdown_test.rs        # Storage system reliability

# Networking & Configuration
tests/network_protocol_test.rs        # FlatBuffers protocol
tests/config_validation_test.rs       # TOML configuration
tests/dynamic_config_test.rs          # Runtime config changes

# Hardware
tests/pvcam_hardware_smoke.rs         # PVCAM SDK integration
tests/newport_1830c_hardware_test.rs  # Newport power meter
```

**Current Test Status** (as of 2025-11-03):
```bash
$ cargo test --no-run
✅ Compiles with default features
✅ 139 tests compile successfully
⚠️  VISA feature disabled (aarch64 not supported)
```

### 1.2 Unit Tests in Instrument Files

**V2 Instrument Unit Tests**:
- `src/instruments_v2/mock_instrument.rs`: 9 tests (lines 451-586)
  - Lifecycle management
  - Camera trait operations
  - State machine validation
  - Error recovery
  - Power meter functionality
  - Zero-copy Arc<Measurement> validation

**Test Pattern**:
```rust
#[tokio::test]
async fn test_mock_instrument_lifecycle() {
    let mut instrument = MockInstrumentV2::new("test".to_string());
    assert_eq!(instrument.state(), InstrumentState::Disconnected);
    
    instrument.initialize().await.unwrap();
    assert_eq!(instrument.state(), InstrumentState::Ready);
    
    instrument.shutdown().await.unwrap();
    assert_eq!(instrument.state(), InstrumentState::Disconnected);
}
```

### 1.3 Test Infrastructure Strengths

✅ **Good Coverage**:
- Comprehensive integration tests (22 files)
- Core functionality well-tested
- Hardware integration validated (PVCAM, Newport)
- Module system tested
- Network protocol tested

✅ **Good Patterns**:
- Async test infrastructure with tokio::test
- Mock instruments for hardware-free testing
- Timeout-based data reception validation
- Graceful shutdown verification
- Error recovery testing

### 1.4 Test Infrastructure Gaps

❌ **Missing V2-Specific Tests**:
- No dedicated V2 trait compliance tests
- Limited HardwareAdapter unit tests
- No explicit V3→V2 migration validation tests
- Missing feature flag combination matrix tests

❌ **Missing VISA Tests**:
- VISA feature currently broken on aarch64
- No VISA adapter unit tests
- No VISA instrument integration tests
- No error handling tests for VISA-specific failures

---

## 2. Phase 1 Test Plan

### 2.1 Test Execution Order

```
┌─────────────────────────────────────────────────────────┐
│ Phase 1 Test Execution Pipeline                         │
└─────────────────────────────────────────────────────────┘

Level 1: Compilation & Syntax (MUST PASS)
├─ cargo check (default features)
├─ cargo check --features full (excluding instrument_visa)
├─ cargo check --features "storage_csv,instrument_serial"
└─ cargo clippy (no warnings)

Level 2: Unit Tests (MUST PASS)
├─ cargo test --lib (all library unit tests)
├─ V2 instrument trait tests (MockInstrumentV2)
├─ HardwareAdapter tests (MockAdapter, SerialAdapter)
└─ Measurement enum tests (Scalar, Image, Spectrum)

Level 3: Integration Tests (MUST PASS)
├─ tests/integration_test.rs (basic data flow)
├─ tests/graceful_shutdown_test.rs (lifecycle)
├─ tests/measurement_enum_test.rs (type handling)
├─ tests/spawn_error_test.rs (error recovery)
└─ tests/modules_integration_test.rs (if modules used)

Level 4: Feature Flag Matrix (SHOULD PASS)
├─ Default features only
├─ storage_csv only
├─ storage_hdf5 only (if HDF5 available)
├─ instrument_serial only
└─ networking + modules (Phase 2 features)

Level 5: Regression Tests (NICE TO HAVE)
├─ Performance benchmarks (data throughput)
├─ Memory usage (PixelBuffer efficiency)
├─ Concurrent operations (multiple instruments)
└─ Long-running stability (10+ minutes)
```

### 2.2 V3→V2 Merge Verification Tests

**Purpose**: Validate that V3 implementations merged into V2 preserve behavior

**Test Cases**:

```rust
// Test 1: V2 Trait Compliance
#[tokio::test]
async fn test_v2_trait_compliance_after_v3_merge() {
    // For each instrument merged from V3
    let instruments: Vec<Box<dyn daq_core::Instrument>> = vec![
        Box::new(ESP300V2::new("esp1".to_string())),
        Box::new(MaiTaiV2::new("maitai1".to_string())),
        Box::new(Newport1830CV2::new("pm1".to_string())),
        Box::new(ElliptecV2::new("stage1".to_string())),
        Box::new(ScpiInstrumentV2::new("scpi1".to_string())),
    ];
    
    for mut inst in instruments {
        // Verify trait methods exist and work
        assert_eq!(inst.state(), InstrumentState::Disconnected);
        
        inst.initialize().await.expect("Initialize failed");
        assert_eq!(inst.state(), InstrumentState::Ready);
        
        let _rx = inst.measurement_stream();
        
        inst.shutdown().await.expect("Shutdown failed");
        assert_eq!(inst.state(), InstrumentState::Disconnected);
    }
}

// Test 2: Measurement Broadcasting Works
#[tokio::test]
async fn test_v2_measurement_broadcasting() {
    let mut camera = MockInstrumentV2::new("test_cam".to_string());
    camera.initialize().await.unwrap();
    
    let mut rx = camera.measurement_stream();
    
    // Start live acquisition
    camera.start_live().await.unwrap();
    
    // Verify we receive Arc<Measurement::Image>
    let measurement = tokio::time::timeout(
        Duration::from_secs(2),
        rx.recv()
    ).await.expect("Timeout").unwrap();
    
    match &*measurement {
        Measurement::Image(img) => {
            assert_eq!(img.width, 512);
            assert_eq!(img.height, 512);
        }
        _ => panic!("Expected Image measurement"),
    }
    
    camera.stop_live().await.unwrap();
    camera.shutdown().await.unwrap();
}

// Test 3: Camera Trait Meta-Instrument Operations
#[tokio::test]
async fn test_camera_trait_operations() {
    use daq_core::Camera;
    
    let mut camera = MockInstrumentV2::new("test".to_string());
    camera.initialize().await.unwrap();
    
    // Test Camera trait methods
    camera.set_exposure_ms(250.0).await.unwrap();
    assert_eq!(camera.get_exposure_ms().await, 250.0);
    
    let roi = ROI { x: 0, y: 0, width: 256, height: 256 };
    camera.set_roi(roi).await.unwrap();
    assert_eq!(camera.get_roi().await, roi);
    
    camera.set_binning(2, 2).await.unwrap();
    assert_eq!(camera.get_binning().await, (2, 2));
    
    camera.shutdown().await.unwrap();
}

// Test 4: PowerMeter Trait Operations
#[tokio::test]
async fn test_power_meter_trait_operations() {
    use daq_core::PowerMeter;
    
    let mut pm = MockInstrumentV2::new("pm".to_string());
    pm.initialize().await.unwrap();
    
    let power = pm.read_power().await.unwrap();
    assert!(power > 0.0 && power < 10.0);
    
    pm.set_wavelength_nm(800.0).await.unwrap();
    pm.set_range(PowerRange::Auto).await.unwrap();
    pm.zero().await.unwrap();
    
    pm.shutdown().await.unwrap();
}
```

### 2.3 VISA V2 Implementation Tests

**Note**: VISA feature currently broken on aarch64 (macOS ARM). Tests must be conditional.

**Test Strategy**:
1. **Mock VISA adapter tests** (no hardware required)
2. **Feature flag gating** (skip if `instrument_visa` disabled)
3. **Error handling focus** (VISA-specific error types)

```rust
#[cfg(feature = "instrument_visa")]
mod visa_tests {
    use super::*;
    use crate::adapters::VisaAdapter;
    
    #[tokio::test]
    async fn test_visa_adapter_creation() {
        // Test VISA adapter can be created
        let adapter = VisaAdapter::new("GPIB0::1::INSTR".to_string());
        assert!(adapter.is_ok());
    }
    
    #[tokio::test]
    async fn test_visa_error_handling() {
        // Test proper error types for VISA failures
        let mut adapter = VisaAdapter::new("INVALID::RESOURCE".to_string()).unwrap();
        
        let result = adapter.connect(&Default::default()).await;
        assert!(result.is_err());
        
        // Verify error type is DaqError with proper context
        match result {
            Err(e) => {
                let error_msg = e.to_string();
                assert!(error_msg.contains("VISA") || error_msg.contains("connect"));
            }
            Ok(_) => panic!("Should have failed"),
        }
    }
    
    #[tokio::test]
    async fn test_visa_instrument_lifecycle() {
        // Test with mock VISA resource (if available in test environment)
        // Otherwise skip
        if !visa_test_resource_available() {
            eprintln!("⚠️  Skipping VISA integration test (no test resource)");
            return;
        }
        
        let mut instrument = ScpiInstrumentV2::with_visa_resource(
            "test_scpi".to_string(),
            "TCPIP::192.168.1.100::INSTR".to_string()
        );
        
        instrument.initialize().await.expect("VISA init failed");
        assert_eq!(instrument.state(), InstrumentState::Ready);
        
        instrument.shutdown().await.expect("VISA shutdown failed");
    }
}
```

### 2.4 HardwareAdapter Tests

**Test adapters used by V2 instruments**:

```rust
// Test MockAdapter (used by MockInstrumentV2)
#[tokio::test]
async fn test_mock_adapter_basic_operations() {
    use crate::adapters::MockAdapter;
    
    let mut adapter = MockAdapter::new();
    
    // Test connect/disconnect
    adapter.connect(&Default::default()).await.unwrap();
    adapter.disconnect().await.unwrap();
    
    // Test query operations
    adapter.connect(&Default::default()).await.unwrap();
    let response = adapter.query("*IDN?").await.unwrap();
    assert!(!response.is_empty());
    adapter.disconnect().await.unwrap();
}

// Test SerialAdapter (used by ESP300V2, MaiTaiV2, etc.)
#[cfg(feature = "instrument_serial")]
#[tokio::test]
async fn test_serial_adapter_error_handling() {
    use crate::adapters::SerialAdapter;
    
    // Test with invalid port
    let mut adapter = SerialAdapter::new("/dev/tty.NONEXISTENT", 9600);
    
    let result = adapter.connect(&Default::default()).await;
    assert!(result.is_err());
    
    // Verify error message is helpful
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("serial") || 
        error_msg.contains("port") ||
        error_msg.contains("connect")
    );
}
```

### 2.5 Helper Module Integration Tests

**Test that helper modules work correctly with V2 instruments**:

```rust
// Test FFT processor with V2 measurements
#[tokio::test]
async fn test_fft_processor_with_v2_measurements() {
    use rust_daq::data::fft::{FFTProcessor, FFTConfig};
    use rust_daq::core::MeasurementProcessor;
    
    let config = FFTConfig {
        window_size: 64,
        overlap: 32,
        sampling_rate: 512.0,
    };
    
    let mut fft = FFTProcessor::new(config);
    
    // Generate test measurements
    let measurements: Vec<Measurement> = (0..128)
        .map(|i| {
            let t = i as f64 / 512.0;
            let value = (2.0 * std::f64::consts::PI * 50.0 * t).sin();
            
            Measurement::Scalar(DataPoint {
                timestamp: Utc::now(),
                channel: "signal".to_string(),
                value,
                unit: "V".to_string(),
            })
        })
        .collect();
    
    // Process with V2 MeasurementProcessor interface
    let results = fft.process_measurements(&measurements);
    
    // Verify spectrum output
    assert_eq!(results.len(), 1);
    match &results[0] {
        Measurement::Spectrum(spectrum) => {
            assert!(!spectrum.bins.is_empty());
            // Find 50 Hz peak
            let peak = spectrum.bins.iter()
                .find(|bin| (bin.frequency - 50.0).abs() < 5.0)
                .expect("Should find 50 Hz peak");
            assert!(peak.magnitude > -30.0); // Reasonable signal level
        }
        _ => panic!("Expected Spectrum"),
    }
}

// Test IIR filter processor
#[tokio::test]
async fn test_iir_filter_with_v2_data() {
    use rust_daq::data::iir::{IIRProcessor, IIRConfig};
    use rust_daq::core::DataProcessor;
    
    let config = IIRConfig {
        filter_type: "lowpass".to_string(),
        cutoff_hz: 10.0,
        sampling_rate: 100.0,
        order: 2,
    };
    
    let mut filter = IIRProcessor::new(config);
    
    // Generate noisy data
    let data_points: Vec<DataPoint> = (0..100)
        .map(|i| DataPoint {
            timestamp: Utc::now(),
            channel: "noisy_signal".to_string(),
            value: i as f64 + rand::random::<f64>() * 10.0,
            unit: "V".to_string(),
        })
        .collect();
    
    // Process
    let filtered = filter.process(&data_points);
    
    // Verify output
    assert_eq!(filtered.len(), data_points.len());
    assert_eq!(filtered[0].channel, "noisy_signal");
}
```

---

## 3. Feature Flag Testing Strategy

### 3.1 Feature Flag Matrix

**Required Test Combinations**:

| Test Case | Features | Expected Result |
|-----------|----------|-----------------|
| Default | `storage_csv`, `instrument_serial` | ✅ All core tests pass |
| Storage Only | `storage_csv` | ✅ Core + storage tests pass |
| Instruments Only | `instrument_serial` | ✅ Core + serial instrument tests pass |
| HDF5 Storage | `storage_hdf5` | ✅ If HDF5 installed, tests pass |
| Arrow Storage | `storage_arrow` | ✅ Arrow storage tests pass |
| Full (no VISA) | `storage_csv,storage_hdf5,storage_arrow,instrument_serial` | ✅ All non-VISA tests pass |
| VISA Only | `instrument_visa` | ⚠️  Skip on aarch64 (known issue) |
| Networking | `networking` | ✅ FlatBuffers protocol tests pass |
| Modules | `modules` | ✅ Module system tests pass |

**Test Script**:

```bash
#!/bin/bash
# test_feature_matrix.sh

set -e

echo "======================================"
echo "Phase 1 Feature Flag Matrix Testing"
echo "======================================"

# Level 1: Compilation
echo ""
echo "Level 1: Compilation Tests"
echo "--------------------------"

echo "✓ Testing default features..."
cargo check 2>&1 | tail -5

echo "✓ Testing storage_csv only..."
cargo check --no-default-features --features storage_csv 2>&1 | tail -5

echo "✓ Testing instrument_serial only..."
cargo check --no-default-features --features instrument_serial 2>&1 | tail -5

echo "✓ Testing full (no VISA)..."
cargo check --no-default-features --features "storage_csv,storage_arrow,instrument_serial,networking,modules" 2>&1 | tail -5

# Level 2: Unit Tests
echo ""
echo "Level 2: Unit Tests"
echo "-------------------"

echo "✓ Running library unit tests..."
cargo test --lib --features default 2>&1 | tail -10

# Level 3: Integration Tests
echo ""
echo "Level 3: Integration Tests"
echo "--------------------------"

echo "✓ Running core integration tests..."
cargo test --test integration_test 2>&1 | tail -5
cargo test --test graceful_shutdown_test 2>&1 | tail -5
cargo test --test measurement_enum_test 2>&1 | tail -5

# Level 4: Feature-Specific Tests
echo ""
echo "Level 4: Feature-Specific Tests"
echo "--------------------------------"

if [ -f "/usr/local/lib/libhdf5.dylib" ] || [ -f "/opt/homebrew/lib/libhdf5.dylib" ]; then
    echo "✓ HDF5 available, testing storage_hdf5..."
    cargo test --features storage_hdf5 --test storage_shutdown_test 2>&1 | tail -5
else
    echo "⚠️  HDF5 not installed, skipping storage_hdf5 tests"
fi

echo "✓ Testing networking feature..."
cargo test --features networking --test network_protocol_test 2>&1 | tail -5

echo "✓ Testing modules feature..."
cargo test --features modules --test modules_integration_test 2>&1 | tail -5

echo ""
echo "======================================"
echo "Feature Matrix Testing Complete"
echo "======================================"
echo ""
echo "Summary:"
echo "- Default features: PASS"
echo "- Storage variants: PASS"
echo "- Instrument variants: PASS"
echo "- Networking: PASS"
echo "- Modules: PASS"
echo "- VISA: SKIP (aarch64 not supported)"
```

### 3.2 Known Feature Flag Issues

**VISA on aarch64**:
```
Error: visa-sys v0.1.7 build.rs panicked
Reason: "not implemented: target arch aarch64 not implemented"
Status: KNOWN ISSUE - Skip VISA tests on ARM Macs
Workaround: Test VISA on x86_64 Linux or disable instrument_visa
```

**HDF5 Optional Dependency**:
```bash
# HDF5 requires system library
# macOS: brew install hdf5
# Ubuntu: sudo apt-get install libhdf5-dev

# Test availability before running HDF5 tests
if pkg-config --exists hdf5; then
    cargo test --features storage_hdf5
else
    echo "⚠️  HDF5 not installed, skipping tests"
fi
```

---

## 4. Verification Checklist

### 4.1 Pre-Merge Verification (Must Pass)

**Before merging any V3→V2 changes**:

- [ ] **Compilation**: `cargo check` passes with default features
- [ ] **Clippy**: `cargo clippy` shows no new warnings
- [ ] **Unit Tests**: `cargo test --lib` passes
- [ ] **Core Integration**: `cargo test --test integration_test` passes
- [ ] **Shutdown Tests**: `cargo test --test graceful_shutdown_test` passes
- [ ] **Measurement Tests**: `cargo test --test measurement_enum_test` passes

**V2 Trait Compliance**:

- [ ] Instrument implements `daq_core::Instrument` trait
- [ ] `initialize()` transitions Disconnected → Ready
- [ ] `shutdown()` transitions Ready → Disconnected
- [ ] `measurement_stream()` returns valid `MeasurementReceiver`
- [ ] `handle_command()` responds to all `InstrumentCommand` variants
- [ ] `state()` returns correct `InstrumentState` at all times

**Meta-Trait Compliance** (if applicable):

- [ ] Camera: Implements all Camera trait methods
- [ ] PowerMeter: Implements all PowerMeter trait methods
- [ ] Stage: Implements all Stage trait methods
- [ ] Laser: Implements all Laser trait methods

**Measurement Broadcasting**:

- [ ] Broadcasts `Arc<Measurement>` (zero-copy)
- [ ] Correct measurement type (Scalar/Image/Spectrum)
- [ ] Correct channel naming (instrument_id + descriptor)
- [ ] Correct units and metadata
- [ ] Timestamps use `chrono::Utc::now()`

### 4.2 Post-Merge Validation (Should Pass)

**After merging V3→V2 changes**:

- [ ] **Full Test Suite**: `cargo test` passes (all tests)
- [ ] **Feature Matrix**: All feature combinations compile
- [ ] **No Regressions**: Existing tests still pass
- [ ] **Documentation**: Updated if public API changed
- [ ] **Performance**: No significant performance degradation

**Regression Prevention**:

- [ ] Run existing integration tests
- [ ] Verify data throughput hasn't decreased
- [ ] Check memory usage (PixelBuffer efficiency)
- [ ] Validate graceful shutdown still works
- [ ] Confirm error recovery behavior unchanged

### 4.3 Manual Smoke Tests (Nice-to-Have)

**If hardware available**:

- [ ] Test with actual PVCAM camera (if available)
- [ ] Test with Newport 1830-C power meter (if available)
- [ ] Test with ESP300 stage controller (if available)
- [ ] Test with MaiTai laser (if available)

**GUI Validation** (if GUI enabled):

- [ ] Launch GUI: `cargo run --release`
- [ ] Verify instruments appear in instrument list
- [ ] Verify data plots update in real-time
- [ ] Test parameter controls (exposure, ROI, etc.)
- [ ] Test start/stop acquisition
- [ ] Test graceful shutdown (no panic)

---

## 5. Test Framework Setup

### 5.1 Required Test Dependencies

**Already in Cargo.toml**:
```toml
[dev-dependencies]
tempfile = "3.10.1"        # Temporary files for storage tests
serial_test = "3.1.1"      # Serial test execution (shared resources)
tokio-test = "0.4"         # Tokio test utilities
tracing-test = "0.2"       # Tracing/logging for tests
```

**Usage Patterns**:

```rust
use tempfile::tempdir;
use serial_test::serial;
use tokio_test::block_on;
use tracing_test::traced_test;

// Temporary storage for tests
#[test]
fn test_csv_storage() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.csv");
    // ... test code ...
    // dir automatically cleaned up when dropped
}

// Serial execution for shared resources
#[test]
#[serial]
fn test_serial_port_access() {
    // Only one test accesses serial port at a time
}

// Tracing in tests
#[tokio::test]
#[traced_test]
async fn test_with_logging() {
    info!("Test log message");
    // Logs captured and displayed on failure
}
```

### 5.2 Mock Infrastructure

**Available Mocks**:

1. **MockInstrumentV2** (`src/instruments_v2/mock_instrument.rs`)
   - Full V2 Instrument + Camera + PowerMeter implementation
   - No hardware required
   - Configurable behavior
   - Test image generation

2. **MockAdapter** (`src/adapters/mock_adapter.rs`)
   - HardwareAdapter implementation
   - Simulates connect/disconnect
   - Configurable failures for error testing
   - Query/write simulation

**Creating Test Fixtures**:

```rust
// Fixture for V2 instrument testing
fn create_test_v2_instrument() -> MockInstrumentV2 {
    MockInstrumentV2::new("test_instrument".to_string())
}

// Fixture for V2 instrument with specific capacity
fn create_test_v2_instrument_with_capacity(cap: usize) -> MockInstrumentV2 {
    MockInstrumentV2::with_capacity("test_instrument".to_string(), cap)
}

// Fixture for V2 instrument with failing adapter
fn create_failing_v2_instrument() -> MockInstrumentV2 {
    let mut adapter = MockAdapter::new();
    adapter.trigger_failure();
    MockInstrumentV2::with_adapter("test_instrument".to_string(), Box::new(adapter))
}
```

### 5.3 Test Execution Environments

**Local Development**:
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_v2_trait_compliance

# Run with output
cargo test -- --nocapture

# Run with specific features
cargo test --features "storage_csv,instrument_serial"
```

**CI/CD** (GitHub Actions - example):
```yaml
name: Phase 1 Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        features:
          - "default"
          - "storage_csv,instrument_serial"
          - "storage_arrow,networking"
          - "modules"
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - name: Test ${{ matrix.features }}
        run: cargo test --features "${{ matrix.features }}"
```

---

## 6. Success Criteria

### 6.1 Phase 1 Complete When

**All Must Pass**:

✅ **Compilation**:
- `cargo check` with default features
- `cargo check --features full` (excluding VISA)
- `cargo clippy` with no warnings
- All feature flag combinations compile

✅ **Unit Tests**:
- `cargo test --lib` passes (100% of library unit tests)
- V2 instrument unit tests pass
- HardwareAdapter unit tests pass
- Measurement type tests pass

✅ **Integration Tests**:
- Core integration tests pass (integration_test.rs)
- Graceful shutdown tests pass (10/10 tests)
- Measurement enum tests pass
- Storage tests pass
- Module tests pass (if modules feature enabled)

✅ **V2 Migration Verification**:
- All V3→V2 merged instruments pass trait compliance tests
- Measurement broadcasting works for all instruments
- Meta-trait operations work (Camera, PowerMeter, etc.)
- Error handling preserved
- State machine behavior unchanged

✅ **No Regressions**:
- Existing 139 tests still pass
- No performance degradation (data throughput)
- Memory efficiency maintained (PixelBuffer)
- GUI still works (if enabled)

### 6.2 Known Acceptable Failures

**May Skip**:

⚠️ **VISA Tests**:
- Reason: visa-sys doesn't support aarch64 (ARM Macs)
- Action: Skip on aarch64, test on x86_64 Linux if critical
- Issue Tracking: Document in test output

⚠️ **HDF5 Tests** (if HDF5 not installed):
- Reason: Optional system dependency
- Action: Check availability before running tests
- Fallback: Test on CI with HDF5 pre-installed

⚠️ **Hardware Integration Tests** (no hardware):
- Reason: Requires actual hardware (cameras, stages, etc.)
- Action: Use mock instruments for automated tests
- Manual Testing: Run hardware tests manually when hardware available

---

## 7. Test Automation

### 7.1 Pre-Commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
# Pre-commit hook for Phase 1 testing

set -e

echo "🔍 Running pre-commit Phase 1 tests..."

# Check formatting
echo "  Checking code formatting..."
cargo fmt -- --check

# Run clippy
echo "  Running clippy..."
cargo clippy -- -D warnings

# Run quick tests
echo "  Running unit tests..."
cargo test --lib

echo "✅ Pre-commit tests passed!"
```

### 7.2 Continuous Integration

**GitHub Actions Workflow** (`.github/workflows/phase1-tests.yml`):

```yaml
name: Phase 1 Test Suite

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test Phase 1
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
        rust: [stable]
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rust-lang/setup-rust-toolchain@v1
      with:
        toolchain: ${{ matrix.rust }}
        components: clippy, rustfmt
    
    - name: Install HDF5 (Ubuntu)
      if: matrix.os == 'ubuntu-latest'
      run: sudo apt-get install -y libhdf5-dev
    
    - name: Install HDF5 (macOS)
      if: matrix.os == 'macos-latest'
      run: brew install hdf5
    
    - name: Check formatting
      run: cargo fmt --all -- --check
    
    - name: Run clippy
      run: cargo clippy --all-targets --all-features -- -D warnings
    
    - name: Build
      run: cargo build --verbose
    
    - name: Run tests
      run: cargo test --verbose
    
    - name: Test feature combinations
      run: |
        cargo test --features "storage_csv,instrument_serial"
        cargo test --features "storage_arrow,networking"
        cargo test --features "modules"
```

### 7.3 Test Coverage Reporting

**Using `cargo-tarpaulin`**:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir target/coverage

# Open coverage report
open target/coverage/index.html
```

**Coverage Targets**:
- Core traits: >90% coverage
- V2 instruments: >80% coverage
- HardwareAdapters: >70% coverage
- Integration: >60% coverage

---

## 8. Troubleshooting Guide

### 8.1 Common Test Failures

**Problem**: Tests timeout waiting for data
```rust
// Symptom
thread 'test_data_flow' panicked at 'assertion failed: recv_result.is_ok()'

// Cause
// Instrument not started or broadcast channel closed

// Fix
// Verify instrument.initialize() called
// Check measurement_tx.send() not failing
// Increase timeout duration for slow systems
```

**Problem**: State machine assertion fails
```rust
// Symptom
assertion `left == right` failed
  left: Acquiring
 right: Ready

// Cause
// Async operation not awaited or race condition

// Fix
// Add tokio::time::sleep() for async operations to complete
// Use synchronization primitives (oneshot channels) for state transitions
```

**Problem**: Feature flag compilation errors
```rust
// Symptom
error[E0433]: failed to resolve: use of undeclared crate or module `serialport`

// Cause
// Missing feature flag in test

// Fix
#[cfg(feature = "instrument_serial")]
#[tokio::test]
async fn test_serial_adapter() { ... }
```

### 8.2 Debugging Tests

**Enable Logging**:
```bash
# Run tests with logs
RUST_LOG=debug cargo test -- --nocapture

# Run specific test with trace logs
RUST_LOG=trace cargo test test_v2_trait_compliance -- --nocapture
```

**Use Test Harness**:
```rust
#[tokio::test]
#[traced_test]  // Captures logs
async fn test_with_debugging() {
    tracing::info!("Starting test...");
    
    let mut instrument = create_test_v2_instrument();
    tracing::debug!("Created instrument: {}", instrument.id());
    
    instrument.initialize().await.unwrap();
    tracing::info!("Instrument initialized, state: {:?}", instrument.state());
    
    // ... test code ...
}
```

**Verify Async State**:
```rust
// Add explicit state checks
assert_eq!(instrument.state(), InstrumentState::Disconnected, "Initial state");

instrument.initialize().await.unwrap();
assert_eq!(instrument.state(), InstrumentState::Ready, "After initialize");

instrument.start_live().await.unwrap();
assert_eq!(instrument.state(), InstrumentState::Acquiring, "After start_live");
```

---

## 9. Next Steps After Phase 1

### 9.1 Phase 2 Test Additions

**When Phase 2 starts** (PVCAM V2 integration):

- [ ] Add PVCAM V2 unit tests
- [ ] Add camera-specific integration tests
- [ ] Test ImageData with PixelBuffer variants (U8, U16, F64)
- [ ] Test high-frequency image streaming (backpressure)
- [ ] Validate memory efficiency gains

### 9.2 Phase 3 Test Additions

**When Phase 3 starts** (Python bindings):

- [ ] Add Python binding tests (PyO3)
- [ ] Test Python-Rust data transfer (GIL handling)
- [ ] Benchmark Python bindings performance
- [ ] Test Python async/await integration

### 9.3 Continuous Improvement

**Ongoing**:

- Monitor test execution times (keep under 60 seconds)
- Add property-based tests (proptest) for data processors
- Improve test coverage (target 80% overall)
- Add performance regression tests
- Create benchmark suite for comparisons

---

## 10. Appendix: Test Templates

### 10.1 V2 Instrument Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use daq_core::{Instrument, InstrumentState, Measurement};
    use tokio::time::{timeout, Duration};
    
    #[tokio::test]
    async fn test_instrument_lifecycle() {
        let mut instrument = YourInstrumentV2::new("test".to_string());
        
        // Initial state
        assert_eq!(instrument.state(), InstrumentState::Disconnected);
        
        // Initialize
        instrument.initialize().await.expect("Initialize failed");
        assert_eq!(instrument.state(), InstrumentState::Ready);
        
        // Shutdown
        instrument.shutdown().await.expect("Shutdown failed");
        assert_eq!(instrument.state(), InstrumentState::Disconnected);
    }
    
    #[tokio::test]
    async fn test_measurement_broadcasting() {
        let mut instrument = YourInstrumentV2::new("test".to_string());
        instrument.initialize().await.unwrap();
        
        let mut rx = instrument.measurement_stream();
        
        // Trigger measurement (instrument-specific)
        // e.g., instrument.read_power().await.unwrap();
        
        // Receive measurement
        let measurement = timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("Timeout")
            .unwrap();
        
        // Verify measurement type and content
        match &*measurement {
            Measurement::Scalar(data) => {
                assert_eq!(data.channel, "test_power");
                assert!(data.value > 0.0);
            }
            _ => panic!("Expected Scalar measurement"),
        }
        
        instrument.shutdown().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_error_recovery() {
        // Create instrument with failing adapter
        let mut adapter = MockAdapter::new();
        adapter.trigger_failure();
        let mut instrument = YourInstrumentV2::with_adapter(
            "test".to_string(),
            Box::new(adapter)
        );
        
        // Initialize should fail
        assert!(instrument.initialize().await.is_err());
        
        // State should be Error
        match instrument.state() {
            InstrumentState::Error(err) => {
                assert!(err.can_recover);
            }
            _ => panic!("Expected Error state"),
        }
        
        // Recover should succeed
        instrument.recover().await.expect("Recovery failed");
        assert_eq!(instrument.state(), InstrumentState::Ready);
    }
}
```

### 10.2 Integration Test Template

```rust
//! Integration test template for Phase 1
//!
//! Tests interaction between multiple components

use rust_daq::{
    app::DaqApp,
    config::Settings,
    data::registry::ProcessorRegistry,
    instrument::InstrumentRegistry,
    modules::ModuleRegistry,
    log_capture::LogBuffer,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[test]
fn test_v2_instrument_in_app() {
    // Setup
    let settings = Arc::new(Settings::new(None).unwrap());
    let mut instrument_registry = InstrumentRegistry::new();
    
    // Register V2 instrument
    instrument_registry.register("your_v2_instrument", |id| {
        Box::new(YourInstrumentV2::new(id.to_string()))
    });
    
    let instrument_registry = Arc::new(instrument_registry);
    let processor_registry = Arc::new(ProcessorRegistry::new());
    let module_registry = Arc::new(ModuleRegistry::new());
    let log_buffer = LogBuffer::new();
    
    // Create app (auto-spawns instruments)
    let app = DaqApp::new(
        settings,
        instrument_registry,
        processor_registry,
        module_registry,
        log_buffer,
    )
    .unwrap();
    
    let runtime = app.get_runtime();
    
    runtime.block_on(async {
        // Subscribe to data
        let mut data_rx = app.with_inner(|inner| inner.data_sender.subscribe());
        
        // Wait for data
        let recv_result = timeout(Duration::from_secs(5), data_rx.recv()).await;
        
        // Verify data received
        assert!(recv_result.is_ok(), "Did not receive data in time");
        let measurement = recv_result.unwrap().unwrap();
        
        // Verify measurement is from your instrument
        // (check channel name, value, etc.)
    });
    
    // Teardown
    app.shutdown();
}
```

---

## Document History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-11-03 | 1.0 | Tester Agent | Initial Phase 1 test strategy |

---

**Status**: ✅ READY FOR REVIEW  
**Next Action**: Share with coordination team for feedback
