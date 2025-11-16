# Rust DAQ Integration Tests

Comprehensive integration test suite for validating multi-instrument scenarios, concurrency, data flow, and system reliability.

## Test Organization

### Integration Tests (`tests/integration/`)

Multi-file integration tests that validate complex system behavior under realistic conditions.

#### `multi_instrument_test.rs` - Concurrent Instrument Spawning

Tests concurrent instrument lifecycle management:

- **test_spawn_10_instruments_concurrently**: Spawns 10 instruments, measures spawn time distribution, verifies all produce data
- **test_spawn_20_instruments_concurrently**: Stress test with 20 concurrent instruments
- **test_spawn_stop_spawn_cycle**: Tests dynamic instrument lifecycle (spawn 5, stop 3, spawn 3 more)
- **test_no_deadlock_on_concurrent_operations**: Validates no deadlocks during concurrent spawn/stop/data operations

**Purpose**: Validates Phase 1 actor model migration ensures non-blocking operations and prevents deadlocks.

#### `session_roundtrip_test.rs` - Session Persistence

Tests session save/load reliability:

- **test_single_session_roundtrip**: Single iteration of save → stop all → load → verify restoration
- **test_100_iteration_session_roundtrip**: 100 iterations with full validation (bd-61 acceptance criteria)
- **test_session_preserves_storage_format**: Verifies storage settings are correctly restored

**Purpose**: Validates Phase 1 requirement: "Session round-trip test 100/100 iterations" (bd-61).

#### `command_flood_test.rs` - GUI Command Handling

Tests system response to high-frequency command bursts:

- **test_command_flood_single_instrument**: 1000 commands/sec to single instrument, measures latency distribution
- **test_command_flood_multiple_instruments**: 5 instruments receiving 200 cmd/sec each concurrently
- **test_command_bursts_dont_block_data_flow**: Verifies data continues flowing during command bursts
- **test_command_queue_recovery**: Validates system recovers after queue saturation

**Purpose**: Validates Quick Win 4 (graceful command handling) and identifies command channel bottlenecks.

#### `data_flow_test.rs` - Data Distribution Validation

Tests data flow integrity and broadcast channel behavior:

- **test_data_flow_from_10_instruments**: Validates data from 10 instruments reaches subscribers
- **test_detect_broadcast_lag**: Tests broadcast::error::RecvError::Lagged detection (20 instruments, slow receiver)
- **test_multiple_subscribers_receive_same_data**: Verifies broadcast semantics (3 subscribers)
- **test_data_continues_during_instrument_lifecycle**: Data flows during spawn/stop operations
- **test_no_data_loss_under_normal_load**: Validates no lag events with 10 instruments

**Purpose**: Validates Quick Win 3 (configurable channel capacity) and identifies broadcast saturation conditions.

### Root-Level Integration Tests (`tests/*.rs`)

Single-file integration tests for specific features:

- `config_validation_test.rs` - TOML configuration validation
- `fft_processor_integration_test.rs` - FFT processor pipeline
- `integration_test.rs` - Basic mock instrument spawning
- `measurement_enum_test.rs` - V2 Measurement enum serialization
- `storage_factory_test.rs` - Storage writer creation
- `storage_shutdown_test.rs` - Graceful storage writer shutdown
- `v2_adapter_test.rs` - V2InstrumentAdapter behavior
- `validation_test.rs` - Input validation helpers

## Running Tests

### Run All Tests

```bash
cargo test
```

### Run Integration Tests Only

```bash
cargo test --test '*'
```

### Run Specific Test Suite

```bash
# Concurrent spawning tests
cargo test --test multi_instrument_test

# Session round-trip tests (100 iterations)
cargo test --test session_roundtrip_test

# Command flood tests
cargo test --test command_flood_test

# Data flow tests
cargo test --test data_flow_test
```

### Run Specific Test

```bash
cargo test test_100_iteration_session_roundtrip -- --nocapture
```

### Run with Output

```bash
cargo test -- --nocapture
```

## Test Expectations

### Baseline (V1 Architecture with Quick Wins)

After completing bd-59 (Phase 0 Quick Wins):

- **Concurrent Spawning**: 10 instruments spawn successfully, 20 may experience lock contention
- **Session Round-Trip**: 100/100 iterations succeed (blocking)
- **Command Flood**: Avg latency < 10ms, no dropped commands (with retry logic from Quick Win 4)
- **Data Flow**: 10 instruments, no lag (with configurable capacity from Quick Win 3)

### Phase 1 Target (Actor Model)

After completing bd-61 (Actor Model Migration):

- **Concurrent Spawning**: 20+ instruments spawn without contention
- **Session Round-Trip**: 100/100 iterations with non-blocking operations
- **Command Flood**: Avg latency < 5ms, 1000 cmd/sec sustained
- **Data Flow**: 20+ instruments, no lag with default capacity

## Phase 1 Acceptance Criteria Validation

From bd-61, these tests validate:

- ✅ **GUI fps >55 during instrument spawn/stop**: Measured with scripts/benchmark.sh
- ✅ **Integration tests pass with 20 concurrent instruments**: multi_instrument_test.rs
- ✅ **Session round-trip test 100/100 iterations**: session_roundtrip_test.rs::test_100_iteration_session_roundtrip
- ✅ **No mutex poisoning risk**: Code review (no .unwrap() on mutex)
- ✅ **Memory usage within 5% of baseline**: Measured with scripts/benchmark.sh

## Debugging Failed Tests

### Enable Debug Logging

```bash
RUST_LOG=debug cargo test test_name -- --nocapture
```

### Isolate Test with Serial Execution

All integration tests use `#[serial]` from `serial_test` crate to prevent interference.

### Increase Timeouts

Some tests use configurable timeouts. If tests fail on slow systems, increase timeout durations in test code.

### Check Baseline Metrics

Run baseline benchmark before and after changes:

```bash
scripts/benchmark.sh
```

Compare baseline_metrics.json to identify performance regressions.

## Adding New Tests

### Integration Test Template

```rust
use rust_daq::{
    app::DaqApp,
    config::Settings,
    data::registry::ProcessorRegistry,
    instrument::{mock::MockInstrument, InstrumentRegistry},
    log_capture::LogBuffer,
    measurement::Measure,
};
use serial_test::serial;
use std::sync::Arc;

fn create_test_app() -> DaqApp<impl Measure> {
    let settings = Arc::new(Settings::new(None).unwrap());
    let mut instrument_registry = InstrumentRegistry::new();
    instrument_registry.register("mock", |_id| Box::new(MockInstrument::new()));
    let instrument_registry = Arc::new(instrument_registry);
    let processor_registry = Arc::new(ProcessorRegistry::new());
    let log_buffer = LogBuffer::new();

    DaqApp::new(
        settings.clone(),
        instrument_registry,
        processor_registry,
        log_buffer,
    )
    .unwrap()
}

#[test]
#[serial]
fn test_my_scenario() {
    let app = create_test_app();
    let runtime = app.get_runtime();

    runtime.block_on(async {
        // Test logic here
    });

    app.shutdown();
}
```

### Best Practices

1. **Always use `#[serial]`** for integration tests to prevent resource conflicts
2. **Call `app.shutdown()`** at end of test to clean up tokio runtime
3. **Use `tokio::time::timeout`** to prevent hanging tests
4. **Print statistics** with `println!` for debugging (visible with `--nocapture`)
5. **Use `tempfile::NamedTempFile`** for session file tests
6. **Check both success and failure paths** where applicable

## CI Integration

These tests are designed to run in CI with default timeouts. For slower CI runners, consider:

- Reducing iteration counts (e.g., 100 → 50 for session tests)
- Increasing timeout durations
- Running tests in parallel (remove `#[serial]` if resources allow)

## Related Issues

- **bd-59**: Phase 0 Quick Wins (baseline for these tests)
- **bd-60**: Build Integration Test Harness (this work)
- **bd-61**: Phase 1 Actor Model Migration (validated by these tests)
