# Phase 3B Module System Design - Runtime Instrument Assignment

**Issue:** bd-64
**Status:** Design Phase
**Date:** 2025-10-22
**Depends On:** bd-57 (Module Trait - COMPLETED)

## Executive Summary

This document specifies the integration of the Module trait system (completed in bd-57) into the actor-based DAQ architecture. The module system enables dynamic experiment workflows that orchestrate instruments through abstract interfaces, with runtime instrument reassignment achieving <100ms swap times.

**Foundation Complete (bd-57):**
- `Module` trait with lifecycle methods
- `ModuleWithInstrument<M>` trait for type-safe assignment
- `ModuleStatus` state machine
- `ModuleRegistry` factory pattern
- Full documentation and examples

**This Design Adds:**
- Actor integration (module tasks managed by DaqManagerActor)
- DaqCommand message extensions
- Runtime reassignment protocol
- Concrete module implementations (PowerMeterModule proof-of-concept)
- GUI integration specification

**Design Inspiration:** DynExp's three-layer architecture where modules control hardware through abstract instrument interfaces, enabling maximum flexibility in laboratory automation.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Module Actor Integration](#module-actor-integration)
3. [DaqCommand Extensions](#daqcommand-extensions)
4. [Concrete Module Implementations](#concrete-module-implementations)
5. [Runtime Reassignment Protocol](#runtime-reassignment-protocol)
6. [GUI Integration](#gui-integration)
7. [Configuration System](#configuration-system)
8. [Testing Strategy](#testing-strategy)
9. [Migration Path](#migration-path)

---

## Architecture Overview

### Three-Layer Hierarchy (DynExp-Inspired)

```
┌──────────────────────────────────────────────────────────────┐
│                      Module Layer                            │
│   (PowerMeterModule, CameraModule, ScanModule)               │
│                                                              │
│   • High-level experimental logic                           │
│   • Orchestrates multiple instruments via abstract traits   │
│   • Hardware-agnostic workflows                              │
│   • Implements Module + ModuleWithInstrument traits          │
└──────────────────────────────────────────────────────────────┘
                            │
                            │ assign_instrument(Arc<dyn Instrument<Measure = M>>)
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                   Instrument Abstraction                     │
│        (Generic Measure type system)                         │
│                                                              │
│   • Type parameter M: Measure enforces compatibility         │
│   • PowerMeterModule<PowerMeasure> only accepts power meters │
│   • CameraModule<ImageMeasure> only accepts cameras          │
│   • Compile-time safety via generics                         │
└──────────────────────────────────────────────────────────────┘
                            │
                            │ implements Instrument<Measure = M>
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                Concrete Instrument Layer                     │
│   (MockInstrument, Newport1830C, ESP300, VISA)               │
│                                                              │
│   • Existing Instrument trait implementations                │
│   • Each has specific Measure type                           │
│   • No changes required to support modules                   │
└──────────────────────────────────────────────────────────────┘
```

### Actor Integration Data Flow

```
User/GUI
   │
   │ DaqCommand::SpawnModule
   │ DaqCommand::AssignInstrumentToModule
   │ DaqCommand::StartModule
   ▼
┌────────────────────────────────────────────┐
│       DaqManagerActor                      │
│  (centralized state owner)                 │
│                                            │
│  • instruments: HashMap<String, Handle>   │
│  • modules: HashMap<String, ModuleHandle> │
│  • module_registry: Arc<ModuleRegistry>   │
└────────────────────────────────────────────┘
         │                │
         │                │ spawns & manages
         │                │
         ▼                ▼
   Instrument Task    Module Task
   (tokio::spawn)     (tokio::spawn)
         │                │
         │ broadcast      │ subscribes
         │                │
         └────────────────┘
          DataDistributor
```

---

## Module Actor Integration

### Module Task Lifecycle

Each module spawns as a dedicated Tokio task, similar to instruments. The task structure follows the actor pattern:

```rust
/// Module task event loop (spawned by DaqManagerActor)
async fn module_task<M: Measure + 'static>(
    mut module: Box<dyn Module>,
    mut command_rx: mpsc::Receiver<ModuleCommand>,
    data_distributor: Arc<Mutex<DataDistributor<Arc<Measurement>>>>,
) -> Result<()> {
    info!("Module '{}' task started", module.name());

    loop {
        tokio::select! {
            // Process commands from DaqManagerActor
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    ModuleCommand::Shutdown => {
                        info!("Module shutdown command received");
                        break;
                    }
                    ModuleCommand::GetStatus { response } => {
                        let _ = response.send(module.status());
                    }
                }
            }

            // Module-specific periodic work (optional)
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                // Module can implement periodic logic here
                // e.g., check thresholds, update state
            }
        }
    }

    info!("Module '{}' task exiting", module.name());
    Ok(())
}
```

### Module Handle Structure

```rust
/// Handle to a running module task
pub struct ModuleHandle {
    /// Module instance ID
    pub id: String,
    /// Module type (e.g., "power_meter", "camera")
    pub module_type: String,
    /// Tokio task handle
    pub task: JoinHandle<Result<()>>,
    /// Command channel sender
    pub command_tx: mpsc::Sender<ModuleCommand>,
    /// Current status (cached from last query)
    pub status: ModuleStatus,
    /// Assigned instrument IDs (for tracking)
    pub assigned_instruments: Vec<String>,
}
```

### Module Commands

```rust
/// Commands sent to module tasks
pub enum ModuleCommand {
    /// Get current module status
    GetStatus {
        response: oneshot::Sender<ModuleStatus>,
    },
    /// Gracefully shut down the module
    Shutdown,
}
```

### DaqManagerActor Extensions

Add module management to the actor:

```rust
pub struct DaqManagerActor<M>
where
    M: Measure + 'static,
    M::Data: Into<daq_core::DataPoint>,
{
    // ... existing fields ...

    /// Active module tasks (keyed by module ID)
    #[cfg(feature = "modules")]
    modules: HashMap<String, ModuleHandle>,

    /// Module registry for creating module instances
    #[cfg(feature = "modules")]
    module_registry: Arc<ModuleRegistry<M>>,
}
```

---

## DaqCommand Extensions

### New Command Variants

Add to `src/messages.rs`:

```rust
pub enum DaqCommand {
    // ... existing variants ...

    /// Spawns a new module task
    ///
    /// The actor will:
    /// 1. Create module instance from registry
    /// 2. Call module.init(config)
    /// 3. Spawn Tokio task with module event loop
    /// 4. Store ModuleHandle
    ///
    /// # Response
    /// - Ok(module_id): Module spawned successfully
    /// - Err(SpawnError): Invalid config, unknown type, or init failed
    SpawnModule {
        name: String,
        module_type: String,
        config: ModuleConfig,
        response: oneshot::Sender<Result<String, SpawnError>>,
    },

    /// Assigns an instrument to a module at runtime
    ///
    /// Type safety enforced via ModuleWithInstrument<M> trait.
    /// The module validates instrument type and rejects incompatible assignments.
    ///
    /// # Response
    /// - Ok(()): Instrument assigned successfully
    /// - Err: Module not found, instrument not found, or type mismatch
    AssignInstrumentToModule {
        module_id: String,
        instrument_id: String,
        role: String, // e.g., "main_camera", "trigger_source"
        response: oneshot::Sender<Result<()>>,
    },

    /// Unassigns an instrument from a module
    UnassignInstrumentFromModule {
        module_id: String,
        role: String,
        response: oneshot::Sender<Result<()>>,
    },

    /// Starts a module's execution
    ///
    /// Transitions: Initialized → Running
    StartModule {
        id: String,
        response: oneshot::Sender<Result<()>>,
    },

    /// Pauses a running module
    ///
    /// Transitions: Running → Paused
    PauseModule {
        id: String,
        response: oneshot::Sender<Result<()>>,
    },

    /// Stops a module
    ///
    /// Transitions: Running/Paused → Stopped
    StopModule {
        id: String,
        response: oneshot::Sender<Result<()>>,
    },

    /// Queries module status
    GetModuleStatus {
        id: String,
        response: oneshot::Sender<Result<ModuleStatus>>,
    },

    /// Lists all active modules
    GetModuleList {
        response: oneshot::Sender<Vec<(String, String, ModuleStatus)>>,
    },

    /// Shuts down a module task
    ShutdownModule {
        id: String,
        response: oneshot::Sender<()>,
    },
}
```

### Helper Methods

```rust
impl DaqCommand {
    // ... existing helpers ...

    pub fn spawn_module(
        name: String,
        module_type: String,
        config: ModuleConfig,
    ) -> (Self, oneshot::Receiver<Result<String, SpawnError>>) {
        let (tx, rx) = oneshot::channel();
        (Self::SpawnModule { name, module_type, config, response: tx }, rx)
    }

    pub fn assign_instrument_to_module(
        module_id: String,
        instrument_id: String,
        role: String,
    ) -> (Self, oneshot::Receiver<Result<()>>) {
        let (tx, rx) = oneshot::channel();
        (Self::AssignInstrumentToModule { module_id, instrument_id, role, response: tx }, rx)
    }

    pub fn start_module(id: String) -> (Self, oneshot::Receiver<Result<()>>) {
        let (tx, rx) = oneshot::channel();
        (Self::StartModule { id, response: tx }, rx)
    }

    pub fn get_module_status(id: String) -> (Self, oneshot::Receiver<Result<ModuleStatus>>) {
        let (tx, rx) = oneshot::channel();
        (Self::GetModuleStatus { id, response: tx }, rx)
    }

    pub fn get_module_list() -> (Self, oneshot::Receiver<Vec<(String, String, ModuleStatus)>>) {
        let (tx, rx) = oneshot::channel();
        (Self::GetModuleList { response: tx }, rx)
    }
}
```

---

## Concrete Module Implementations

### PowerMeterModule (Proof-of-Concept)

**File:** `src/modules/power_meter.rs`

**Purpose:** Monitor laser power with threshold alerts and statistical analysis.

**Features:**
- Configurable low/high thresholds
- Real-time power monitoring
- Statistical windowing (mean, std dev, min/max)
- Alert generation on threshold violations
- Supports any `Instrument<Measure = PowerMeasure>`

**Implementation:** See full implementation in proof-of-concept section below.

### CameraModule (Planned)

**Purpose:** Control camera acquisition with ROI and exposure management.

**Features:**
- Exposure time and gain control
- Region of interest (ROI) selection
- Automatic dark frame subtraction
- Image processing pipeline integration

### SpectrometerModule (Planned)

**Purpose:** Coordinate spectrometer with synchronized light source control.

**Features:**
- Automatic baseline/dark spectrum acquisition
- Integration time optimization
- Multi-instrument synchronization
- Wavelength calibration

### ScanModule (Planned)

**Purpose:** Orchestrate multi-axis scanning experiments.

**Features:**
- 1D/2D/3D scan patterns
- Snake scan optimization
- Position-triggered data acquisition
- Coordinate transformation

---

## Runtime Reassignment Protocol

### Performance Goal: <100ms

**Design Strategy:**

The reassignment protocol leverages the Module trait's `assign_instrument()` method for type-safe, fast instrument swapping.

### Reassignment Sequence

```rust
async fn assign_instrument_to_module(
    &mut self,
    module_id: &str,
    instrument_id: &str,
    role: &str,
) -> Result<()> {
    // STEP 1: Lookup module (~1μs HashMap access)
    let module_handle = self.modules.get(module_id)
        .ok_or_else(|| anyhow!("Module '{}' not found", module_id))?;

    // STEP 2: Get module as ModuleWithInstrument (~1μs downcast)
    let module_trait = module_handle.module_ref.as_ref()
        .downcast_ref::<dyn ModuleWithInstrument<M>>()
        .ok_or_else(|| anyhow!("Module does not support instrument assignment"))?;

    // STEP 3: Lookup instrument (~1μs HashMap access)
    let instrument_handle = self.instruments.get(instrument_id)
        .ok_or_else(|| anyhow!("Instrument '{}' not found", instrument_id))?;

    // STEP 4: Get Arc reference to instrument (~1ns Arc clone)
    let instrument_arc = Arc::clone(&instrument_handle.instrument_arc);

    // STEP 5: Call module's assign_instrument (~10μs type validation)
    module_trait.assign_instrument(role.to_string(), instrument_arc)?;

    // STEP 6: Update tracking metadata (~1μs)
    module_handle.assigned_instruments.push(instrument_id.to_string());

    Ok(())
}
```

**Estimated Total Time: ~15μs** (well under 100ms target)

### Type Safety Enforcement

**Compile-Time Safety:**

The `ModuleWithInstrument<M>` trait enforces type compatibility via generics:

```rust
// PowerMeterModule only accepts instruments with PowerMeasure
impl ModuleWithInstrument<PowerMeasure> for PowerMeterModule {
    fn assign_instrument(
        &mut self,
        id: String,
        instrument: Arc<dyn Instrument<Measure = PowerMeasure>>,
    ) -> Result<()> {
        // Compiler guarantees instrument.measure() returns PowerMeasure
        self.power_meter = Some(instrument);
        Ok(())
    }
}
```

**Runtime Safety:**

Modules can add additional validation:

```rust
fn assign_instrument(&mut self, id: String, instrument: Arc<...>) -> Result<()> {
    // Reject assignment if module is running
    if self.status() == ModuleStatus::Running {
        return Err(anyhow!("Cannot reassign while module is running"));
    }

    // Module-specific validation
    // ... check capabilities, configuration, etc.

    self.instrument = Some(instrument);
    Ok(())
}
```

### Active Data Stream Handling

**Recommended Strategy: Pause-and-Resume**

```rust
// 1. Pause module (saves state)
module.pause()?;

// 2. Reassign instrument
module.assign_instrument("new_id", new_instrument)?;

// 3. Resume module (restores state with new instrument)
module.start()?;
```

**What happens to data streams?**
- Each instrument has independent broadcast channel
- Module switches data stream subscription
- GUI subscribers unaffected (they subscribe to DataDistributor)
- No data loss if module buffers recent samples

---

## GUI Integration

### Module Control Panel Design

**Layout Wireframe:**

```
┌─────────────────────────────────────────────────────────┐
│  Module Control Panel                            [+New] │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  Active Modules:                                         │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Power Monitor         [● Running]  [⏸]  [⏹]     │  │
│  │  Type: power_meter                                │  │
│  │  Instrument: power_meter_1 → main                 │  │
│  │  Status: Power: 95.2 W (threshold: 50-150 W)     │  │
│  │    └─ [Reassign] [Details] [Configure]           │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Camera Acquisition    [○ Initialized]  [▶]      │  │
│  │  Type: camera                                     │  │
│  │  Instrument: [None assigned]                      │  │
│  │    └─ [Assign] [Configure]                       │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**Status Indicators:**
- ● Running (green)
- ○ Initialized (yellow)
- ⏸ Paused (orange)
- ■ Stopped (gray)
- ⚠ Error (red)

**User Workflows:**

1. **Create Module:** Click [+New] → Select type → Configure → Spawn
2. **Assign Instrument:** Click [Assign] → Select from dropdown → Confirm
3. **Control Lifecycle:** [▶] Start, [⏸] Pause, [⏹] Stop buttons
4. **Reassign:** [Reassign] button stops module → swap → ready to restart

### GUI Component Structure

```rust
// src/gui/module_panel.rs
pub struct ModuleControlPanel {
    cmd_tx: mpsc::Sender<DaqCommand>,
    modules: Vec<(String, String, ModuleStatus)>, // cached list
    show_create_dialog: bool,
    show_assign_dialog: Option<String>, // module_id when dialog open
}

impl ModuleControlPanel {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Module list with controls
        // Dialogs for create/assign/configure
    }
}
```

---

## Configuration System

### TOML Configuration

Modules can be pre-configured in `config/default.toml`:

```toml
[application]
name = "Rust DAQ"

# Instruments
[[instruments.power_meter_1]]
type = "mock"
[instruments.power_meter_1.params]
measurement_type = "scalar"
channel_count = 1

# Modules (new section)
[[modules.power_monitor]]
type = "power_meter"
auto_start = false
[modules.power_monitor.config]
low_threshold = 50.0
high_threshold = 150.0
window_duration_s = 60.0

# Module-to-Instrument Assignments
[[module_assignments.power_monitor]]
role = "main"
instrument = "power_meter_1"
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_power_meter_lifecycle() {
        let mut module = PowerMeterModule::new("test".to_string());

        // Test initialization
        let config = ModuleConfig::new();
        module.init(config).unwrap();
        assert_eq!(module.status(), ModuleStatus::Initialized);
    }

    #[tokio::test]
    async fn test_type_safety() {
        let mut module = PowerMeterModule::new("test".to_string());
        let instrument = Arc::new(MockPowerInstrument::new());

        // Should succeed (correct type)
        module.assign_instrument("main".into(), instrument).unwrap();
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_module_actor_integration() {
    // Spawn module via actor
    // Assign instrument via DaqCommand
    // Verify module receives instrument
    // Test lifecycle transitions
}
```

---

## Migration Path

### Phase 1: Core Infrastructure (COMPLETE - bd-57)
✅ Module trait with lifecycle methods
✅ ModuleWithInstrument trait for assignment
✅ ModuleStatus state machine
✅ ModuleRegistry factory
✅ Documentation and examples

### Phase 2: Actor Integration (THIS DESIGN - bd-64)
- [ ] DaqCommand extensions
- [ ] ModuleHandle and task spawning
- [ ] Runtime assignment protocol
- [ ] PowerMeterModule proof-of-concept

### Phase 3: Additional Modules
- [ ] CameraModule
- [ ] SpectrometerModule
- [ ] ScanModule

### Phase 4: GUI Integration
- [ ] ModuleControlPanel component
- [ ] Create/configure/assign workflows
- [ ] Real-time status display

---

## Proof-of-Concept: PowerMeterModule

See `src/modules/power_meter.rs` (implemented separately).

**Key Features:**
- Demonstrates Module + ModuleWithInstrument traits
- Type-safe power meter assignment
- Threshold monitoring with alerts
- Statistical analysis (mean, std dev, min/max)
- Clean integration with actor system

---

## Open Questions

1. **Module Data Output:**
   Should modules emit data to DataDistributor or have private channels?
   - **Proposed:** Emit to DataDistributor for uniform storage/plotting

2. **Shared Instruments:**
   Can multiple modules share the same instrument?
   - **Current:** Exclusive ownership via Arc
   - **Future:** Reference counting if needed

3. **Module-to-Module Communication:**
   Should modules communicate directly or via DaqManagerActor?
   - **Proposed:** Actor mediates via module events

---

## Conclusion

This design builds on the completed bd-57 Module trait foundation to provide a production-ready module system that:
- Integrates seamlessly with the actor architecture
- Achieves <100ms instrument reassignment
- Enforces type safety via generics
- Provides clear GUI workflows
- Maintains backward compatibility

The PowerMeterModule proof-of-concept demonstrates feasibility and serves as a template for future modules.
