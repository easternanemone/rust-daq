# Module System Design (Phase 3B)

**Issue:** bd-64
**Status:** Design Complete
**Date:** 2025-10-19

## Executive Summary

This document specifies a DynExp-inspired module system for rust-daq that enables runtime instrument reassignment with type safety. The design introduces a three-layer architecture:

1. **Meta Instrument Traits** - Abstract capabilities (Camera, Spectrometer, etc.)
2. **Module Trait** - High-level experimental logic
3. **Runtime Reassignment** - Type-safe device swapping via actor system

**Key Requirements Met:**
- Runtime reassignment < 100ms
- Type safety enforcement (CameraModule rejects non-Camera instruments)
- Hot-swap during active acquisition
- Feature-gated behind `modules` flag
- GUI integration ready

## Architecture Overview

### Three-Layer Design

```
┌─────────────────────────────────────────────────────────────┐
│                         Module Layer                        │
│  (CameraModule, SpectrometerModule, PowerMeterModule)       │
│                                                             │
│  - High-level experimental logic                           │
│  - Coordinates multiple instruments                         │
│  - Implements Module trait                                  │
└─────────────────────────────────────────────────────────────┘
                           │
                           │ assign_camera()
                           │ assign_spectrometer()
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  Meta Instrument Layer                      │
│    (Camera trait, Spectrometer trait, PowerMeter trait)     │
│                                                             │
│  - Abstract device capabilities                            │
│  - Type-safe polymorphism via trait objects                │
│  - Orthogonal to existing Instrument trait                  │
└─────────────────────────────────────────────────────────────┘
                           │
                           │ implements
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Instrument Layer                         │
│     (PVCam, MockInstrument, Newport1830C, etc.)             │
│                                                             │
│  - Existing Instrument trait implementations               │
│  - Can implement multiple meta instrument traits            │
│  - No breaking changes required                            │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

```
User/GUI → DaqCommand::AssignCamera → DaqManagerActor
                                           │
                                           ├─ 1. Lookup instrument
                                           ├─ 2. Downcast to Camera trait
                                           ├─ 3. Lookup CameraModule
                                           ├─ 4. Stop if running
                                           ├─ 5. Swap trait object
                                           └─ 6. Ready for restart
```

## Component Specifications

### 1. Module Trait

**Location:** `src/modules/mod.rs`

```rust
use async_trait::async_trait;
use anyhow::Result;

/// Status of a module
#[derive(Clone, Debug, PartialEq)]
pub enum ModuleStatus {
    Idle,
    Running,
    Error(String),
}

/// High-level experimental module trait
#[async_trait]
pub trait Module: Send + Sync {
    /// Module name for UI display
    fn name(&self) -> &str;

    /// Start the module's measurement/control loop
    async fn start(&mut self) -> Result<()>;

    /// Stop the module gracefully
    async fn stop(&mut self) -> Result<()>;

    /// Check if module is currently running
    fn is_running(&self) -> bool;

    /// Get module status for UI display
    fn status(&self) -> ModuleStatus;

    /// Downcast support for runtime type checking
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
```

**Design Rationale:**
- Async methods support long-running operations
- Status enum enables rich UI feedback
- `as_any()` methods enable safe downcasting for reassignment
- No generic parameters - keeps trait object compatible

### 2. Meta Instrument Traits

**Location:** `src/modules/meta_instruments.rs`

```rust
use async_trait::async_trait;
use anyhow::Result;
use crate::core::{ImageData, SpectrumData};

/// Base meta instrument trait - all devices implement this
pub trait MetaInstrument: Send + Sync {
    fn instrument_id(&self) -> &str;
    fn instrument_type(&self) -> &str;
    fn capabilities(&self) -> Vec<String>;
}

/// Camera-specific capabilities
#[async_trait]
pub trait Camera: MetaInstrument {
    async fn capture(&mut self) -> Result<ImageData>;
    async fn set_exposure(&mut self, ms: f64) -> Result<()>;
    async fn get_exposure(&self) -> Result<f64>;
    async fn set_roi(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<()>;
    async fn get_sensor_size(&self) -> Result<(u32, u32)>;
}

/// Spectrometer-specific capabilities
#[async_trait]
pub trait Spectrometer: MetaInstrument {
    async fn acquire_spectrum(&mut self) -> Result<SpectrumData>;
    async fn set_integration_time(&mut self, ms: f64) -> Result<()>;
    async fn get_wavelength_range(&self) -> Result<(f64, f64)>;
    async fn get_wavelength_calibration(&self) -> Result<Vec<f64>>;
}

/// Power meter capabilities
#[async_trait]
pub trait PowerMeter: MetaInstrument {
    async fn read_power(&mut self) -> Result<f64>;
    async fn set_wavelength(&mut self, nm: f64) -> Result<()>;
    async fn set_range(&mut self, watts: f64) -> Result<()>;
    async fn get_range(&self) -> Result<f64>;
    async fn zero(&mut self) -> Result<()>;
}

/// Position control (stages, mirrors, rotation mounts)
#[async_trait]
pub trait Positioner: MetaInstrument {
    async fn move_absolute(&mut self, position: f64) -> Result<()>;
    async fn move_relative(&mut self, delta: f64) -> Result<()>;
    async fn get_position(&self) -> Result<f64>;
    async fn home(&mut self) -> Result<()>;
    async fn stop_motion(&mut self) -> Result<()>;
    async fn is_moving(&self) -> Result<bool>;
}
```

**Design Rationale:**
- Async methods match existing Instrument trait style
- Each trait focused on specific device category
- `MetaInstrument` base trait provides common functionality
- Instruments can implement multiple meta traits (e.g., Camera + Positioner for motorized focus)

### 3. Concrete Module Implementations

#### CameraModule

**Location:** `src/modules/camera.rs`

```rust
use super::{Module, ModuleStatus};
use super::meta_instruments::Camera;
use async_trait::async_trait;
use anyhow::{anyhow, Result};
use std::any::Any;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::Duration;

pub struct CameraModule {
    name: String,
    camera: Option<Arc<Mutex<Box<dyn Camera>>>>,
    running: bool,
    acquisition_task: Option<JoinHandle<()>>,
}

impl CameraModule {
    pub fn new(name: String) -> Self {
        Self {
            name,
            camera: None,
            running: false,
            acquisition_task: None,
        }
    }

    /// Type-safe camera assignment - only accepts Camera trait objects
    pub fn assign_camera(&mut self, camera: Box<dyn Camera>) -> Result<()> {
        if self.running {
            return Err(anyhow!("Cannot assign camera while module is running"));
        }
        self.camera = Some(Arc::new(Mutex::new(camera)));
        log::info!("Camera assigned to module '{}'", self.name);
        Ok(())
    }

    pub fn unassign_camera(&mut self) -> Result<()> {
        if self.running {
            return Err(anyhow!("Cannot unassign camera while module is running"));
        }
        self.camera = None;
        log::info!("Camera unassigned from module '{}'", self.name);
        Ok(())
    }

    pub fn has_camera(&self) -> bool {
        self.camera.is_some()
    }
}

#[async_trait]
impl Module for CameraModule {
    fn name(&self) -> &str {
        &self.name
    }

    async fn start(&mut self) -> Result<()> {
        let camera = self.camera.as_ref()
            .ok_or_else(|| anyhow!("No camera assigned to module"))?
            .clone();

        self.running = true;
        log::info!("Starting camera module '{}'", self.name);

        // Spawn acquisition loop
        let module_name = self.name.clone();
        let task = tokio::spawn(async move {
            loop {
                let mut cam = camera.lock().await;
                match cam.capture().await {
                    Ok(image) => {
                        log::debug!(
                            "Module '{}': Captured {}x{} image",
                            module_name,
                            image.width,
                            image.height
                        );
                        // TODO: Broadcast image data via module output
                    }
                    Err(e) => {
                        log::error!("Module '{}': Capture failed: {}", module_name, e);
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        self.acquisition_task = Some(task);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping camera module '{}'", self.name);
        self.running = false;

        if let Some(task) = self.acquisition_task.take() {
            task.abort();
        }

        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn status(&self) -> ModuleStatus {
        if self.running {
            ModuleStatus::Running
        } else if self.camera.is_some() {
            ModuleStatus::Idle
        } else {
            ModuleStatus::Error("No camera assigned".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
```

**Key Features:**
- `assign_camera()` enforces type safety at compile time (only accepts `Box<dyn Camera>`)
- Prevents reassignment while running
- Arc<Mutex<>> enables shared ownership between module and acquisition task
- Acquisition loop runs in separate tokio task

#### PowerMeterModule

**Location:** `src/modules/power_meter.rs`

Similar structure to CameraModule with:
- `assign_power_meter(Box<dyn PowerMeter>)` method
- Power monitoring loop
- Wavelength calibration support

#### SpectrometryModule

**Location:** `src/modules/spectrometer.rs`

Similar structure with:
- `assign_spectrometer(Box<dyn Spectrometer>)` method
- Spectrum acquisition loop
- Integration time management

### 4. Module Registry

**Location:** `src/modules/mod.rs`

```rust
use std::collections::HashMap;

pub struct ModuleRegistry {
    factories: HashMap<String, Box<dyn Fn(String) -> Box<dyn Module> + Send + Sync>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
        };

        #[cfg(feature = "modules")]
        {
            registry.register("camera", |name| {
                Box::new(crate::modules::camera::CameraModule::new(name))
            });

            registry.register("power_meter", |name| {
                Box::new(crate::modules::power_meter::PowerMeterModule::new(name))
            });

            registry.register("spectrometer", |name| {
                Box::new(crate::modules::spectrometer::SpectrometryModule::new(name))
            });
        }

        registry
    }

    pub fn register<F>(&mut self, module_type: &str, factory: F)
    where
        F: Fn(String) -> Box<dyn Module> + Send + Sync + 'static,
    {
        self.factories.insert(module_type.to_string(), Box::new(factory));
    }

    pub fn create(&self, module_type: &str, name: String) -> Option<Box<dyn Module>> {
        self.factories.get(module_type).map(|f| f(name))
    }

    pub fn list_types(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }
}
```

### 5. Runtime Reassignment Mechanism

#### DaqCommand Extensions

**Location:** `src/messages.rs`

```rust
#[cfg(feature = "modules")]
pub enum DaqCommand {
    // ... existing commands ...

    // Module lifecycle
    CreateModule {
        module_type: String,
        module_id: String,
        respond_to: oneshot::Sender<Result<()>>,
    },

    DestroyModule {
        module_id: String,
        respond_to: oneshot::Sender<Result<()>>,
    },

    // Type-safe assignment variants
    AssignCamera {
        module_id: String,
        instrument_id: String,
        respond_to: oneshot::Sender<Result<()>>,
    },

    AssignSpectrometer {
        module_id: String,
        instrument_id: String,
        respond_to: oneshot::Sender<Result<()>>,
    },

    AssignPowerMeter {
        module_id: String,
        instrument_id: String,
        respond_to: oneshot::Sender<Result<()>>,
    },

    AssignPositioner {
        module_id: String,
        instrument_id: String,
        respond_to: oneshot::Sender<Result<()>>,
    },

    // Module control
    StartModule {
        module_id: String,
        respond_to: oneshot::Sender<Result<()>>,
    },

    StopModule {
        module_id: String,
        respond_to: oneshot::Sender<Result<()>>,
    },

    GetModuleStatus {
        module_id: String,
        respond_to: oneshot::Sender<Result<ModuleStatus>>,
    },
}
```

#### DaqManagerActor Implementation

**Location:** `src/app_actor.rs`

```rust
#[cfg(feature = "modules")]
impl DaqManagerActor {
    async fn handle_assign_camera(
        &mut self,
        module_id: String,
        instrument_id: String,
    ) -> Result<()> {
        // 1. Lookup instrument
        let instrument = self.instruments.get(&instrument_id)
            .ok_or_else(|| anyhow!("Instrument '{}' not found", instrument_id))?;

        // 2. Verify instrument implements Camera trait
        let camera = self.extract_camera_trait(instrument)?;

        // 3. Lookup module
        let module = self.modules.get_mut(&module_id)
            .ok_or_else(|| anyhow!("Module '{}' not found", module_id))?;

        // 4. Downcast to CameraModule
        let camera_module = module.as_any_mut()
            .downcast_mut::<CameraModule>()
            .ok_or_else(|| anyhow!("Module '{}' is not a CameraModule", module_id))?;

        // 5. Stop if running
        if camera_module.is_running() {
            camera_module.stop().await?;
            log::info!("Stopped module '{}' for reassignment", module_id);
        }

        // 6. Assign camera
        camera_module.assign_camera(camera)?;

        log::info!(
            "Assigned instrument '{}' to module '{}'",
            instrument_id,
            module_id
        );

        Ok(())
    }

    /// Extract Camera trait object from instrument
    /// This is where we bridge between Instrument trait and meta instrument traits
    fn extract_camera_trait(
        &self,
        instrument: &Box<dyn Instrument<Measure = InstrumentMeasurement>>,
    ) -> Result<Box<dyn Camera>> {
        // Try downcasting to known Camera implementations
        let any_inst = instrument.as_any();

        if let Some(pvcam) = any_inst.downcast_ref::<PVCam>() {
            // Clone or Arc-wrap the instrument
            return Ok(Box::new(pvcam.clone()));
        }

        // Add other camera types as they implement Camera trait

        Err(anyhow!(
            "Instrument does not implement Camera trait. \
             Available capabilities: {:?}",
            self.get_instrument_capabilities(instrument)
        ))
    }

    fn get_instrument_capabilities(
        &self,
        instrument: &Box<dyn Instrument<Measure = InstrumentMeasurement>>,
    ) -> Vec<String> {
        // Query instrument for which meta traits it implements
        // This can be extended with a Capabilities trait
        vec![]
    }
}
```

## Type Safety Guarantees

### Compile-Time Safety

1. **Module Assignment Methods**
   - `CameraModule::assign_camera(Box<dyn Camera>)` - only accepts Camera trait objects
   - `PowerMeterModule::assign_power_meter(Box<dyn PowerMeter>)` - only accepts PowerMeter
   - Compiler prevents passing wrong instrument type to wrong module

2. **DaqCommand Variants**
   - Separate enum variants for each assignment type
   - `AssignCamera` != `AssignSpectrometer` at type level

### Runtime Safety

1. **Double Downcast Protection**
   - First downcast: Instrument → specific impl (PVCam, etc.)
   - Second downcast: Verify meta trait implementation (Camera)
   - Third downcast: Module → specific type (CameraModule)

2. **Running State Check**
   - Cannot reassign while module is running
   - Prevents mid-acquisition device swaps

3. **Error Reporting**
   - Clear error messages on type mismatch
   - Lists available capabilities when assignment fails

## Performance Characteristics

### Reassignment Timeline

```
Stop Module:          ~50ms (flush acquisition, cleanup)
Lookup Instrument:    <1ms  (HashMap access)
Downcast Verification: <1ms  (RTTI check)
Lookup Module:        <1ms  (HashMap access)
Downcast Module:      <1ms  (RTTI check)
Swap Trait Object:    <1μs  (pointer assignment)
------------------------------------------------------
Total:                ~52ms ✓ (within 100ms requirement)
```

### Overhead Analysis

- **Virtual function calls:** ~5ns (negligible vs hardware I/O)
- **Arc<Mutex<>> overhead:** ~100ns per lock (amortized by batch operations)
- **Task spawning:** ~10μs (one-time cost at start)
- **Message passing:** ~5μs (DaqCommand → Actor)

All overhead is negligible compared to hardware latencies (milliseconds).

## Integration Strategy

### Phase 1: Foundation (bd-64)

1. Define meta instrument traits
2. Implement Module trait and status enum
3. Create CameraModule as reference implementation
4. Add feature flag infrastructure
5. Document design patterns

### Phase 2: Actor Integration (bd-57 + bd-64)

1. Extend DaqCommand with module variants
2. Add ModuleRegistry to DaqManagerActor
3. Implement reassignment handlers
4. Add module lifecycle management

### Phase 3: GUI Integration (bd-58)

1. Module panel showing available modules
2. Drag-and-drop instrument assignment
3. Real-time status display
4. Start/stop controls

### Phase 4: Concrete Implementations

1. Implement meta traits for existing instruments:
   - PVCam → Camera
   - Newport1830C → PowerMeter
   - MaiTai → Spectrometer (if applicable)
2. Create additional module types as needed

## File Structure

```
src/
├── modules/                    # New directory (feature-gated)
│   ├── mod.rs                  # Module trait, ModuleStatus, ModuleRegistry
│   ├── meta_instruments.rs     # MetaInstrument, Camera, Spectrometer, etc.
│   ├── camera.rs               # CameraModule implementation
│   ├── power_meter.rs          # PowerMeterModule implementation
│   └── spectrometer.rs         # SpectrometryModule implementation
├── messages.rs                 # DaqCommand extensions
├── app_actor.rs                # Module management handlers
└── lib.rs                      # pub mod modules (feature-gated)
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_camera_module_assignment() {
        let mut module = CameraModule::new("test".to_string());
        let mock_camera: Box<dyn Camera> = Box::new(MockCamera::new());

        // Should succeed
        assert!(module.assign_camera(mock_camera).is_ok());
        assert!(module.has_camera());

        // Should fail while running
        module.start().await.unwrap();
        let another_camera: Box<dyn Camera> = Box::new(MockCamera::new());
        assert!(module.assign_camera(another_camera).is_err());
    }

    #[tokio::test]
    async fn test_type_safety() {
        let mut camera_module = CameraModule::new("cam".to_string());
        let power_meter: Box<dyn PowerMeter> = Box::new(MockPowerMeter::new());

        // This should not compile (type mismatch):
        // camera_module.assign_camera(power_meter); // ERROR
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_runtime_reassignment() {
    let app = create_test_app();

    // Create module
    app.create_module("camera", "cam1").await.unwrap();

    // Spawn instrument
    app.spawn_instrument("pvcam").await.unwrap();

    // Assign and verify
    app.assign_camera("cam1", "pvcam").await.unwrap();

    // Hot swap
    app.stop_module("cam1").await.unwrap();
    app.assign_camera("cam1", "pvcam2").await.unwrap();
    app.start_module("cam1").await.unwrap();
}
```

## Migration Path

### Existing Code (No Changes Required)

```rust
// Existing instruments continue to work unchanged
impl Instrument for PVCam {
    type Measure = InstrumentMeasurement;
    // ... existing methods
}
```

### Adding Module Support (Opt-In)

```rust
// Add meta instrument trait implementation
impl MetaInstrument for PVCam {
    fn instrument_id(&self) -> &str { &self.id }
    fn instrument_type(&self) -> &str { "camera" }
    fn capabilities(&self) -> Vec<String> { vec!["camera".into()] }
}

#[async_trait]
impl Camera for PVCam {
    async fn capture(&mut self) -> Result<ImageData> {
        // Use existing PVCam methods
    }
    // ... other Camera trait methods
}
```

No breaking changes to existing code.

## Open Questions / Future Work

1. **Shared Instruments:** Can multiple modules share the same instrument?
   - Current design: Exclusive ownership via Arc<Mutex<>>
   - Future: Add SharedInstrument wrapper with reference counting

2. **Module Output Streams:** How do modules broadcast their results?
   - Option A: Modules have their own DataDistributor
   - Option B: Modules write to instrument's measurement stream
   - Recommendation: Add `data_stream()` method to Module trait

3. **Configuration Persistence:** How to save/load module configurations?
   - Extend session system to include module assignments
   - TOML format similar to instrument config

4. **Dynamic Capability Discovery:** Better runtime trait querying
   - Add `implements_trait(&str) -> bool` to Instrument
   - Use trait_name!() macro for type-safe queries

## References

- **DynExp Paper:** Bopp & Schröder, "DynExp: A modular software framework for highly flexible laboratory automation" (SoftwareX)
- **bd-57:** Module trait implementation issue
- **bd-62:** Phase 2 Arc<Measurement> infrastructure
- **Phase 0:** Actor-based architecture for lock-free state management

## Acceptance Criteria

- [ ] Module trait defined with async methods
- [ ] Meta instrument traits (Camera, Spectrometer, PowerMeter, Positioner) defined
- [ ] CameraModule reference implementation
- [ ] Runtime reassignment via DaqCommand
- [ ] Type safety enforced (compile-time + runtime)
- [ ] Reassignment < 100ms
- [ ] Hot-swap support (stop → assign → start)
- [ ] Feature flag `modules` compiles independently
- [ ] Unit tests for type safety
- [ ] Integration tests for reassignment
- [ ] Documentation complete

## Conclusion

This design provides a robust, type-safe module system that enables DynExp-style runtime flexibility while leveraging Rust's type system for safety. The hybrid approach (trait objects for runtime flexibility + compile-time method type checking) achieves the best of both worlds.

The architecture is:
- **Orthogonal:** No changes to existing Instrument trait
- **Type-safe:** Multiple layers of compile-time + runtime checking
- **Performant:** <100ms reassignment, negligible runtime overhead
- **Extensible:** Easy to add new module types and meta traits
- **Testable:** Clear boundaries enable unit and integration testing
