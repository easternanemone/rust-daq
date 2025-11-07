# Phase 2 Plan: Instrument Migration

**Date**: 2025-10-25  
**Status**: 🔄 IN PROGRESS  
**Duration**: Weeks 3-4 (estimated 2 weeks)  
**Previous Phase**: Phase 1 Complete ✅

---

## Objective

Migrate all instruments from V1/V2 architecture to unified V3 architecture, proving the new design scales across different instrument types and complexities.

---

## Priority Order

1. ✅ **MockInstrument** (Phase 1) - COMPLETE
   - MockCameraV3 implemented and tested
   - Serves as reference implementation

2. 🔄 **PVCAM Camera** (Current) - IN PROGRESS
   - Most complex instrument (2209 lines)
   - Proves scalability of V3 architecture
   - SDK abstraction layer
   - High-frequency data streaming (10-30 Hz)
   - Multiple parameters (exposure, ROI, binning, gain, trigger mode)

3. **Newport 1830C** (Power Meter)
   - Simple serial instrument (~400 lines)
   - Demonstrates PowerMeter trait
   - Single scalar measurement

4. **ESP300** (Stage Controller)
   - Serial instrument (~600 lines)
   - Demonstrates Stage trait
   - Position control + motion

5. **MaiTai** (Laser)
   - Serial instrument (~400 lines)
   - Demonstrates Laser trait
   - Wavelength + power control

6. **Elliptec** (Stage Controller)
   - Serial instrument (~300 lines)
   - Another Stage trait implementation
   - Validates trait reusability

7. **VISA Instruments** (Generic)
   - SCPI-based instruments
   - Demonstrates generic SCPI pattern
   - Template for future VISA instruments

---

## Per-Instrument Migration Checklist

For each instrument:

### Phase A: Analysis (30 min)
- [ ] Read existing V1/V2 implementation
- [ ] Identify meta trait(s) to implement (Camera, Stage, PowerMeter, etc.)
- [ ] List all parameters and their constraints
- [ ] Document command handling patterns
- [ ] Identify data streaming approach

### Phase B: Implementation (2-4 hours)
- [ ] Create new `{instrument}_v3.rs` file
- [ ] Implement base `Instrument` trait
- [ ] Implement relevant meta trait(s)
- [ ] Convert parameters to `Parameter<T>` with constraints
- [ ] Replace `handle_command()` with direct trait methods
- [ ] Implement data streaming with `data_channel()`
- [ ] Add shutdown handling

### Phase C: Testing (1-2 hours)
- [ ] Port existing unit tests to V3 API
- [ ] Add new tests for meta trait methods
- [ ] Test parameter validation
- [ ] Test state transitions
- [ ] Test data streaming

### Phase D: Validation (30 min)
- [ ] All tests passing (unit + integration)
- [ ] Benchmark vs V1/V2 (must equal or exceed performance)
- [ ] Memory profiling (ensure no regressions)
- [ ] Code review checklist

---

## Migration Template

Based on MockCameraV3, here's the template for migrating instruments:

```rust
//! {InstrumentName} V3 - Unified architecture implementation

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::core_v3::{
    {MetaTrait}, Command, Instrument, InstrumentState, Measurement,
    ParameterBase, Response,
};
use crate::parameter::{Parameter, ParameterBuilder};

pub struct {InstrumentName}V3 {
    id: String,
    state: InstrumentState,
    data_tx: broadcast::Sender<Measurement>,
    parameters: HashMap<String, Box<dyn ParameterBase>>,
    
    // Typed parameters for direct access
    param1: Arc<RwLock<Parameter<Type1>>>,
    param2: Arc<RwLock<Parameter<Type2>>>,
    
    // Instrument-specific state
    // ...
}

impl {InstrumentName}V3 {
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        let (data_tx, _) = broadcast::channel(1024);
        
        // Create parameters with constraints
        let param1 = Arc::new(RwLock::new(
            ParameterBuilder::new("param1", default_value)
                .description("Parameter 1")
                .unit("unit")
                .range(min, max)
                .build(),
        ));
        
        // ... more parameters
        
        Self {
            id,
            state: InstrumentState::Uninitialized,
            data_tx,
            parameters: HashMap::new(),
            param1,
            // ...
        }
    }
}

#[async_trait]
impl Instrument for {InstrumentName}V3 {
    fn id(&self) -> &str { &self.id }
    fn state(&self) -> InstrumentState { self.state }
    
    async fn initialize(&mut self) -> Result<()> {
        // Hardware initialization
        self.state = InstrumentState::Idle;
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<()> {
        // Cleanup
        self.state = InstrumentState::ShuttingDown;
        Ok(())
    }
    
    fn data_channel(&self) -> broadcast::Receiver<Measurement> {
        self.data_tx.subscribe()
    }
    
    async fn execute(&mut self, cmd: Command) -> Result<Response> {
        match cmd {
            Command::Start => { /* ... */ Ok(Response::Ok) }
            Command::Stop => { /* ... */ Ok(Response::Ok) }
            Command::GetParameter(name) => { /* ... */ }
            Command::SetParameter(name, value) => { /* ... */ }
            _ => Ok(Response::Error("Unsupported".to_string()))
        }
    }
    
    fn parameters(&self) -> &HashMap<String, Box<dyn ParameterBase>> {
        &self.parameters
    }
    
    fn parameters_mut(&mut self) -> &mut HashMap<String, Box<dyn ParameterBase>> {
        &mut self.parameters
    }
}

#[async_trait]
impl {MetaTrait} for {InstrumentName}V3 {
    // Implement meta trait methods
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_initialization() { /* ... */ }
    
    #[tokio::test]
    async fn test_parameters() { /* ... */ }
    
    #[tokio::test]
    async fn test_data_streaming() { /* ... */ }
}
```

---

## PVCAM V3 Specific Plan

### Analysis

**Existing Implementation**: `src/instruments_v2/pvcam.rs` (2209 lines)

**Meta Trait**: `Camera`

**Parameters** (7 total):
1. `exposure_ms: f64` - Range: 1.0 - 10000.0 ms
2. `roi: ROI` - Struct with x, y, width, height (bounded by sensor size)
3. `binning: (u16, u16)` - Choices: (1,1), (2,2), (4,4), (8,8)
4. `gain: u16` - Range: depends on camera model
5. `trigger_mode: TriggerMode` - Enum: Internal, ExternalEdge, ExternalLevel
6. `camera_name: String` - Read-only after initialization
7. `polling_rate_hz: f64` - Range: 1.0 - 100.0 Hz

**Data Streaming**:
- Already emits `Measurement::Image` with `PixelBuffer::U16`
- Background task polls SDK at specified rate
- Atomic counters for diagnostics (total_frames, dropped_frames)

**SDK Abstraction**:
- Keep existing `PvcamSdk` trait (MockPvcamSdk, RealPvcamSdk)
- SDK mode selection: Mock or Real
- CameraHandle wraps SDK lifecycle

**Key Challenges**:
1. SDK initialization in `initialize()` vs constructor
2. Background polling task management
3. Atomic counter access from trait methods
4. ROI validation against sensor size
5. Trigger mode enum handling

### Implementation Strategy

**Phase 1**: Core Structure (1 hour)
- Create `src/instruments_v2/pvcam_v3.rs`
- Define `PVCAMCameraV3` struct
- Set up parameters with constraints
- Implement basic `Instrument` trait

**Phase 2**: Camera Trait (1 hour)
- Implement `Camera` trait methods
- Wire parameters to trait methods
- Handle ROI/binning validation

**Phase 3**: Data Streaming (1 hour)
- Port background polling task
- Convert to direct broadcast (no double-broadcast)
- Maintain atomic counters

**Phase 4**: SDK Integration (1 hour)
- Port SDK initialization logic
- Handle Mock vs Real mode selection
- Implement shutdown sequence

**Phase 5**: Testing (2 hours)
- Port 33 existing tests from V2
- Add Camera trait tests
- Add parameter validation tests
- Verify diagnostic counters

**Total Estimate**: 6 hours

---

## Success Criteria

### Technical Metrics

For each migrated instrument:
- [ ] All existing tests pass (ported to V3 API)
- [ ] New tests for meta trait methods pass
- [ ] Parameter validation works correctly
- [ ] Performance equals or exceeds V1/V2
- [ ] Memory usage equals or better than V1/V2
- [ ] No data races (validated with atomic counters)

### Code Quality

- [ ] Follows V3 patterns from MockCameraV3
- [ ] Comprehensive inline documentation
- [ ] Clear error messages
- [ ] Proper resource cleanup (shutdown)
- [ ] No clippy warnings
- [ ] Code formatted with rustfmt

### Integration

- [ ] Compiles alongside V1/V2 code (no conflicts)
- [ ] Can be instantiated and used
- [ ] Data streaming works end-to-end
- [ ] Parameters update correctly

---

## Risk Mitigation

### Risk: Performance Regression

**Mitigation**:
- Benchmark each instrument vs V1/V2
- Use same SDK abstraction (no extra layers)
- Direct broadcast (eliminate double-copy)
- Profiling with `cargo flamegraph`

**Acceptance**: Performance must equal or exceed V1/V2

### Risk: Complex State Management

**Mitigation**:
- Follow MockCameraV3 template
- Use `Arc<RwLock<Parameter<T>>>` for thread-safe access
- Keep background tasks simple (polling + broadcast)
- Clear state transitions

**Detection**: Unit tests for state transitions

### Risk: Incomplete Migration

**Mitigation**:
- Migration checklist per instrument
- Code review after each migration
- Validation tests before marking complete

**Prevention**: TodoList tracking, clear deliverables

---

## Timeline

**Week 3** (Now - Week 3 End):
- Day 1-2: PVCAM V3 (6 hours)
- Day 3: Newport 1830C V3 (3 hours)
- Day 4: ESP300 V3 (4 hours)
- Day 5: MaiTai V3 (3 hours)

**Week 4** (Week 4 Start - Week 4 End):
- Day 1: Elliptec V3 (3 hours)
- Day 2-3: VISA/SCPI instruments V3 (6 hours)
- Day 4: Integration testing (4 hours)
- Day 5: Performance benchmarking + Phase 2 completion report

**Total**: 29 hours over 2 weeks

---

## Tracking

**Current Status**:
- ✅ MockInstrument (MockCameraV3)
- 🔄 PVCAM (in progress)
- ⏳ Newport 1830C
- ⏳ ESP300
- ⏳ MaiTai
- ⏳ Elliptec
- ⏳ VISA/SCPI

**Next Action**: Implement PVCAM V3 Phase 1 (Core Structure)

---

## References

- Phase 1 Completion: `docs/PHASE_1_COMPLETION.md`
- Architectural Redesign: `docs/ARCHITECTURAL_REDESIGN_2025.md`
- MockCameraV3 Reference: `src/instrument/mock_v3.rs`
- PVCAM V2 Source: `src/instruments_v2/pvcam.rs`
- Core V3 Traits: `src/core_v3.rs`
- Parameter System: `src/parameter.rs`

---

**Phase 2 Status**: 🔄 **IN PROGRESS**

**Current Task**: PVCAM V3 Implementation

**Estimated Completion**: 6 hours from start
