# V5 Reactive Parameter System - Critical Path

**Date**: 2025-12-02
**Analysis**: clink/gemini confirmation of architectural fragmentation
**Status**: CRITICAL - Split Brain Architecture Blocking All Features

## Executive Summary

**CONFIRMED**: rust-daq V5 has a "Split Brain" architecture where three independent state systems exist without integration:
- **Brain A (Hardware)**: `Arc<RwLock<T>>` - opaque, silent
- **Brain B (High-Level)**: `Parameter<T>` - rich, reactive, **UNUSED**
- **Brain C (Modules)**: `Observable<T>` - lightweight notifications

**Result**: gRPC clients can't see hardware changes, presets can't snapshot state, experiments lack metadata.

## Root Cause

Drivers were **never migrated** to use the reactive parameter system. The V5 architecture was built but V4 patterns (raw locks) were never removed.

## Critical Path (Dependency-Ordered)

### Phase 1: Foundation (P0 - BLOCKING EVERYTHING)

**Epic**: bd-gcjl - V5 Reactive Parameter System Integration

#### 1.1 Unify Reactive Primitives
**Issue**: bd-si19 (P0)
**Title**: CRITICAL: Unify Parameter and Observable into single reactive primitive

**What**: Refactor `Parameter<T>` to **compose** `Observable<T>` with strict hierarchy:
```rust
pub struct Parameter<T> {
    inner: Observable<T>,  // Base primitive (watch, subscriptions, validation)
    hw_writer: Option<Box<dyn Fn(T) -> Result<()>>>,  // Hardware write callback
    hw_reader: Option<Box<dyn Fn() -> Result<T>>>,    // Hardware read callback
}

impl<T> Parameter<T> {
    pub async fn set(&mut self, value: T) -> Result<()> {
        if let Some(writer) = &self.hw_writer {
            writer(value.clone())?;  // 1. Execute hardware command
        }
        self.inner.set(value)?;  // 2. Broadcast to subscribers
        Ok(())
    }

    pub fn subscribe(&self) -> watch::Receiver<T> {
        self.inner.subscribe()  // Delegate to Observable
    }
}
```

**Why Critical**: Eliminates code duplication. Ensures `Parameter` updates trigger `Observable` subscribers. Creates single source of truth.

**Blocks**: ALL other work

---

#### 1.2 Add Central Parameter Registry
**Issue**: bd-9clg (P0)
**Title**: Add Parameterized trait for central parameter registry

**What**: Missing from mature frameworks - the "Parameter Tree":
```rust
pub trait Parameterized {
    fn parameters(&self) -> &ParameterSet;
}

// All drivers implement this
impl Parameterized for MockCamera {
    fn parameters(&self) -> &ParameterSet {
        &self.params
    }
}

// DeviceRegistry stores parameters
pub struct RegisteredDevice {
    config: DeviceConfig,
    movable: Option<Arc<dyn Movable>>,
    // ... other capabilities ...
    parameters: Option<ParameterSet>,  // NEW
}
```

**Why Critical**: Without this, generic code (gRPC, HDF5 writers, presets) can't enumerate or access parameters. list_parameters RPC literally can't work.

**Depends On**: bd-si19 (need unified Parameter<T> first)
**Blocks**: bd-2s41, bd-dili

---

### Phase 2: Driver Migration (P0 - THE SMOKING GUN)

#### 2.1 Migrate Mock Drivers
**Issue**: bd-dili (P0)
**Title**: Migrate all hardware drivers to unified Parameter system

**What**: Replace **all** raw state with `Parameter<T>`:
```rust
// BEFORE (current - BROKEN)
pub struct MockCamera {
    exposure_s: Arc<RwLock<f64>>,  // ← Opaque, silent
}

impl ExposureControl for MockCamera {
    async fn set_exposure(&self, seconds: f64) -> Result<()> {
        *self.exposure_s.write().await = seconds;
        // ← NO ONE KNOWS THIS CHANGED!
        Ok(())
    }
}

// AFTER (target - FUNCTIONAL)
pub struct MockCamera {
    exposure_s: Parameter<f64>,  // ← Reactive, observable
    params: ParameterSet,
}

impl Parameterized for MockCamera {
    fn parameters(&self) -> &ParameterSet { &self.params }
}

impl ExposureControl for MockCamera {
    async fn set_exposure(&self, seconds: f64) -> Result<()> {
        self.exposure_s.set(seconds).await?;
        // ← Automatically:
        //    1. Writes to hardware (via callback)
        //    2. Notifies all subscribers (gRPC, modules, logger)
        //    3. Updates parameter registry
        Ok(())
    }
}

impl MockCamera {
    pub fn new(width: u32, height: u32) -> (Self, ParameterSet) {
        let mut params = ParameterSet::new();

        let exposure = Parameter::new("exposure_s", 0.1)
            .with_unit("s")
            .with_range(0.001, 10.0);
        params.register(exposure.clone());

        (Self { exposure_s: exposure, params: params.clone() }, params)
    }
}
```

**Why Critical**: "The smoking gun" - until this is done, gRPC API **lies to clients** about system state. Hardware changes are invisible.

**Drivers to Migrate**:
1. MockCamera (pilot)
2. MockStage (pilot)
3. PVCAM
4. ELL14
5. ESP300
6. MaiTai
7. Newport1830C

**Depends On**: bd-si19, bd-9clg
**Blocks**: bd-zafg

---

### Phase 3: Integration (P1 - CONNECT THE PIECES)

#### 3.1 Delete bd-f5yh Manual Streaming
**Issue**: bd-2s41 (P1)
**Title**: DELETE bd-f5yh manual parameter streaming (replace with Parameter subscribe)

**What**: **PURGE** the manual broadcast channel added in bd-f5yh:
```rust
// DELETE THIS (bd-f5yh implementation - WRONG):
impl HardwareServiceImpl {
    param_change_tx: broadcast::Sender<ParameterChange>,  // ← DELETE
}

async fn set_parameter(...) {
    settable.set_value(...)?;
    self.param_change_tx.send(ParameterChange { ... })?;  // ← DELETE
}

// REPLACE WITH (correct integration):
async fn stream_parameter_changes(...) {
    let params = registry.get_parameters(&device_id)?;

    for param in params.iter() {
        let mut rx = param.subscribe();  // ← Use existing mechanism!
        tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                tx.send(ParameterChange::from(rx.borrow())).await?;
            }
        });
    }
}
```

**Why**: bd-f5yh duplicates `Parameter<T>.subscribe()` functionality. It's a band-aid for drivers not using parameters. Once drivers are migrated, natural propagation works automatically.

**Depends On**: bd-si19, bd-gajr
**Blocks**: None (cleanup)

---

#### 3.2 Hardware Change Propagation
**Issue**: bd-zafg (P1)
**Title**: Implement hardware change notification propagation

**What**: Ensure background tasks (polling, interrupts) update parameters:
```rust
impl MockCamera {
    async fn start_temperature_monitoring(&self) {
        let temp_param = self.temperature.clone();
        tokio::spawn(async move {
            loop {
                match read_sensor_temp().await {
                    Ok(temp) => {
                        temp_param.set(temp).await?;  // ← Broadcasts automatically!
                    }
                    Err(e) => error!("Temp read failed: {}", e),
                }
                sleep(Duration::from_secs(1)).await;
            }
        });
    }
}
```

**Why**: Hardware-initiated changes (not just RPC calls) must propagate. Temperature sensors, position readbacks, status flags need real-time updates.

**Depends On**: bd-dili
**Blocks**: None

---

### Phase 4: System Features (P2 - ENABLED BY FOUNDATION)

#### 4.1 ParameterRegistry in DeviceRegistry
**Issue**: bd-gajr (P1/P2)
**Title**: Create centralized ParameterRegistry for cross-layer parameter access

**Status**: Foundation for Phase 2. Enables:
- `list_parameters` RPC
- Preset snapshot/restore
- Experiment manifest generation
- Module access to hardware parameters

---

#### 4.2 Ring Buffer Tapping
**Issue**: bd-dqic (P2)
**Title**: Implement ring buffer tap mechanism for live data visualization

**What**: Secondary consumers for ring buffer data without disrupting HDF5 writer. Enables headless with remote preview.

---

#### 4.3 System Health Monitoring
**Issue**: bd-pauy (P2)
**Title**: Add system health monitoring for headless operation

**What**: Prevents "silent failure" in headless mode. Heartbeat monitoring, error propagation, `GetSystemHealth` RPC.

---

#### 4.4 Experiment Manifests
**Issue**: bd-ej44 (P2)
**Title**: Implement automatic experiment manifest injection to HDF5

**What**: Auto-snapshot all parameters to HDF5 metadata at experiment start. Required for reproducibility.

---

## Dependency Graph

```
Epic: bd-gcjl
├─ bd-si19 (P0) Unify Parameter/Observable [NO DEPS - START HERE]
├─ bd-9clg (P0) Parameterized trait
│  └─ depends: bd-si19
├─ bd-dili (P0) Migrate drivers
│  └─ depends: bd-si19, bd-9clg
├─ bd-2s41 (P1) DELETE bd-f5yh
│  └─ depends: bd-si19, bd-gajr
├─ bd-zafg (P1) Hardware change propagation
│  └─ depends: bd-dili
└─ [P2 issues: bd-gajr, bd-dqic, bd-pauy, bd-ej44]
```

## Success Criteria

The V5 architecture is "complete" when:
- ✅ All driver state is `Parameter<T>`-based (no raw `Arc<RwLock>`)
- ✅ gRPC `list_parameters` returns actual hardware parameters
- ✅ gRPC `stream_parameter_changes` receives hardware-initiated updates
- ✅ Modules can observe driver parameter changes via `ParameterSet`
- ✅ Presets can snapshot and restore entire system state
- ✅ HDF5 files contain complete parameter manifests
- ✅ No manual broadcast channels exist (natural propagation only)

## What NOT to Do (Anti-Patterns)

❌ **DON'T**: Try to "refactor" bd-f5yh incrementally
✅ **DO**: Delete it completely after foundation is ready

❌ **DON'T**: Add more manual notification code
✅ **DO**: Let `Parameter<T>.set()` handle all notifications

❌ **DON'T**: Create parameters as local variables
✅ **DO**: Store them in `ParameterSet` and expose via `Parameterized` trait

❌ **DON'T**: Use `Arc<RwLock<T>>` for any hardware state
✅ **DO**: Use `Parameter<T>` for all mutable state

## Timeline Estimate

- **Week 1**: bd-si19 (Unify primitives) + bd-9clg (Parameterized trait)
- **Week 2**: bd-dili Phase 1 (MockCamera, MockStage migration)
- **Week 3**: bd-dili Phase 2 (Real drivers: PVCAM, ELL14, etc.)
- **Week 4**: bd-2s41 (Delete bd-f5yh), bd-zafg (Hardware propagation)
- **Week 5+**: P2 features (health monitoring, tapping, manifests)

## References

- **clink Analysis**: Confirmed "Split Brain" architecture, recommended Observable<T> composition
- **V5_INTEGRATION_ANALYSIS.md**: Comprehensive assessment of fragmentation
- **PyMoDAQ**: LoggedQuantity pattern (Parameter<T> equivalent)
- **ScopeFoundry**: Central parameter tree (Parameterized trait equivalent)
- **Qudi**: Logic/Hardware separation (Module system)

## Notes

- bd-o5n9: Closed as obsolete (replaced by bd-2s41)
- bd-f5yh: Original parameter streaming work - to be completely removed
- All P0 issues block critical business features (presets, remote sync, reproducibility)
