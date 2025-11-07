# Helper Modules V2 Migration Plan

**Issue**: bd-dbc1 (Helper modules migration for V2 architecture)
**Date**: 2025-11-03
**Researcher**: Hive Mind Research Agent

## Executive Summary

This document provides a comprehensive analysis of helper modules used by V1 instruments and a migration strategy for V2 architecture. The research reveals that V2 instruments have already implemented their own patterns for serial/SCPI communication, making the V1 helper modules obsolete for new code.

## Current Helper Module Inventory

### 1. `src/instrument/scpi_common.rs` (175 lines)

**Purpose**: Common SCPI communication abstractions for V1 instruments.

**Key Components**:
- `ScpiTransport` trait: Abstraction for SCPI query/command operations
- `SerialScpiTransport`: Serial-based SCPI transport implementation
- `open_serial_instrument()`: Serial port configuration helper
- `parse_f64_response()`: SCPI response parsing
- `spawn_polling_task()`: Generic polling task spawner

**V1 Dependencies**:
- ✅ Uses `daq_core::Measurement` (already V2 compatible)
- ❌ Imports `crate::core::DataPoint` (V1 type)
- ⚠️ Uses `crate::instrument::serial_helper` (V1 helper)
- ✅ Uses `crate::adapters::serial::SerialAdapter` (V1/V2 compatible)

**V1 Instrument Usage**:
- **None found** - No V1 instruments currently import or use `scpi_common`
- This module appears to be legacy/unused code

**V2 Equivalent**:
- V2 instruments implement SCPI directly (see `src/instruments_v2/scpi_v3.rs`)
- V2 uses `write()` and `query()` methods directly on instrument structs
- No transport abstraction layer needed

### 2. `src/instrument/serial_helper.rs` (83 lines)

**Purpose**: Temporary serial communication helper for V1 instruments during V2 migration.

**Key Components**:
- `send_command_async()`: Send command and wait for delimited response
- Handles timeout, delimiter detection, and retries
- Feature-gated on `instrument_serial`

**V1 Dependencies**:
- ✅ Uses `crate::adapters::serial::SerialAdapter` (V1/V2 compatible)
- ✅ Uses `anyhow::Result` (architecture-agnostic)

**V1 Instrument Usage** (4 instruments):
1. **ESP300** (line 59): Motion controller
   - Terminator: `"\r\n"`, Delimiter: `b'\n'`, Timeout: 1s
   - Commands: Position queries (`TP`), velocity (`TV`), move (`PA`)

2. **MaiTai** (line 56): Tunable laser
   - Terminator: `"\r"`, Delimiter: `b'\r'`, Timeout: 2s
   - Commands: Wavelength, power, shutter control

3. **Newport 1830C** (line 68): Power meter
   - Terminator: `"\n"`, Delimiter: `b'\n'`, Timeout: 500ms
   - Commands: Power reading (`D?`), attenuator, filter

4. **Elliptec** (line 189): Rotation mount
   - Uses SerialAdapter directly via `serial_helper::send_command_async`
   - RS-485 multidrop protocol with device addressing

**V2 Equivalent**:
- V2 instruments implement serial communication directly
- Uses trait abstraction (`SerialPort` trait in V2 instruments)
- No shared helper module needed

### 3. `src/instrument/capabilities.rs` (303 lines)

**Purpose**: Capability-based instrument control system for V1 architecture.

**Key Components**:
- Trait definitions: `PositionControl`, `PowerMeasurement`, `SpectrumAnalyzer`
- Proxy pattern for capability-based command routing
- `CapabilityProxyHandle` enum for type-erased capabilities
- `InstrumentCommand::Capability` message passing

**V1 Dependencies**:
- ❌ `crate::core::InstrumentCommand` (V1 message-passing architecture)
- ❌ `crate::core::ParameterValue` (V1 type system)
- ⚠️ Uses `tokio::sync::mpsc` for command channels (V1 actor pattern)

**V1 Instrument Usage**:
- MaiTai: Implements `PowerMeasurement` capability
- Newport 1830C: Implements `PowerMeasurement` capability
- ESP300: Could implement `PositionControl` (not currently used)

**V2 Equivalent**:
- V2 uses direct trait implementation (e.g., `core_v3::Laser`, `core_v3::PowerMeter`)
- No proxy/message-passing pattern needed
- Direct async methods instead of capability commands

### 4. `src/instrument/config.rs` (125 lines)

**Purpose**: Type-safe configuration value objects for instruments.

**Key Components**:
- `MockInstrumentConfig`: Example configuration struct
- `from_toml()`: TOML parsing
- `validate()`: Parameter validation
- Serde-based deserialization

**Dependencies**:
- ✅ Architecture-agnostic (uses `anyhow`, `serde`, `toml`)
- ✅ No V1-specific types

**V1 Instrument Usage**:
- Only used by mock instrument (example code)
- Not used by production V1 instruments

**V2 Equivalent**:
- V2 instruments receive configuration via constructor parameters
- V2 uses `Parameter<T>` for runtime parameter management
- TOML parsing happens in app layer, not instrument layer

## V2 Best Practices (From Existing V2 Instruments)

### 1. Serial Communication Pattern (MaiTai V3)

```rust
// V2 Pattern: Trait abstraction for testing
#[async_trait]
trait SerialPort: Send + Sync {
    async fn write(&mut self, data: &str) -> Result<()>;
    async fn read_line(&mut self) -> Result<String>;
}

// Mock implementation for testing
struct MockSerialPort { /* state */ }

// Real implementation (feature-gated)
#[cfg(feature = "instrument_serial")]
struct RealSerialPort {
    port: std::sync::Mutex<Box<dyn serialport::SerialPort>>,
}

// Instrument owns the abstraction
pub struct MaiTaiV3 {
    serial_port: Option<Box<dyn SerialPort>>,
    // ...
}
```

**Key Differences from V1**:
- ✅ Trait abstraction for mock/real modes
- ✅ Owned by instrument (no shared SerialAdapter)
- ✅ Direct async methods (no helper module)
- ✅ Protocol-specific logic in instrument code

### 2. SCPI Communication Pattern (SCPI V3)

```rust
// V2 Pattern: VISA abstraction trait
#[async_trait]
trait VisaResource: Send + Sync {
    async fn write(&mut self, cmd: &str) -> Result<()>;
    async fn query(&mut self, cmd: &str) -> Result<String>;
    async fn close(&mut self) -> Result<()>;
}

// Instrument methods for SCPI operations
impl ScpiInstrumentV3 {
    pub async fn write(&self, cmd: &str) -> Result<()> { /* ... */ }
    pub async fn query(&self, cmd: &str) -> Result<String> { /* ... */ }
    pub async fn query_and_broadcast(&self, name: &str, cmd: &str, unit: &str) -> Result<f64> { /* ... */ }
}
```

**Key Differences from V1**:
- ✅ No transport abstraction layer needed
- ✅ Direct VISA resource ownership
- ✅ Convenience methods on instrument struct
- ✅ Broadcast integration at instrument level

### 3. No Shared Helper Modules

V2 instruments DO NOT use shared helper modules. Instead:
- Each instrument implements its own protocol logic
- Serial/VISA abstractions are local to the instrument
- Testing happens via trait mocks (not shared test infrastructure)
- Configuration comes from constructor parameters

**Rationale**:
- **Simplicity**: No indirection through helper modules
- **Testability**: Mock implementations via trait bounds
- **Maintainability**: Protocol logic co-located with instrument
- **Flexibility**: Each instrument can customize behavior

## Migration Strategy

### Option A: Create _v2 Versions (RECOMMENDED)

**Approach**: Create new V2-specific helper modules alongside V1 versions.

**Rationale**:
- ❌ **NOT RECOMMENDED**: V2 instruments already have better patterns
- V1 helpers are tightly coupled to V1 architecture
- V2 instruments don't need shared helpers
- Creating _v2 versions would duplicate already-existing V2 code

**Verdict**: **Do not create _v2 versions**

### Option B: Update In-Place (NOT RECOMMENDED)

**Approach**: Modify existing helper modules to support both V1 and V2.

**Rationale**:
- ❌ **NOT RECOMMENDED**: Would create complex dual-mode code
- V1 and V2 have fundamentally different architectures
- V2 instruments already use better patterns
- Would slow down V2 development

**Verdict**: **Do not update in place**

### Option C: Deprecate and Remove (RECOMMENDED)

**Approach**: Mark helper modules as deprecated, migrate V1 instruments to V2, remove helpers.

**Rationale**:
- ✅ **RECOMMENDED**: Clean migration path
- V2 instruments don't need these helpers
- Removes technical debt
- Forces proper V2 adoption

**Migration Order**:
1. **Phase 1**: Mark helpers as deprecated in documentation
2. **Phase 2**: Migrate V1 instruments to V2 (bd-51, bd-dbc0)
3. **Phase 3**: Remove helper modules when all V1 instruments migrated

## Timeline Estimate

### Phase 1: Documentation (1 day) - CURRENT PHASE
- ✅ Document helper module inventory
- ✅ Identify V1 instrument dependencies
- ✅ Document V2 patterns
- ✅ Create migration plan

### Phase 2: V1 to V2 Instrument Migration (2-3 weeks)
**Dependencies**: bd-51 (V2 integration), bd-dbc0 (Instrumentation redesign)

**Per-Instrument Estimates**:
- ESP300: 2-3 days (complex motion control)
- MaiTai: 1-2 days (already has V3 implementation as reference)
- Newport 1830C: 1-2 days (simple power meter)
- Elliptec: 2-3 days (complex RS-485 protocol)

**Includes**:
- Implement V2 instrument struct
- Port serial/SCPI logic to local methods
- Create mock implementations for testing
- Update configuration handling
- Integration testing

### Phase 3: Helper Module Removal (1 day)
**Dependencies**: All V1 instruments migrated to V2

**Tasks**:
- Remove `src/instrument/serial_helper.rs`
- Remove `src/instrument/scpi_common.rs`
- Remove capability system (if unused)
- Update imports across codebase
- Verify compilation

## Recommendations

### 1. Do NOT Create V2 Helper Modules

V2 instruments already implement optimal patterns:
- Direct serial/SCPI methods on instrument structs
- Local trait abstractions for testing
- No shared helper code needed

**Action**: Document V2 patterns in `CLAUDE.md`, do not create new helpers.

### 2. Migrate V1 Instruments to V2 Architecture

Priority order based on complexity and usage:
1. **Newport 1830C** (simplest, good test case)
2. **MaiTai** (already has V3 reference implementation)
3. **ESP300** (motion control complexity)
4. **Elliptec** (most complex protocol)

**Action**: Create per-instrument migration issues in bd tracker.

### 3. Remove Helper Modules After V1 Instruments Gone

Helper modules are V1-specific technical debt:
- `serial_helper.rs`: Only used by 4 V1 instruments
- `scpi_common.rs`: Unused (zero imports)
- `capabilities.rs`: V1 message-passing architecture

**Action**: Remove after V1 instrument migration complete.

### 4. Document V2 Patterns as Examples

The existing V2 instruments demonstrate excellent patterns:
- `src/instruments_v2/maitai_v3.rs`: Serial communication pattern
- `src/instruments_v2/scpi_v3.rs`: SCPI/VISA abstraction pattern
- `src/instruments_v2/esp300_v3.rs`: Motion control pattern

**Action**: Reference V2 instruments as examples in migration docs.

## Breaking Changes to Handle

### 1. Serial Communication Architecture

**V1 Pattern**:
```rust
// Shared SerialAdapter + helper function
use crate::adapters::serial::SerialAdapter;
use crate::instrument::serial_helper;

serial_helper::send_command_async(adapter, id, cmd, term, timeout, delim).await?
```

**V2 Pattern**:
```rust
// Local SerialPort trait + owned implementation
#[async_trait]
trait SerialPort: Send + Sync {
    async fn write(&mut self, data: &str) -> Result<()>;
    async fn read_line(&mut self) -> Result<String>;
}

impl MyInstrumentV2 {
    async fn send_command(&mut self, cmd: &str) -> Result<()> { /* ... */ }
}
```

**Breaking**: V1 instruments must implement their own serial methods.

### 2. SCPI Command Pattern

**V1 Pattern**:
```rust
// ScpiTransport trait + polling task helper
use crate::instrument::scpi_common::{ScpiTransport, spawn_polling_task};

let transport = SerialScpiTransport::new_rs232(adapter);
spawn_polling_task(interval, tx, || async { transport.query("CMD?").await });
```

**V2 Pattern**:
```rust
// Direct async methods on instrument
impl ScpiInstrumentV3 {
    pub async fn query(&self, cmd: &str) -> Result<String> { /* ... */ }
    pub async fn query_and_broadcast(&self, name: &str, cmd: &str, unit: &str) -> Result<f64> { /* ... */ }
}
```

**Breaking**: V1 instruments must implement polling logic directly.

### 3. Capability System

**V1 Pattern**:
```rust
// Proxy-based capability system
use crate::instrument::capabilities::{PowerMeasurement, CapabilityProxyHandle};

fn capabilities(&self) -> Vec<TypeId> {
    vec![power_measurement_capability_id()]
}

async fn handle_command(&mut self, cmd: InstrumentCommand) -> Result<()> {
    match cmd {
        InstrumentCommand::Capability { capability, operation, parameters } => { /* ... */ }
    }
}
```

**V2 Pattern**:
```rust
// Direct trait implementation
#[async_trait]
impl PowerMeter for Newport1830CV3 {
    async fn read_power(&self) -> Result<f64> { /* ... */ }
    async fn set_range(&mut self, range: f64) -> Result<()> { /* ... */ }
}
```

**Breaking**: V1 capability system completely replaced by direct trait methods.

## Risk Assessment

### Low Risk

1. **Helper module removal**: Clean break after V1 instruments gone
2. **SCPI pattern migration**: V2 pattern already proven in V3 instruments
3. **Serial abstraction**: Local traits work well in V2 instruments

### Medium Risk

1. **Configuration migration**: V1 uses TOML directly, V2 uses constructor params
2. **Testing infrastructure**: Mock implementations need per-instrument creation
3. **Protocol edge cases**: Each instrument may have quirks to preserve

### High Risk

1. **Capability system removal**: If GUI depends on capability queries
2. **Breaking API changes**: If external code depends on helper modules
3. **Hardware validation**: Limited access to physical instruments for testing

## Gaps Identified

### 1. No Shared Test Infrastructure in V2

**Gap**: V2 instruments implement mocks individually (no shared mock serial/VISA).

**Impact**: Duplicated mock code across V2 instruments.

**Recommendation**:
- Document common mock patterns in `CLAUDE.md`
- Consider shared mock traits in future (after Phase 1 complete)
- Current per-instrument mocks are acceptable for Phase 1

### 2. No V2 Configuration Helper

**Gap**: V1 has `config.rs` for type-safe TOML parsing, V2 uses ad-hoc constructor params.

**Impact**: Configuration validation happens at runtime, not parse time.

**Recommendation**:
- Document configuration patterns in V2 instrument docs
- Consider shared config types in future
- Current approach is acceptable for Phase 1

### 3. No V2 Capability Query System

**Gap**: V1 uses `capabilities()` for runtime capability discovery, V2 uses static trait bounds.

**Impact**: Runtime discovery of instrument features not possible in V2.

**Recommendation**:
- Acceptable trade-off for simplicity
- Trait bounds provide compile-time safety
- If needed, implement capability query via metadata in V3

## Conclusion

**Summary**: V1 helper modules are V1-specific technical debt. V2 instruments have already implemented superior patterns that eliminate the need for shared helpers. The recommended approach is:

1. ✅ **Do NOT create V2 helper modules** (V2 patterns already optimal)
2. ✅ **Migrate V1 instruments to V2** (per-instrument migration, 2-3 weeks)
3. ✅ **Remove helper modules** (after all V1 instruments migrated)

**Timeline**: 2-3 weeks for full migration, 1 day for helper removal.

**Dependencies**: bd-51 (V2 integration), bd-dbc0 (instrumentation redesign).

**Next Steps**:
1. Create per-instrument migration issues in bd tracker
2. Start with Newport 1830C (simplest)
3. Use existing V3 instruments as reference implementations
4. Remove helpers after all V1 instruments migrated

---

**Research Complete**: 2025-11-03
**Researcher**: Hive Mind Research Agent
**Coordination**: See `.swarm/memory.db` for shared findings
