# InstrumentRegistry V2 Migration Design

**Issue**: bd-46c9
**Phase**: Phase 2.1 - Core Infrastructure Migration
**Author**: Architect (Hive Mind Swarm)
**Date**: 2025-11-03

## Executive Summary

This document outlines the migration strategy for updating `InstrumentRegistry` from V1 trait-based architecture to V2 native `daq_core::Instrument` trait support. This is a critical infrastructure change that eliminates the temporary `Measure` trait abstraction and enables direct integration of V2 instruments with their native measurement channels.

## Current State Analysis

### V1 Architecture (Current Implementation)

```rust
// src/instrument/mod.rs
type InstrumentFactory<M> = Box<dyn Fn(&str) -> Box<dyn Instrument<Measure = M>> + Send + Sync>;

pub struct InstrumentRegistry<M: Measure> {
    factories: HashMap<String, InstrumentFactory<M>>,
}

impl<M: Measure> InstrumentRegistry<M> {
    pub fn register<F>(&mut self, instrument_type: &str, factory: F)
    where
        F: Fn(&str) -> Box<dyn Instrument<Measure = M>> + Send + Sync + 'static
    {
        self.factories.insert(instrument_type.to_string(), Box::new(factory));
    }

    pub fn create(&self, instrument_type: &str, id: &str) -> Option<Box<dyn Instrument<Measure = M>>>
}
```

**Key Characteristics:**
- Generic over `Measure` trait (temporary abstraction)
- V1 Instrument trait: `trait Instrument { type Measure: Measure; ... }`
- Data flow: `DataPoint` → `InstrumentMeasurement` → `DataDistributor<Arc<DataPoint>>`
- Instruments registered with string keys mapping to factory functions
- Factory signature: `Fn(&str) -> Box<dyn Instrument<Measure = M>>`

### V2 Architecture (Target)

```rust
// crates/daq-core/src/lib.rs
#[async_trait]
pub trait Instrument: Send + Sync {
    fn id(&self) -> &str;
    fn instrument_type(&self) -> &str;
    fn state(&self) -> InstrumentState;
    async fn initialize(&mut self) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;
    fn measurement_stream(&self) -> MeasurementReceiver;
    async fn handle_command(&mut self, cmd: InstrumentCommand) -> Result<()>;
    async fn recover(&mut self) -> Result<()>;
}
```

**Key Characteristics:**
- No generic parameters - single concrete trait
- Native measurement channel: `MeasurementReceiver = broadcast::Receiver<Arc<Measurement>>`
- Data flow: `Measurement` enum → `Arc<Measurement>` → `broadcast::Receiver`
- Explicit state machine with `InstrumentState` enum
- Meta-instrument traits (Camera, PowerMeter, etc.) extend base Instrument

## Breaking Changes Analysis

### 1. Type Signature Changes

**V1:**
```rust
// Generic over Measure trait
InstrumentRegistry<M: Measure>
Box<dyn Instrument<Measure = M>>
```

**V2:**
```rust
// No generics - concrete types only
InstrumentRegistry  // No type parameters
Box<dyn daq_core::Instrument>  // No associated types
```

**Impact:**
- All code generic over `M: Measure` must be specialized or removed
- `DaqApp<M>`, `DaqManagerActor<M>`, `ModuleRegistry<M>` all affected
- `InstrumentMeasurement` type can be removed entirely

### 2. Data Flow Changes

**V1 Flow:**
```
Instrument::Measure::Data (DataPoint)
  ↓
InstrumentMeasurement::broadcast()
  ↓
DataDistributor<Arc<DataPoint>>
  ↓
App converts to Measurement::Scalar
  ↓
DataDistributor<Arc<Measurement>>
```

**V2 Flow:**
```
Instrument produces Measurement directly
  ↓
broadcast::Sender<Arc<Measurement>> (owned by instrument)
  ↓
broadcast::Receiver<Arc<Measurement>> (via measurement_stream())
  ↓
App subscribes and distributes to GUI/Storage
```

**Impact:**
- No intermediate conversion layer needed
- Instruments own their broadcast channels
- App acts as coordinator, not converter
- Native support for Image and Spectrum measurements

### 3. Factory Function Signature

**V1:**
```rust
F: Fn(&str) -> Box<dyn Instrument<Measure = M>> + Send + Sync + 'static
```

**V2:**
```rust
F: Fn(&str) -> Box<dyn daq_core::Instrument> + Send + Sync + 'static
```

**Impact:**
- All factory registrations in main.rs must be updated
- V1 instruments need migration to daq_core::Instrument
- V2 instruments can be registered directly

### 4. Lifecycle Changes

**V1:**
```rust
async fn connect(&mut self, id: &str, settings: &Arc<Settings>) -> Result<()>;
async fn disconnect(&mut self) -> Result<()>;
fn measure(&self) -> &Self::Measure;
```

**V2:**
```rust
async fn initialize(&mut self) -> Result<()>;
async fn shutdown(&mut self) -> Result<()>;
fn measurement_stream(&self) -> MeasurementReceiver;
fn state(&self) -> InstrumentState;
```

**Impact:**
- `connect` → `initialize` (no settings parameter - use config)
- `disconnect` → `shutdown` (consistent naming)
- `measure()` → `measurement_stream()` (pull vs push model)
- Explicit state tracking required

## Migration Strategy

### Phase 1: Parallel V2 Registry (Recommended Approach)

Create a separate V2 registry alongside V1 to enable incremental migration:

```rust
// src/instrument/registry_v2.rs
pub struct InstrumentRegistryV2 {
    factories: HashMap<String, InstrumentFactoryV2>,
}

type InstrumentFactoryV2 = Box<dyn Fn(&str) -> Box<dyn daq_core::Instrument> + Send + Sync>;

impl InstrumentRegistryV2 {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, instrument_type: &str, factory: F)
    where
        F: Fn(&str) -> Box<dyn daq_core::Instrument> + Send + Sync + 'static,
    {
        self.factories.insert(instrument_type.to_string(), Box::new(factory));
    }

    pub fn create(&self, instrument_type: &str, id: &str) -> Option<Box<dyn daq_core::Instrument>> {
        self.factories.get(instrument_type).map(|factory| factory(id))
    }

    pub fn list(&self) -> impl Iterator<Item = String> + '_ {
        self.factories.keys().cloned()
    }
}
```

**Benefits:**
- Zero disruption to existing V1 instruments
- V2 instruments can be tested independently
- Gradual migration path
- Easy rollback if issues arise

**Drawbacks:**
- Temporary code duplication
- Two registries to maintain during migration
- More complex main.rs setup phase

### Phase 2: App Infrastructure Updates

Update `DaqApp` and `DaqManagerActor` to support both registries:

```rust
// src/app.rs
pub struct DaqApp {
    command_tx: mpsc::Sender<DaqCommand>,
    runtime: Arc<Runtime>,
    settings: Settings,
    log_buffer: LogBuffer,
    // V1 registry (to be deprecated)
    instrument_registry_v1: Arc<InstrumentRegistry<InstrumentMeasurement>>,
    // V2 registry (new)
    instrument_registry_v2: Arc<InstrumentRegistryV2>,
}
```

```rust
// src/app_actor.rs
pub struct DaqManagerActor {
    settings: Settings,
    instrument_registry_v1: Arc<InstrumentRegistry<InstrumentMeasurement>>,
    instrument_registry_v2: Arc<InstrumentRegistryV2>,
    // Both V1 and V2 instruments stored in same HashMap
    instruments: HashMap<String, InstrumentHandle>,
    data_distributor: Arc<DataDistributor<Arc<Measurement>>>,
    // ... rest unchanged
}
```

**Spawn logic:**
```rust
async fn spawn_instrument(&mut self, id: String) -> Result<()> {
    let config = self.settings.instruments.get(&id)
        .ok_or_else(|| anyhow!("No config for instrument '{}'", id))?;

    // Try V2 registry first (preferred)
    if let Some(instrument) = self.instrument_registry_v2.create(&config.instrument_type, &id) {
        self.spawn_v2_instrument(id, instrument).await?;
    }
    // Fallback to V1 registry
    else if let Some(instrument) = self.instrument_registry_v1.create(&config.instrument_type, &id) {
        self.spawn_v1_instrument(id, instrument).await?;
    } else {
        return Err(anyhow!("Unknown instrument type: {}", config.instrument_type));
    }

    Ok(())
}
```

### Phase 3: V1 Instrument Migration Path

Two options for existing V1 instruments:

#### Option A: Adapter Pattern (Fast but temporary)
```rust
// src/instrument/v1_to_v2_adapter.rs
pub struct V1ToV2Adapter<M: Measure> {
    inner: Box<dyn crate::core::Instrument<Measure = M>>,
    state: InstrumentState,
    measurement_tx: broadcast::Sender<Arc<Measurement>>,
}

#[async_trait]
impl<M: Measure> daq_core::Instrument for V1ToV2Adapter<M>
where
    M::Data: Into<daq_core::Measurement>,
{
    fn id(&self) -> &str {
        &self.inner.name()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state = InstrumentState::Connecting;
        let settings = Arc::new(Settings::default()); // TODO: get actual settings
        self.inner.connect(&self.id(), &settings).await?;
        self.state = InstrumentState::Ready;
        Ok(())
    }

    fn measurement_stream(&self) -> MeasurementReceiver {
        self.measurement_tx.subscribe()
    }

    // ... implement other methods
}
```

#### Option B: Native Rewrite (Clean but time-consuming)
Rewrite each V1 instrument to implement `daq_core::Instrument` directly.

**Recommendation**: Use Option A for rapid Phase 2 completion, then Option B for Phase 3 cleanup.

### Phase 4: Generic Type Removal

Once all instruments are V2-compatible:

1. Remove `<M: Measure>` from `DaqApp`, `DaqManagerActor`, `ModuleRegistry`
2. Remove `InstrumentMeasurement` type
3. Remove `Measure` trait
4. Remove V1 registry and adapter code
5. Update all type signatures to concrete types

## Implementation Checklist

### Step 1: Create V2 Registry
- [ ] Create `src/instrument/registry_v2.rs`
- [ ] Implement `InstrumentRegistryV2` with HashMap-based storage
- [ ] Add registration and creation methods
- [ ] Write unit tests for registry operations

### Step 2: Update App Infrastructure
- [ ] Add `instrument_registry_v2` field to `DaqApp`
- [ ] Add `instrument_registry_v2` field to `DaqManagerActor`
- [ ] Update `spawn_instrument` to check V2 registry first
- [ ] Create `spawn_v2_instrument` method in DaqManagerActor
- [ ] Update constructor signatures

### Step 3: Update main.rs
- [ ] Create and populate `InstrumentRegistryV2`
- [ ] Register V2 instruments (MockInstrumentV2, PVCAMInstrumentV2, etc.)
- [ ] Pass both registries to DaqApp::new()
- [ ] Add comments indicating V1 instruments are deprecated

### Step 4: Test Dual Registry
- [ ] Verify V1 instruments still work
- [ ] Verify V2 instruments can be spawned
- [ ] Test mixed V1/V2 instrument configurations
- [ ] Verify Image/Spectrum data flows through correctly

### Step 5: Migrate Instruments (Iterative)
For each V1 instrument:
- [ ] Create V1ToV2Adapter wrapper (if using adapter pattern)
- [ ] OR rewrite to implement daq_core::Instrument natively
- [ ] Update registration in main.rs to use V2 registry
- [ ] Test instrument with real hardware (if available)
- [ ] Update documentation

### Step 6: Remove V1 Infrastructure (Final Cleanup)
- [ ] Remove `<M: Measure>` from DaqApp
- [ ] Remove `<M: Measure>` from DaqManagerActor
- [ ] Remove `InstrumentRegistry<M>`
- [ ] Remove `InstrumentMeasurement`
- [ ] Remove `Measure` trait
- [ ] Update all imports and type signatures

## Code Examples

### Before (V1 Registration in main.rs)

```rust
let mut instrument_registry = InstrumentRegistry::<InstrumentMeasurement>::new();
instrument_registry.register("mock", |_id| Box::new(MockInstrument::new()));
instrument_registry.register("maitai", |id| Box::new(MaiTai::new(id)));

let app = DaqApp::<InstrumentMeasurement>::new(
    settings,
    Arc::new(instrument_registry),
    processor_registry,
    module_registry,
    log_buffer,
)?;
```

### After (V2 Registration in main.rs)

```rust
// Create V2 registry
let mut instrument_registry_v2 = InstrumentRegistryV2::new();
instrument_registry_v2.register("mock_v2", |id| {
    Box::new(MockInstrumentV2::new(id.to_string()))
});
instrument_registry_v2.register("pvcam_v2", |id| {
    Box::new(PVCAMInstrumentV2::new(id.to_string()))
});
instrument_registry_v2.register("maitai_v2", |id| {
    Box::new(MaiTaiInstrumentV2::new(id.to_string()))
});

// Keep V1 registry temporarily for migration
let mut instrument_registry_v1 = InstrumentRegistry::<InstrumentMeasurement>::new();
instrument_registry_v1.register("mock", |_id| Box::new(MockInstrument::new()));
// ... other V1 instruments

let app = DaqApp::new(  // No type parameter!
    settings,
    Arc::new(instrument_registry_v1),
    Arc::new(instrument_registry_v2),
    processor_registry,
    module_registry,
    log_buffer,
)?;
```

### Final State (V1 Removed)

```rust
// Only V2 registry remains
let mut instrument_registry = InstrumentRegistryV2::new();
instrument_registry.register("mock", |id| {
    Box::new(MockInstrumentV2::new(id.to_string()))
});
instrument_registry.register("pvcam", |id| {
    Box::new(PVCAMInstrumentV2::new(id.to_string()))
});

let app = DaqApp::new(  // No type parameters!
    settings,
    Arc::new(instrument_registry),
    processor_registry,
    module_registry,
    log_buffer,
)?;
```

## Data Flow Diagrams

### Current V1 Architecture

```
┌─────────────────┐
│ V1 Instrument   │
│ (MockInstrument)│
├─────────────────┤
│ Measure trait   │
│ impl: Instru-   │
│ mentMeasurement │
└────────┬────────┘
         │ DataPoint
         ▼
┌─────────────────┐
│ DataDistributor │
│<Arc<DataPoint>> │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ App converts to │
│ Measurement::   │
│ Scalar(dp)      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ DataDistributor │
│<Arc<Measurement>│
├─────────────────┤
│ → GUI           │
│ → Storage       │
└─────────────────┘
```

### Target V2 Architecture

```
┌─────────────────┐
│ V2 Instrument   │
│ (MockInstrument │
│    V2)          │
├─────────────────┤
│ daq_core::      │
│ Instrument      │
├─────────────────┤
│ broadcast::     │
│ Sender<Arc<     │
│ Measurement>>   │
└────────┬────────┘
         │ Measurement enum
         │ (Scalar/Image/Spectrum)
         ▼
┌─────────────────┐
│ broadcast::     │
│ Receiver<Arc<   │
│ Measurement>>   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ App subscribes  │
│ and distributes │
├─────────────────┤
│ → GUI           │
│ → Storage       │
│ → Processors    │
└─────────────────┘
```

## Risk Assessment

### High Risk
- **Breaking all V1 instruments simultaneously**: Mitigated by parallel registry approach
- **Data loss during migration**: Mitigated by thorough testing and gradual rollout
- **Type signature cascading changes**: Mitigated by phased approach with adapters

### Medium Risk
- **Performance regression from dual registry**: Minimal - registry lookups are infrequent
- **Adapter overhead**: Temporary - removed in Phase 3
- **Configuration compatibility**: Need to document instrument type name changes

### Low Risk
- **GUI compatibility**: GUI already uses `Arc<Measurement>` - no changes needed
- **Storage compatibility**: Storage already uses `Arc<Measurement>` - no changes needed

## Testing Strategy

### Unit Tests
- [ ] Test V2 registry creation and registration
- [ ] Test factory function invocation
- [ ] Test instrument type listing
- [ ] Test error handling for unknown types

### Integration Tests
- [ ] Test V1 instrument spawning with dual registry
- [ ] Test V2 instrument spawning with dual registry
- [ ] Test mixed V1/V2 configuration
- [ ] Test measurement data flow end-to-end

### Hardware Tests
- [ ] Test each V2 instrument with real hardware
- [ ] Verify Image data from PVCAM camera
- [ ] Verify Scalar data from power meters
- [ ] Verify Spectrum data (if applicable)

### Regression Tests
- [ ] All existing V1 functionality preserved
- [ ] No performance degradation
- [ ] Configuration files backward compatible

## Success Criteria

1. ✅ V2 registry successfully creates and manages V2 instruments
2. ✅ V1 instruments continue to work without modification
3. ✅ Image data flows from PVCAM V2 to GUI without conversion
4. ✅ Zero compilation errors in dual-registry phase
5. ✅ All tests pass with both registries active
6. ✅ Clear migration path documented for each V1 instrument

## Dependencies and Blockers

### Prerequisites (Completed)
- ✅ Phase 1: V3 files removed (bd-46c9)
- ✅ Phase 1: V2 instruments stable and tested
- ✅ Phase 1: Codebase compiles without errors

### Parallel Work (Can proceed independently)
- Instrument-specific V2 migrations (bd-84ed for PVCAM, etc.)
- GUI enhancements for Image viewing
- Storage backend optimizations

### Downstream Work (Blocked on this)
- bd-51: Full V2 integration (requires this registry work)
- Generic type removal from app (requires all instruments migrated)

## Timeline Estimate

- **Step 1 (V2 Registry)**: 2-4 hours
- **Step 2 (App Infrastructure)**: 4-6 hours
- **Step 3 (main.rs Updates)**: 1-2 hours
- **Step 4 (Testing)**: 2-4 hours
- **Step 5 (Per-instrument migration)**: 2-4 hours each (iterative)
- **Step 6 (V1 Cleanup)**: 4-8 hours

**Total for dual registry setup**: ~10-16 hours
**Total for complete migration**: 30-50 hours (depends on number of instruments)

## References

- Issue bd-46c9: Phase 2.1 V2 infrastructure migration
- Issue bd-51: Full V2 integration (Phase 3)
- Issue bd-62: V2InstrumentAdapter removal
- File: `crates/daq-core/src/lib.rs` - V2 trait definitions
- File: `src/instrument/mod.rs` - V1 registry implementation
- File: `src/core.rs` - V1 trait definitions
- File: `src/measurement/instrument_measurement.rs` - Measure trait impl

## Appendix A: Type Signature Mapping

| V1 Type | V2 Type | Notes |
|---------|---------|-------|
| `Instrument<Measure = M>` | `daq_core::Instrument` | No associated types |
| `M::Data` | `Measurement` | Enum with Scalar/Image/Spectrum |
| `Arc<DataPoint>` | `Arc<Measurement>` | Unified measurement type |
| `InstrumentMeasurement` | (removed) | No longer needed |
| `Measure trait` | (removed) | Replaced by daq_core::Instrument |
| `connect()` | `initialize()` | Renamed for clarity |
| `disconnect()` | `shutdown()` | Renamed for consistency |
| `measure()` | `measurement_stream()` | Pull → Push model |

## Appendix B: File Impact Analysis

### Files Requiring Changes (Dual Registry Phase)
- `src/instrument/mod.rs` - Add registry_v2 module
- `src/instrument/registry_v2.rs` - **NEW** V2 registry implementation
- `src/app.rs` - Add registry_v2 field
- `src/app_actor.rs` - Add registry_v2 field, update spawn logic
- `src/main.rs` - Register V2 instruments, pass both registries
- `tests/integration_test.rs` - Test dual registry behavior

### Files Requiring Changes (V1 Removal Phase)
- `src/app.rs` - Remove `<M>` generic, remove registry_v1
- `src/app_actor.rs` - Remove `<M>` generic, remove registry_v1
- `src/modules/mod.rs` - Remove `<M>` generic from ModuleRegistry
- `src/measurement/instrument_measurement.rs` - **DELETE**
- `src/measurement/mod.rs` - Remove Measure trait
- `src/core.rs` - Update to reference daq_core types only

### Files Unchanged
- `src/gui/mod.rs` - Already uses `Arc<Measurement>`
- `src/data/storage.rs` - Already uses `Arc<Measurement>`
- `crates/daq-core/src/lib.rs` - V2 trait definitions (stable)
- `src/instruments_v2/*` - V2 instruments (stable)

---

**End of Design Document**

**Next Steps for Coder Worker:**
1. Review this design document
2. Ask clarifying questions if any
3. Implement Step 1: Create V2 Registry
4. Run tests and verify compilation
5. Proceed to Step 2 once Step 1 is validated
