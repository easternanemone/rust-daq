# Rust-DAQ Architectural Analysis: V1/V2/V3 Fragmentation (bd-7e51)

**Date**: 2025-11-03
**Status**: Critical Priority (P0)
**Issues Addressed**: bd-7e51, bd-de55, bd-9f85, bd-f301

## Executive Summary

The rust-daq project is experiencing **critical architectural fragmentation** due to three coexisting and incompatible architectures (V1, V2, V3) being developed simultaneously without a unified migration strategy. This fragmentation is causing:

- **Data loss** through lossy adapter conversions
- **Performance bottlenecks** via blocking operations in async contexts
- **High coupling** and unclear module boundaries
- **Inconsistent error handling** across architectural layers
- **Poor testability** due to entangled state management

**Recommendation**: **HALT all V3 development immediately** until V2 migration is complete. Complete the V2 migration for all instruments, remove compatibility layers, and only then consider V3 enhancements.

## Architecture Overview

### Current State (Three Coexisting Architectures)

```
┌─────────────────────────────────────────────────────────────┐
│                         GUI (egui)                           │
│                     (blocking_send calls)                    │
└─────────────┬───────────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────┐
│                    DaqApp (app.rs)                           │
│              Arc<Runtime> + mpsc channels                    │
│                  ⚠️  blocking_send()                         │
└─────────────┬───────────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────┐
│              DaqManagerActor (app_actor.rs)                  │
│                   [V3 Actor Pattern]                         │
│          Single owner, sequential message processing         │
└─────────────┬───────────────────────────────────────────────┘
              │
              ├───────────────────┬──────────────────┐
              ▼                   ▼                  ▼
┌─────────────────┐   ┌─────────────────┐   ┌──────────────┐
│ V1 Instruments  │   │ V2 Instruments  │   │ V3 Instruments│
│   (legacy)      │   │  (via adapter)  │   │   (native)    │
│                 │   │                 │   │               │
│ src/instrument/ │   │ instruments_v2/ │   │ instruments_v2│
│   - mock.rs     │   │ + v2_adapter.rs │   │   *_v3.rs     │
│   - esp300.rs   │   │                 │   │               │
│   - maitai.rs   │   │ daq-core crate: │   │ Uses V3       │
│   - etc.        │   │ - Measurement   │   │ actor model   │
│                 │   │ - PixelBuffer   │   │               │
│ Broadcasts      │   │                 │   │               │
│ DataPoint via   │   │ Converts to V1  │   │ Native V3     │
│ InstrumentMea-  │   │ DataPoint       │   │ messaging     │
│ surement        │   │ (LOSSY!)        │   │               │
└─────────────────┘   └─────────────────┘   └──────────────┘
```

### Problem 1: V2InstrumentAdapter (bd-de55)

**File**: `src/instrument/v2_adapter.rs`

The V2InstrumentAdapter is a **lossy bottleneck**:

```rust
// V2 instrument produces rich Measurement enum
enum Measurement {
    Scalar(DataPoint),
    Spectrum(SpectrumData),
    Image(ImageData),  // Contains PixelBuffer with full image data
}

// Adapter converts to V1 format, DISCARDING image/spectrum data
// Only broadcasts statistics as DataPoints
pub struct V2InstrumentAdapter {
    measurement: InstrumentMeasurement,  // V1 legacy type
    v2_distributor: Option<DataDistributor>,  // Workaround added later
}
```

**Key Issues**:
1. **Data Loss**: Images and spectra converted to scalar statistics
2. **Broadcast Complexity**: Two parallel broadcast mechanisms
3. **Performance**: blocking_lock() in async context
4. **Memory Inefficiency**: PixelBuffer advantages lost in conversion

**Example of Data Loss**:
```rust
// PVCAM V2 captures 2048x2048 image (8.4 MB with U16)
let image = Measurement::Image(ImageData {
    pixels: PixelBuffer::U16(raw_data),  // Full image
    width: 2048,
    height: 2048,
});

// Adapter converts to:
DataPoint {
    channel: "pvcam_mean",
    value: 1234.5,  // Just the mean! Image LOST
}
```

### Problem 2: DaqApp Blocking Compatibility (bd-7e51, bd-9f85)

**File**: `src/app.rs`

The DaqApp acts as a **thick blocking compatibility layer**:

```rust
// GUI calls this from main thread
pub fn new(...) -> Result<Self> {
    // Spawns actor in background
    runtime_clone.spawn(async move {
        actor.run(command_rx).await;
    });

    // BUT THEN blocks to spawn instruments
    for id in instrument_ids {
        let (cmd, rx) = DaqCommand::spawn_instrument(id.clone());
        if command_tx.blocking_send(cmd).is_ok() {  // ⚠️ BLOCKS GUI THREAD
            if let Ok(result) = rx.blocking_recv() {
                // ...
            }
        }
    }
}
```

**Key Issues**:
1. **GUI Thread Blocking**: `blocking_send()` can stall egui
2. **Negates Async Benefits**: Actor pattern gains lost
3. **Error Propagation**: Errors from async backend may be lost
4. **Testability**: Hard to mock or test in isolation

### Problem 3: Three Competing Instrument APIs

**V1 API** (Legacy - `src/core.rs`):
```rust
#[async_trait]
pub trait Instrument: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    fn data_stream(&self) -> broadcast::Receiver<DataPoint>;  // V1 only
    async fn handle_command(&mut self, command: InstrumentCommand) -> Result<()>;
}
```

**V2 API** (Modern - `crates/daq-core/src/lib.rs`):
```rust
#[async_trait]
pub trait Instrument: Send + Sync {
    async fn initialize(&mut self) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;
    fn measurement_stream(&self) -> MeasurementReceiver;  // Measurement enum
    async fn send_command(&mut self, command: InstrumentCommand) -> Result<()>;
    fn state(&self) -> InstrumentState;  // Explicit state machine
}
```

**V3 API** (Emerging - `src/core_v3.rs`):
```rust
// Actor-based pattern with message passing
// Uses mpsc channels instead of traits
// Each instrument is its own actor
```

**Impact**:
- **17 instrument implementations** split across three APIs
- V1: `src/instrument/` (8 files)
- V2: `src/instruments_v2/` (9 files, including some *_v3.rs)
- V3: Mixed in with V2 (*_v3.rs files)
- Adapter code to bridge between them
- Unclear which API to use for new instruments

## Dependency Analysis

**bd-7e51** (P0) blocks:
- **bd-de55** (P1): Can't remove adapter until V1 instruments migrated
- **bd-9f85** (P1): Can't refactor module boundaries with three APIs
- **bd-f301** (P2): Can't unify error handling across mixed architectures

## Migration Status

### V1 Instruments (Need Migration to V2)
- [ ] `src/instrument/mock.rs` → `src/instruments_v2/mock_instrument.rs` ✓ (exists)
- [ ] `src/instrument/esp300.rs` → `src/instruments_v2/esp300.rs` ✓ (exists)
- [ ] `src/instrument/maitai.rs` → `src/instruments_v2/maitai.rs` ✓ (exists)
- [ ] `src/instrument/newport_1830c.rs` → `src/instruments_v2/newport_1830c.rs` ✓ (exists)
- [ ] `src/instrument/elliptec.rs` → `src/instruments_v2/elliptec.rs` ✓ (exists)
- [ ] `src/instrument/pvcam.rs` → `src/instruments_v2/pvcam.rs` ✓ (exists)
- [ ] `src/instrument/scpi.rs` → `src/instruments_v2/scpi.rs` ✓ (exists)
- [ ] `src/instrument/visa.rs` → Need V2 version

**Status**: **V2 implementations exist for most instruments!**
The problem is they're **not being used** because the app still expects V1 API.

### V3 Instruments (Premature)
- `src/instruments_v2/*_v3.rs` (7 files)
- These were created **before V2 migration was complete**
- Should be **reverted to V2** until migration finished

## Recommended Solution

### Phase 1: HALT V3, Stabilize on V2 (2-3 weeks)

1. **Remove V3 implementations** (revert *_v3.rs to V2)
2. **Update app.rs to use V2 API directly**
   - Remove DaqApp blocking compatibility layer
   - Make app_actor the single entry point
   - Use async message passing throughout
3. **Remove V2InstrumentAdapter**
   - All instruments native V2
   - Single Measurement enum throughout
   - No lossy conversions
4. **Update GUI for async communication**
   - Replace blocking_send with async patterns
   - Use tokio channels properly
5. **Migrate remaining V1 instruments**
   - Update instrument registry to expect V2 trait
   - Wire V2 implementations into app_actor

### Phase 2: Clean Architecture (1-2 weeks)

1. **Establish clear module boundaries**
   - `crates/daq-core`: Core traits and types only
   - `src/instruments_v2`: All instrument implementations
   - `src/app_actor.rs`: Single application actor
   - `src/gui`: GUI components (no business logic)
2. **Remove legacy code**
   - Delete `src/instrument/` (V1 implementations)
   - Delete `src/core.rs` (V1 trait definitions)
   - Delete `src/instrument/v2_adapter.rs`
3. **Unified error handling** (bd-f301)
   - Single `DaqError` type from daq-core
   - Consistent propagation via Result<T>
   - Proper error context with anyhow
4. **Increase test coverage**
   - Actor-based design is highly testable
   - Mock instruments for unit tests
   - Integration tests with real hardware

### Phase 3: Consider V3 Enhancements (Future)

**Only after Phase 1-2 are complete:**
- V3 actor pattern per-instrument (if needed)
- Supervisor hierarchies
- Distributed systems support
- Network protocols

## Immediate Actions (bd-7e51)

### Action 1: Create V2 Migration Checklist
Track the work needed to remove V1 completely.

### Action 2: Remove V3 Files
Revert *_v3.rs files to V2 equivalents to reduce confusion.

### Action 3: Update InstrumentRegistry
Change registry to accept `Box<dyn daq_core::Instrument>` instead of V1 trait.

### Action 4: Refactor app.rs
Remove blocking_send, make everything async-first.

### Action 5: Remove V2InstrumentAdapter
Once registry uses V2 directly, adapter is no longer needed.

## Success Metrics

- [ ] Zero V1 instrument implementations
- [ ] Zero V3 instrument implementations (until Phase 3)
- [ ] Zero adapter/compatibility layers
- [ ] Single Measurement enum used throughout
- [ ] No blocking operations in async contexts
- [ ] 80%+ test coverage
- [ ] Clear module boundaries
- [ ] Consistent error handling

## Risk Assessment

**Risks of NOT fixing this (maintaining status quo)**:
- Continued data loss in V2 instruments
- Performance degradation under load
- Impossible to maintain three APIs
- New developers confused by architecture
- Bugs difficult to diagnose across layers

**Risks of fixing this**:
- 3-5 weeks of focused refactoring
- Temporary instability during migration
- Need to update all instrument configurations
- Potential temporary breakage of some instruments

**Mitigation**:
- Work in feature branch
- Migrate instruments incrementally
- Keep old code until all tests pass
- Extensive integration testing before merge

## Conclusion

The current three-architecture approach is **unsustainable**. The project must:

1. **HALT V3** development immediately
2. **Complete V2** migration for all instruments
3. **Remove all** compatibility layers
4. **Establish clear** architectural boundaries

This is the **only path** to a maintainable, performant, and testable system.

## Related Issues

- **bd-7e51** (P0): This document addresses this issue
- **bd-de55** (P1): V2InstrumentAdapter removal plan in Phase 1
- **bd-9f85** (P1): Module refactoring plan in Phase 2
- **bd-f301** (P2): Error handling unification in Phase 2

## References

- `crates/daq-core/src/lib.rs` - V2 trait definitions
- `src/core.rs` - V1 trait definitions (to be removed)
- `src/app.rs` - Blocking compatibility layer
- `src/app_actor.rs` - V3 actor pattern
- `src/instrument/v2_adapter.rs` - Lossy adapter
- `src/instruments_v2/` - V2 implementations
