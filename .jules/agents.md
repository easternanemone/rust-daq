# rust-daq Jules Agent Guidelines

## 🏗️ Architecture Overview

**Current State: V5 Headless-First Architecture**
- **Phase 3 COMPLETE**: Network Layer with gRPC server, CLI client, broadcast channels
- **Phase 4 PENDING**: Arrow batching for high-throughput data streaming (PR #104)

### Data Flow Architecture

```
Hardware Drivers → DataPoint (broadcast::channel) → [RingBuffer, gRPC Clients]
                                                           ↓              ↓
                                                   HDF5 Storage    Remote Monitoring
```

## 📁 Key Modules

### Core V5 Components
- **`src/hardware/`** - Capability traits (Movable, Triggerable, Camera, Laser)
- **`src/scripting/`** - Rhai + PyO3 scripting engines with hardware bindings
- **`src/grpc/`** - gRPC server for remote control (requires `networking` feature)
- **`src/measurement_types.rs`** - Shared DataPoint type for measurements
- **`src/data/`** - RingBuffer for zero-copy streaming

### Legacy (DO NOT MODIFY)
- **`src/app/`** - REMOVED (depends on deleted V2 actor pattern)
- **`src/app_actor/`** - REMOVED (V2 architecture deleted in bd-9si6)
- **`src/adapters/`** - REMOVED (V2 instruments)
- **`src/instruments_v2/`** - REMOVED (V2 architecture)

## 🛠️ Development Commands

### Build
```bash
# Library (recommended for development)
cargo build --lib --features networking

# Full binary (daemon + CLI client)
cargo build --bin rust_daq --features networking

# All features
cargo build --all-features
```

### Test
```bash
# Standard test suite (116 tests)
cargo test --lib --features networking

# Specific module
cargo test --lib --features networking scripting::

# With output
cargo test --lib --features networking -- --nocapture
```

### Feature Flags
- **`networking`** - gRPC server, CLI client, broadcast channels (ALWAYS USE)
- **`storage`** - HDF5 storage backend (requires libhdf5)
- **`python_scripting`** - PyO3 Python backend (requires Python dev libs)
- **`instrument_serial`** - Serial hardware drivers (requires libusb)

## 📋 Working on PRs

### Before Starting Work

1. **Check current branch**: `git branch -v`
2. **Rebase on main**: `git fetch origin && git rebase origin/main`
3. **Verify build**: `cargo build --lib --features networking`
4. **Run tests**: `cargo test --lib --features networking`

### PR Guidelines

- **Always use feature flags**: `#[cfg(feature = "networking")]` for networking code
- **Module visibility**: Use `crate::measurement_types::DataPoint` (not `grpc::server::DataPoint`)
- **Test coverage**: Add tests to `tests/` for integration tests
- **Documentation**: Update docs/ if changing architecture

### Common Patterns

**Hardware sends measurements to broadcast:**
```rust
if let Some(tx) = &self.data_tx {
    let data = DataPoint {
        channel: "sensor_name".to_string(),
        value: measured_value,
        timestamp_ns: now_ns(),
    };
    let _ = tx.send(data); // Fire-and-forget
}
```

**gRPC streaming with filtering:**
```rust
async fn stream_measurements(
    &self,
    request: Request<MeasurementRequest>,
) -> Result<Response<Self::StreamMeasurementsStream>, Status> {
    let mut rx = self.data_tx.subscribe();
    let channels = request.into_inner().channels;

    // Filter and stream...
}
```

## 🐛 Debugging Tips

### Compilation Errors

**"cannot find `grpc` in the crate root"**
- Use `crate::measurement_types::DataPoint` instead of `crate::grpc::server::DataPoint`

**"missing field `data_tx` in initializer"**
- Add `data_tx: None` to handle constructors (or `Some(tx)` for networking)

**"feature `networking` is required"**
- Add `#[cfg(feature = "networking")]` before the module/function

### Test Failures

**"test failed with 116 tests passing"**
- This is correct! 116 is the expected count with `networking` feature

**"cannot find type `DataPoint`"**
- Import from `crate::measurement_types`, not `grpc::server`

## 🚀 Phase 4: Arrow Batching (Next Task)

**Goal**: Replace JSON serialization with Apache Arrow for 10-100x throughput

**Key Changes Needed**:
1. `src/data/distributor.rs` - Convert DataPoint stream to Arrow RecordBatch
2. `src/data/ring_buffer.rs` - Write Arrow batches instead of JSON
3. `src/data/hdf5_writer.rs` - Accept Arrow batches directly

**See PR #104** for current Arrow batching work (needs rebase on Phase 3)

## 📚 Important Documentation

- `docs/PHASE_3_NETWORK_LAYER_COMPLETE.md` - Phase 3 architecture and verification
- `docs/CLIENT_USAGE_EXAMPLES.md` - CLI client usage examples
- `docs/V5_OPTIMIZATION_STRATEGIES.md` - Performance optimization guide
- `docs/HARDWARE_DRIVERS_EXAMPLE.md` - Hardware driver reference

## 🔬 Hardware Testing

**Remote Test Machine**: `maitai@100.117.5.12`
- Real hardware: Newport 1830-C stage, PVCAM camera, MaiTai laser
- Test command: `ssh maitai@100.117.5.12 "cd ~/rust-daq && cargo test --features instrument_serial"`

**Mock Hardware**: Use `src/hardware/mock.rs` for local development

## ⚠️ Common Pitfalls

1. **DO NOT** add `networking` feature to `lib.rs` modules - use `#[cfg(feature = "networking")]`
2. **DO NOT** create circular dependencies between `scripting` and `grpc` modules
3. **DO NOT** modify V2 code (it's been removed)
4. **DO NOT** use `Arc<Mutex<T>>` for data streaming - use `broadcast::channel` pattern
5. **ALWAYS** rebase on latest main before creating PR

## 🎯 Current Priorities

1. ✅ Phase 3: Network Layer - **COMPLETE**
2. 🔄 Phase 4: Arrow Batching - **IN PROGRESS** (PR #104)
3. 📋 Hardware Validation - **PENDING** (test on maitai)
4. 📋 Storage Integration - **PENDING** (Arrow → HDF5)

---

**Last Updated**: 2025-11-21 (Phase 3 completion)
**Architecture Version**: V5 Headless-First
**Test Count**: 116 tests passing with `--features networking`
