# V2 Migration Roadmap

**Date**: 2025-11-03
**Parent Issue**: bd-7e51 (Architectural fragmentation)
**Goal**: Complete migration from V1 to V2 architecture, remove all compatibility layers

## Overview

This roadmap provides a **step-by-step plan** to complete the V2 migration, removing the V1 architecture and all compatibility/adapter layers. Each step is designed to be **incremental** and **testable** to minimize risk.

## Current State Assessment

### V1 Instruments (src/instrument/)
- ✅ `mock.rs` - V2 version exists (`instruments_v2/mock_instrument.rs`)
- ✅ `esp300.rs` - V2 version exists (`instruments_v2/esp300.rs`)
- ✅ `maitai.rs` - V2 version exists (`instruments_v2/maitai.rs`)
- ✅ `newport_1830c.rs` - V2 version exists (`instruments_v2/newport_1830c.rs`)
- ✅ `elliptec.rs` - V2 version exists (`instruments_v2/elliptec.rs`)
- ✅ `pvcam.rs` - V2 version exists (`instruments_v2/pvcam.rs`)
- ✅ `scpi.rs` - V2 version exists (`instruments_v2/scpi.rs`)
- ❌ `visa.rs` - No V2 version yet
- ❌ `scpi_common.rs` - Helper code, needs V2 update
- ❌ `serial_helper.rs` - Helper code, needs V2 update

### V3 Files (Premature - Should be V2)
- `instruments_v2/elliptec_v3.rs`
- `instruments_v2/esp300_v3.rs`
- `instruments_v2/maitai_v3.rs`
- `instruments_v2/mock_power_meter_v3.rs`
- `instruments_v2/newport_1830c_v3.rs`
- `instruments_v2/pvcam_v3.rs`
- `instruments_v2/scpi_v3.rs`

### Compatibility Layers (To Remove)
- ✅ `src/instrument/v2_adapter.rs` - V2→V1 adapter (bd-de55)
- ✅ `src/app.rs` - DaqApp blocking compatibility layer
- ✅ `src/core.rs` - V1 trait definitions

## Migration Phases

### Phase 1: Freeze V3, Stabilize V2 (Week 1-2)

#### Step 1.1: Revert V3 Files to V2
**Goal**: Remove confusion, consolidate on V2 API

**Actions**:
1. Review each *_v3.rs file
2. If V3 adds significant features, port them to V2 version
3. Delete *_v3.rs files
4. Update mod.rs to only export V2 versions

**Files to change**:
- `src/instruments_v2/elliptec_v3.rs` → merge to `elliptec.rs`, delete
- `src/instruments_v2/esp300_v3.rs` → merge to `esp300.rs`, delete
- `src/instruments_v2/maitai_v3.rs` → merge to `maitai.rs`, delete
- `src/instruments_v2/mock_power_meter_v3.rs` → merge to `mock_instrument.rs`, delete
- `src/instruments_v2/newport_1830c_v3.rs` → merge to `newport_1830c.rs`, delete
- `src/instruments_v2/pvcam_v3.rs` → merge to `pvcam.rs`, delete
- `src/instruments_v2/scpi_v3.rs` → merge to `scpi.rs`, delete
- `src/instruments_v2/mod.rs` - Update exports

**Testing**:
```bash
# Should still compile (V3 not used yet)
cargo check
cargo test --all-features
```

**Success Criteria**:
- [ ] No *_v3.rs files in instruments_v2/
- [ ] All V2 instruments implement `daq_core::Instrument`
- [ ] No compilation errors
- [ ] Existing tests pass

#### Step 1.2: Create VISA V2 Implementation
**Goal**: Finish V2 implementation for remaining V1 instrument

**Actions**:
1. Create `src/instruments_v2/visa.rs`
2. Implement `daq_core::Instrument` trait
3. Use same VISA-RS bindings as V1
4. Add unit tests

**Files to create**:
- `src/instruments_v2/visa.rs`

**Testing**:
```bash
cargo test instrument::visa --features instrument_visa
```

**Success Criteria**:
- [ ] `visa.rs` implements `daq_core::Instrument`
- [ ] Unit tests for VISA V2 pass
- [ ] Feature flag `instrument_visa` works

#### Step 1.3: Update Helper Modules for V2
**Goal**: Update shared code to work with V2 types

**Actions**:
1. Update `scpi_common.rs` to use `daq_core` types
2. Update `serial_helper.rs` to use `daq_core` types
3. Add `_v2` suffix if V1 versions still needed temporarily

**Files to change**:
- `src/instrument/scpi_common.rs` or create `scpi_common_v2.rs`
- `src/instrument/serial_helper.rs` or create `serial_helper_v2.rs`

**Testing**:
```bash
cargo check --all-features
```

**Success Criteria**:
- [ ] Helper modules compile with V2 types
- [ ] No regression in V2 instruments using helpers

### Phase 2: Update Core Infrastructure (Week 2-3)

#### Step 2.1: Update InstrumentRegistry for V2
**Goal**: Registry accepts V2 instruments directly, no adapter

**Actions**:
1. Change `InstrumentRegistry` to use `daq_core::Instrument` trait
2. Update factory signatures
3. Update spawn logic in app_actor.rs

**Files to change**:
- `src/instrument/mod.rs` - InstrumentRegistry trait bound
- `src/app_actor.rs` - Instrument spawn logic

**Example**:
```rust
// Before (V1)
pub struct InstrumentRegistry<M: Measure> {
    factories: HashMap<String, Box<dyn Fn(&str) -> Box<dyn crate::core::Instrument>>>,
}

// After (V2)
pub struct InstrumentRegistry {
    factories: HashMap<String, Box<dyn Fn(&str) -> Box<dyn daq_core::Instrument>>>,
}
```

**Testing**:
```bash
# Will have compilation errors - expected
cargo check 2>&1 | tee registry_errors.txt
# Fix errors incrementally
```

**Success Criteria**:
- [ ] InstrumentRegistry uses `daq_core::Instrument`
- [ ] No references to V1 `crate::core::Instrument` in registry

#### Step 2.2: Update DaqManagerActor for V2
**Goal**: Actor works directly with V2 Measurement enum

**Actions**:
1. Remove V1 `InstrumentMeasurement` broadcasts
2. Use V2 `Measurement` enum directly
3. Update DataDistributor to handle `Arc<Measurement>`
4. Remove V2InstrumentAdapter usage

**Files to change**:
- `src/app_actor.rs` - Remove adapter, use V2 directly
- `src/measurement/mod.rs` - Update DataDistributor type
- `src/gui/` - Update GUI to receive `Arc<Measurement>`

**Example**:
```rust
// Before (with adapter)
let adapted = V2InstrumentAdapter::new(v2_instrument);
self.instruments.insert(id, Box::new(adapted));

// After (direct V2)
self.instruments.insert(id, Box::new(v2_instrument));
```

**Testing**:
```bash
cargo check
cargo test actor
```

**Success Criteria**:
- [ ] DaqManagerActor uses V2 instruments directly
- [ ] No V2InstrumentAdapter imports
- [ ] DataDistributor handles `Arc<Measurement>`

#### Step 2.3: Update GUI for V2 Measurement Enum
**Goal**: GUI displays all Measurement types (Scalar, Spectrum, Image)

**Actions**:
1. Update plot tabs to receive `Arc<Measurement>`
2. Add pattern matching for Scalar/Spectrum/Image
3. Use ImageTab for Image measurements
4. Use PlotTab for Scalar measurements
5. Add SpectrumTab for Spectrum measurements (if needed)

**Files to change**:
- `src/gui/plot_tab.rs` - Handle `Measurement::Scalar`
- `src/gui/image_tab.rs` - Handle `Measurement::Image`
- `src/gui/tabs.rs` - Route measurements to correct tab
- `src/gui/mod.rs` - Update channel types

**Example**:
```rust
// Receive Arc<Measurement> instead of DataPoint
fn update(&mut self, measurement: Arc<Measurement>) {
    match measurement.as_ref() {
        Measurement::Scalar(dp) => self.plot_scalar(dp),
        Measurement::Image(img) => self.show_image(img),
        Measurement::Spectrum(spec) => self.plot_spectrum(spec),
    }
}
```

**Testing**:
```bash
cargo run --features full
# Verify GUI displays all measurement types
# Test with mock instrument generating each type
```

**Success Criteria**:
- [ ] GUI receives `Arc<Measurement>`
- [ ] Scalar data plots correctly
- [ ] Image data displays in ImageTab
- [ ] Spectrum data plots correctly (if implemented)
- [ ] No data loss from conversions

#### Step 2.4: Remove app.rs Blocking Layer
**Goal**: Remove DaqApp compatibility wrapper, use actor directly

**Actions**:
1. Update main.rs to create DaqManagerActor directly
2. Remove app.rs DaqApp struct
3. Update GUI to send async commands
4. Replace blocking_send with async send or try_send

**Files to change**:
- `src/main.rs` - Create actor directly, remove DaqApp
- `src/app.rs` - Delete or gut this file
- `src/gui/mod.rs` - Use actor's command channel directly

**Example**:
```rust
// Before (blocking)
impl DaqApp {
    pub fn new(...) -> Result<Self> {
        // ... blocking_send calls ...
    }
}

// After (async-first)
fn main() -> Result<()> {
    let runtime = Runtime::new()?;
    runtime.block_on(async {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let actor = DaqManagerActor::new(...)?;
        tokio::spawn(actor.run(cmd_rx));

        // Pass cmd_tx to GUI
        eframe::run_native("DAQ", options, Box::new(|cc| {
            Box::new(DaqGui::new(cc, cmd_tx))
        }))?;
        Ok(())
    })
}
```

**Testing**:
```bash
cargo run
# Verify app starts and instruments spawn
# Test all GUI buttons and controls
# Check for blocking/freezing under load
```

**Success Criteria**:
- [ ] No `blocking_send()` calls in codebase
- [ ] App starts and runs normally
- [ ] GUI remains responsive under load
- [ ] All commands work async

### Phase 3: Remove Legacy Code (Week 3)

#### Step 3.1: Delete V1 Instrument Implementations
**Goal**: Remove all V1 instrument files

**Actions**:
1. Verify all instruments migrated to V2
2. Delete V1 implementations
3. Update imports throughout codebase

**Files to delete**:
- `src/instrument/mock.rs`
- `src/instrument/esp300.rs`
- `src/instrument/maitai.rs`
- `src/instrument/newport_1830c.rs`
- `src/instrument/elliptec.rs`
- `src/instrument/pvcam.rs`
- `src/instrument/scpi.rs`
- `src/instrument/visa.rs` (after V2 version working)

**Testing**:
```bash
# Should still compile without V1 files
cargo check --all-features
cargo test --all-features
```

**Success Criteria**:
- [ ] All V1 instrument files deleted
- [ ] No compilation errors
- [ ] All tests pass

#### Step 3.2: Delete V1 Trait Definitions
**Goal**: Remove V1 core.rs trait definitions

**Actions**:
1. Delete `src/core.rs` (V1 Instrument trait)
2. Update all imports to use `daq_core::Instrument`
3. Remove InstrumentCommand V1 if duplicated

**Files to delete**:
- `src/core.rs`

**Files to update**:
- All files importing `crate::core::Instrument`
- Update to `daq_core::Instrument`

**Testing**:
```bash
cargo check --all-features
cargo clippy -- -D warnings
```

**Success Criteria**:
- [ ] `src/core.rs` deleted
- [ ] No imports of `crate::core::Instrument`
- [ ] Only `daq_core::Instrument` used

#### Step 3.3: Delete V2InstrumentAdapter
**Goal**: Remove bd-de55 bottleneck entirely

**Actions**:
1. Verify no usage of V2InstrumentAdapter
2. Delete the adapter file
3. Remove from mod.rs exports

**Files to delete**:
- `src/instrument/v2_adapter.rs`

**Files to update**:
- `src/instrument/mod.rs` - Remove adapter export

**Testing**:
```bash
cargo check --all-features
grep -r "V2InstrumentAdapter" src/
# Should return no results
```

**Success Criteria**:
- [ ] `v2_adapter.rs` deleted
- [ ] No references to V2InstrumentAdapter
- [ ] No adapter imports anywhere

#### Step 3.4: Delete V1 Measurement Types
**Goal**: Remove InstrumentMeasurement and legacy DataPoint

**Actions**:
1. Verify all code uses `daq_core::Measurement`
2. Delete `measurement/instrument_measurement.rs`
3. Delete legacy `measurement/datapoint.rs` if separate from daq_core

**Files to delete**:
- `src/measurement/instrument_measurement.rs`
- `src/measurement/datapoint.rs` (if not same as daq_core version)

**Files to update**:
- `src/measurement/mod.rs` - Remove legacy exports

**Testing**:
```bash
cargo check --all-features
cargo test --all-features
```

**Success Criteria**:
- [ ] Legacy measurement types deleted
- [ ] Only `daq_core::Measurement` used throughout
- [ ] All tests pass

### Phase 4: Cleanup and Documentation (Week 4)

#### Step 4.1: Clean Module Structure
**Goal**: Establish clear boundaries (bd-9f85)

**Actions**:
1. Organize instrument implementations in `src/instruments_v2/`
2. Keep core traits in `crates/daq-core/`
3. App logic in `src/app_actor.rs` only
4. GUI components in `src/gui/`

**Files to organize**:
- Move any stray instrument code to `instruments_v2/`
- Ensure clear separation of concerns

**Testing**:
```bash
cargo check --all-features
cargo doc --no-deps --open
# Review module organization
```

**Success Criteria**:
- [ ] Clear module boundaries
- [ ] No mixing of concerns
- [ ] Documentation builds cleanly

#### Step 4.2: Unify Error Handling (bd-f301)
**Goal**: Consistent error strategy across codebase

**Actions**:
1. Ensure all errors use `daq_core::DaqError`
2. Use `thiserror` for custom errors
3. Use `anyhow` for error context
4. Add proper error propagation

**Files to update**:
- `crates/daq-core/src/lib.rs` - Define DaqError
- `src/app_actor.rs` - Use DaqError
- `src/instruments_v2/*.rs` - Use DaqError

**Example**:
```rust
#[derive(Error, Debug)]
pub enum DaqError {
    #[error("Instrument '{0}' failed to initialize: {1}")]
    InitializationFailed(String, #[source] anyhow::Error),

    #[error("Communication error with {device}: {msg}")]
    CommunicationError {
        device: String,
        msg: String,
        #[source]
        source: std::io::Error,
    },

    // ... more variants
}
```

**Testing**:
```bash
cargo check
cargo clippy -- -D warnings
```

**Success Criteria**:
- [ ] Single DaqError type
- [ ] Consistent error handling
- [ ] Good error context
- [ ] No unwrap/expect in prod code

#### Step 4.3: Increase Test Coverage
**Goal**: 80%+ coverage, actor pattern is testable

**Actions**:
1. Add unit tests for each instrument
2. Add integration tests for actor messaging
3. Add GUI tests (if possible)
4. Use mock instruments for testing

**Files to create**:
- `tests/actor_test.rs` - Test actor message handling
- `tests/instrument_lifecycle_test.rs` - Test connect/disconnect
- `tests/measurement_flow_test.rs` - Test data flow
- Individual test modules in `instruments_v2/*.rs`

**Testing**:
```bash
cargo test --all-features
cargo tarpaulin --all-features
# Aim for 80%+ coverage
```

**Success Criteria**:
- [ ] 80%+ code coverage
- [ ] All critical paths tested
- [ ] Mock instruments work
- [ ] Integration tests pass

#### Step 4.4: Update Documentation
**Goal**: Document new architecture, migration complete

**Actions**:
1. Update CLAUDE.md with V2 architecture
2. Update AGENTS.md with current state
3. Update README.md
4. Add migration notes
5. Document breaking changes

**Files to update**:
- `CLAUDE.md` - Update architecture section
- `AGENTS.md` - Remove V1 references
- `README.md` - Update examples
- `docs/MIGRATION_V1_TO_V2.md` - Create migration guide
- `CHANGELOG.md` - Document breaking changes

**Success Criteria**:
- [ ] Documentation accurate
- [ ] Examples work
- [ ] Migration guide complete
- [ ] Breaking changes documented

## Rollback Plan

If migration encounters critical issues:

1. **Keep V1 code in git history** - Don't force push
2. **Work in feature branch** - `git checkout -b v2-migration`
3. **Incremental merges** - Merge steps individually
4. **Feature flags** - Use Cargo features to toggle V1/V2
5. **Parallel operation** - Keep V1 working while V2 developed

## Testing Strategy

### Unit Tests
- Test each instrument's V2 implementation
- Mock hardware interfaces
- Test state transitions

### Integration Tests
- Test actor message passing
- Test data flow from instrument → actor → GUI
- Test graceful shutdown

### System Tests
- Run with real hardware (if available)
- Test all instrument types
- Test under load
- Test error recovery

### Regression Tests
- Ensure all previous features still work
- Test with old configuration files
- Verify data compatibility

## Success Criteria (Overall)

- [ ] Zero V1 code in main branch
- [ ] All instruments implement `daq_core::Instrument`
- [ ] No compatibility/adapter layers
- [ ] Single Measurement enum throughout
- [ ] No blocking operations in async code
- [ ] 80%+ test coverage
- [ ] All documentation updated
- [ ] All tests passing
- [ ] Performance equal or better than V1

## Timeline

- **Week 1**: Phase 1 (Freeze V3, Stabilize V2)
- **Week 2**: Phase 2 Steps 2.1-2.2 (Core Infrastructure)
- **Week 3**: Phase 2 Steps 2.3-2.4 + Phase 3 (Remove Legacy)
- **Week 4**: Phase 4 (Cleanup and Documentation)

**Total**: 4 weeks for complete migration

## Resources Needed

- 1-2 developers full-time
- Access to test hardware (if available)
- CI/CD pipeline for testing
- Review from maintainers

## Risks and Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking existing workflows | High | Feature flags, parallel operation |
| Hardware not available | Medium | Use mock instruments, emulators |
| Performance regression | High | Benchmark before/after |
| Incomplete testing | High | Aim for 80%+ coverage |
| Scope creep | Medium | Strict phase boundaries |

## Next Steps

1. Review this roadmap with team
2. Create bd issues for each phase
3. Set up feature branch
4. Begin Phase 1.1 (Revert V3 files)

## Related Issues

- **bd-7e51**: Parent issue (architectural fragmentation)
- **bd-de55**: V2InstrumentAdapter removal (Phase 3.3)
- **bd-9f85**: Module boundaries (Phase 4.1)
- **bd-f301**: Error handling (Phase 4.2)
