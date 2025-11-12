# Architectural Analysis: rust-daq vs Reference Frameworks

**Date**: 2025-10-25
**Status**: Critical architectural review before proceeding
**Recommendation**: STRATEGIC RESET REQUIRED

---

## Executive Summary

After comprehensive analysis of DynExp (C++), PyMODAQ (Python), ScopeFoundry (Python), and Qudi (Python), we have identified **fundamental architectural mismatches** in rust-daq that create roadblocks. The current design is more complex than reference frameworks without providing equivalent benefits.

**Key Finding**: All reference frameworks use **direct method calls + Qt signals/slots**, not actor-based message passing. Our Tokio actor model adds unnecessary latency and complexity.

---

## Reference Framework Comparison

### 1. DynExp (C++, Qt-based)

**Three-Tier Architecture**:
```
HardwareAdapter → Instrument → Module
```

**Key Patterns**:
- **HardwareAdapters**: Low-level device communication (serial, USB, network)
- **Instruments**: Hardware-specific implementations OR hardware-agnostic "meta instruments"
- **Modules**: Application logic that works only with meta instruments (polymorphism)
- **Runtime Reconfiguration**: "HardwareAdapters can be assigned and reassigned to (multiple) Instruments... at runtime without any programming"

**Communication**:
- Task-based between modules and instruments
- Event-based between modules and UI (Qt signals/slots)
- Data streams for real-time measurements
- gRPC for network distribution

**Data Flow**: Simple and direct
```cpp
// DynExp pattern (conceptual)
auto instrument = GetInstrument("camera");
instrument->StartAcquisition();  // Direct call
// Data flows via Qt signals to GUI
```

**What We Can Learn**:
1. Runtime reconfiguration doesn't need actor model
2. Polymorphism (meta instruments) enables hardware-agnostic modules
3. Task-based communication is simpler than full actor model
4. Qt event loop handles threading naturally

---

### 2. PyMODAQ (Python, Qt-based)

**Plugin Architecture**:
```
DAQ_Move_base / DAQ_Viewer_base → Dashboard → Extensions (Scan, Logger)
```

**Key Patterns**:
- **DAQ_Move**: Actuator control plugins
- **DAQ_Viewer**: Detector/sensor plugins
- **Dashboard**: Orchestrates multiple instruments
- **Extensions**: DAQ_Scan (automated sweeps), DAQ_Logger (HDF5 persistence)

**Data Emission**:
```python
# PyMODAQ plugin pattern
self.data_grabed_signal.emit([
    DataFromPlugins(name='Ch1', data=data1, dim='Data0D'),
    DataFromPlugins(name='Ch2', data=data2, dim='Data2D')
])
```

**Dynamic Viewer Creation**:
- Number of DataFromPlugins objects = number of GUI viewers created
- `dim` attribute ('Data0D', 'Data1D', 'Data2D') determines viewer type
- GUI automatically adapts to plugin capabilities

**Communication**: Qt signals/slots
```python
# 3 lines for camera acquisition
def grab_camera(self):
    data = self.camera.read_frame()  # Direct call
    self.emit_data(data)  # Qt signal
```

**What We Can Learn**:
1. Plugins are **simple** - just emit data
2. GUI dynamically creates viewers based on data structure
3. No complex message passing - Qt handles everything
4. Separate signals for temporary data vs final data

---

### 3. ScopeFoundry (Python, Qt-based)

**Plugin System**:
```
HardwareComponent + Measurement → App
```

**Key Innovation: LoggedQuantity**

The killer feature of ScopeFoundry is `LoggedQuantity` - a unified abstraction for parameters that synchronizes:
- **GUI widgets** (automatic bidirectional binding)
- **Hardware devices** (read_func/write_func callbacks)
- **Application logic** (change listeners)
- **File storage** (automatic persistence)

```python
# LoggedQuantity example
class MyCameraHW(HardwareComponent):
    def setup(self):
        # Create logged quantity
        self.exposure = self.settings.New('exposure_ms', dtype=float,
                                          initial=100, vmin=1, vmax=10000)

    def connect(self):
        # Connect to hardware
        self.exposure.connect_to_hardware(
            read_func=self.camera.get_exposure,
            write_func=self.camera.set_exposure
        )

        # Connect to GUI (automatic widget creation)
        self.exposure.connect_to_widget(some_spinbox)
```

**Threading Model**:
- Framework handles threading automatically
- `HardwareComponent.lock` (Qt QLock) for thread safety
- `thread_lock_all_lq()` synchronizes all LoggedQuantities with hardware lock

**What We Can Learn**:
1. **Declarative parameter management** - specify what syncs, not how
2. GUI widgets auto-created from dtype
3. Thread safety built into abstraction
4. Direct hardware synchronization via callbacks

---

### 4. Qudi (Python, Qt-based)

**Modular Layers**:
```
Hardware Modules ← Logic Modules ← GUI Modules
```

**Key Patterns**:
- **Hardware modules**: Abstract hardware interfaces
- **Logic modules**: Experiment control and processing
- **GUI modules**: Visualization (separate layer)
- **Connector system**: Modules declare dependencies, runtime wiring

**Focus**: Quantum experiment control (confocal microscopy, NV centers)

**What We Can Learn**:
1. Clear separation: Hardware ← Logic ← GUI
2. Module dependencies declared, not hardcoded
3. Specialized for scientific experiments, not general DAQ

---

## Common Patterns Across All Reference Frameworks

### ✅ What They All Do

1. **Qt-based GUI** with signals/slots for communication
2. **Plugin/module discovery** at runtime
3. **Direct method calls** instead of message passing
4. **Hardware abstraction layers** (adapters, components)
5. **Configuration via TOML/JSON/XML**
6. **HDF5 for data storage**
7. **Simple data flow**: instrument → signal → GUI/storage

### ❌ What None of Them Do

1. **Actor model** with message queues
2. **Generic type parameters** for measurement types
3. **V1/V2 instrument splits**
4. **Complex broadcast channels** for data distribution
5. **Capability-based proxies** for module-instrument coupling

---

## rust-daq Current Architecture (Problems)

### Problem 1: Actor Model Overhead

**Current Pattern**:
```
GUI → DaqCommand → Actor mailbox → InstrumentCommand → Instrument → broadcast
```

**Reference Pattern**:
```
GUI → instrument.method() → Qt signal → GUI update
```

**Comparison**:
- **PyMODAQ**: 3 lines for camera acquisition
- **rust-daq**: 3 layers of indirection

**Why It's Wrong**:
- Instruments are already async (Tokio tasks)
- Actor adds serialization overhead for local calls
- No parallelism benefit (instruments don't compete for resources)
- Debugging is harder (message passing obscures call stack)

### Problem 2: V1/V2 Instrument Split

**Current State**:
- V1: `InstrumentMeasurement` (wrapper) → `Measurement::Scalar`
- V2: `Measurement` enum directly
- Two parallel codebases, no clear migration

**Reference Approach**:
- DynExp: Instruments emit via signals, data type handled by consumers
- PyMODAQ: `DataFromPlugins` with `dim` attribute
- ScopeFoundry: `LoggedQuantity` dtype + flexible data types

**Why It's Wrong**:
- Creates confusion about which pattern to follow
- Conversion overhead (V1 → V2)
- Never finished migrating
- No architectural benefit

### Problem 3: Broken Module System

**Vestigial Code**:
```rust
trait ModuleWithInstrument<M: Measure + 'static> {
    fn assign_instrument(&mut self, id: String,
                         instrument: Arc<dyn Instrument<Measure = M>>);
}
```

**Status**: Defined but **NEVER CALLED**

**Problem**:
- Capability system replaced trait-based assignment
- But `PowerMeterModule<M>` still stores `Arc<dyn Instrument<Measure = M>>`
- Two incompatible systems coexisting

**Reference Approach**:
- DynExp: Modules work with meta instruments (polymorphism)
- PyMODAQ: Dashboard assigns plugins to roles
- ScopeFoundry: Measurements declare hardware dependencies

### Problem 4: Generic `<M: Measure>` Lost Type Safety

**Original Intent**:
```rust
trait Instrument {
    type Measure: Measure;  // Compile-time type safety
}
```

**Current Reality**:
```rust
enum Measurement {  // Runtime type erasure
    Scalar(DataPoint),
    Image(ImageData),
    Spectrum(SpectrumData),
}

struct DaqManagerActor<M: Measure> {  // M unused
    // ... M = InstrumentMeasurement (always)
}
```

**Why It Failed**:
- GUI needs heterogeneous data types (scalars + images)
- Enum was easier than existential types
- But generic wasn't removed → complexity without benefit

**Reference Approach**:
- DynExp: Task-based data streaming, consumers handle types
- PyMODAQ: `dim` attribute determines viewer type
- ScopeFoundry: LoggedQuantity dtype + flexible Measurement types

---

## Architectural Recommendations

### Option 1: Radical Simplification (RECOMMENDED)

Follow DynExp + PyMODAQ + ScopeFoundry patterns:

**1. Remove Actor Model**
```rust
// Before
manager.send(DaqCommand::StartInstrument { id }).await?;

// After
manager.start_instrument(&id).await?;
```

**2. Unified Instrument Trait**
```rust
trait Instrument: Send + Sync {
    fn id(&self) -> &str;
    fn state(&self) -> InstrumentState;

    // Simple lifecycle
    async fn initialize(&mut self) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;

    // Data channel (like PyMODAQ DataFromPlugins)
    fn data_channel(&self) -> Receiver<DataPacket>;

    // Commands (like DynExp task-based)
    async fn execute(&mut self, cmd: Command) -> Result<Response>;
}
```

**3. DataPacket Enum (Like DynExp)**
```rust
enum DataPacket {
    Scalar {
        name: String,
        value: f64,
        unit: String,
        time: DateTime<Utc>,
    },
    Vector {
        name: String,
        values: Vec<f64>,
        unit: String,
        time: DateTime<Utc>,
    },
    Image {
        name: String,
        pixels: Array2<u16>,
        unit: String,
        time: DateTime<Utc>,
    },
}
```

**4. Direct Manager (No Actor)**
```rust
struct DaqManager {
    instruments: HashMap<String, Box<dyn Instrument>>,
    // No generic M, no actor
}

impl DaqManager {
    async fn start_instrument(&mut self, id: &str) -> Result<()> {
        let inst = self.instruments.get_mut(id)?;
        inst.execute(Command::Start).await
    }

    fn subscribe(&self, id: &str) -> Receiver<DataPacket> {
        self.instruments.get(id)?.data_channel()
    }
}
```

**5. LoggedQuantity-Inspired Parameters (Like ScopeFoundry)**
```rust
struct Parameter<T> {
    name: String,
    value: T,
    dtype: ParameterType,

    // Callbacks (like ScopeFoundry)
    on_change: Vec<Box<dyn Fn(&T)>>,

    // Hardware sync (optional)
    read_func: Option<Box<dyn Fn() -> Result<T>>>,
    write_func: Option<Box<dyn Fn(&T) -> Result<()>>>,
}

impl<T> Parameter<T> {
    fn connect_to_hardware(&mut self,
                           read: impl Fn() -> Result<T>,
                           write: impl Fn(&T) -> Result<()>) {
        self.read_func = Some(Box::new(read));
        self.write_func = Some(Box::new(write));
    }

    fn set(&mut self, value: T) -> Result<()> {
        if let Some(write_func) = &self.write_func {
            write_func(&value)?;
        }
        self.value = value;
        for callback in &self.on_change {
            callback(&value);
        }
        Ok(())
    }
}
```

**Benefits**:
- Aligns with all reference frameworks
- 50% less code
- No message passing overhead
- Direct call stack (easier debugging)
- Instruments remain async (Tokio tasks)
- Simplified testing

**Effort**: 2-3 weeks
**Risk**: High (major refactor), but validated by reference implementations

---

### Option 2: Incremental Improvements (Lower Risk)

Keep structure, fix obvious issues:

1. **Unify V1/V2**: Migrate all to V2-style `Measurement`
2. **Remove vestigial code**: Delete `ModuleWithInstrument<M>`
3. **Simplify generic**: Make DaqManager concrete (no `<M>`)
4. **Keep actor**: Don't rock the boat

**Benefits**: Low risk, removes confusion
**Drawbacks**: Doesn't fix fundamental issues
**Effort**: 1 week

---

### Option 3: Deep Study + Prototype (RECOMMENDED FIRST STEP)

**Week 1: Deep Dive**
1. Clone DynExp, PyMODAQ, ScopeFoundry repos
2. Trace one instrument end-to-end (camera preferred)
3. Document exact data flow patterns
4. Understand configuration systems

**Week 2: Rust Prototype**
1. Implement simplified `Instrument` trait
2. Port MockInstrument to new pattern
3. Prove data flow works
4. Benchmark vs current architecture

**Weeks 3-4: Migration**
1. Deprecate actor model
2. Unify V1/V2
3. Migrate instruments incrementally
4. Comprehensive testing

---

## Specific Lessons for rust-daq

### From DynExp

1. **Three-tier separation works**: HardwareAdapter → Instrument → Module
2. **Meta instruments enable polymorphism**: Modules work with abstractions
3. **Runtime reconfiguration**: Don't need actor model for dynamic assignment
4. **Task-based** communication simpler than full actor model

### From PyMODAQ

1. **Plugins are simple**: Just emit DataFromPlugins
2. **Dynamic GUI creation**: Viewers adapt to data structure
3. **Signal/slot** is all you need (Qt or custom)
4. **3 lines** for instrument control (direct calls)

### From ScopeFoundry

1. **LoggedQuantity pattern is brilliant**: Unified parameter abstraction
2. **Declarative synchronization**: Specify what, not how
3. **Thread safety built-in**: No manual lock management
4. **Automatic widget creation**: From dtype specification

### From All Frameworks

1. **Direct method calls > message passing** for instrument control
2. **Qt signals/slots (or equivalent)** for data distribution
3. **Plugin discovery** at runtime (not compile-time registry)
4. **HDF5 storage** is standard
5. **Simplicity wins**: 3 lines beats 3 layers

---

## Critical Questions Answered

**Q: Why actor model?**
**A**: No good reason. Reference frameworks don't use it. Adds overhead.

**Q: Why generic `Instrument<Measure = M>`?**
**A**: Originally for type safety, but Measurement enum erases types. Generic is now vestigial.

**Q: Why V1/V2 split?**
**A**: V2 created for Image/Spectrum data. Never finished migrating. Should unify.

**Q: Why modules in current form?**
**A**: Premature abstraction. Reference frameworks have real modules (scans, automation). Ours barely add value.

**Q: Do we need Tokio?**
**A**: YES for async instrument I/O (serial, network). NO for orchestration (use direct calls).

---

## Strategic Recommendation

**PAUSE all feature development.**

**Execute Option 3 (Study + Prototype):**

1. **Week 1**: Deep dive into DynExp source code
   - Understand HardwareAdapter → Instrument → Module flow
   - Document exact patterns
   - Identify what translates to Rust

2. **Week 2**: Rust prototype
   - Simplified Instrument trait
   - MockInstrument + PVCAM ports
   - Prove concept works

3. **Weeks 3-4**: Migration if prototype validates
   - Remove actor model
   - Unify V1/V2
   - Direct calls + channels (not broadcast)
   - Keep Tokio for instrument tasks only

**Why This Approach**:
- We already made mistakes by not studying references
- DynExp is battle-tested C++ with similar goals
- 2 weeks of study prevents months of wrong direction
- Prototype validates before committing to major refactor

---

## Conclusion

The user's intuition was **absolutely correct**: we're hitting logical roadblocks because our architecture doesn't match proven patterns from DynExp, PyMODAQ, ScopeFoundry, or Qudi.

**Core Issues**:
1. Actor model adds complexity without benefit
2. V1/V2 split is incomplete technical debt
3. Module system has vestigial code from abandoned patterns
4. Generic `<M>` provides no value after Measurement enum

**Path Forward**:
1. Study reference implementations deeply (especially DynExp)
2. Prototype simplified architecture
3. Radical simplification if prototype validates
4. **DO NOT proceed with current architecture**

**Bottom Line**: PVCAM V2 works (33/33 tests pass), but this doesn't validate the architecture. We need a strategic reset based on proven patterns from successful frameworks.
