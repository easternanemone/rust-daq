# Jules Agent Dependency Coordination Map

**Jules-17 (Dependency Coordinator) - Mission Control**

Created: 2025-11-20
Status: Active Coordination

## Executive Summary

**Total Tasks**: 14 coding tasks across 4 phases (P1-P4)
**Critical Path**: P4.1 (ScriptEngine trait) → P4.2, P4.4 (PyO3/Rhai backends)
**Current Blockers**: 5 tasks blocked on ScriptEngine trait definition
**Ready Tasks**: 9 tasks (P1-P3 migrations and cleanups)
**Progress**: 27% overall (Phase 1 complete, Phase 2 in progress)

## Dependency Graph (Visual)

```
PHASE 1 (Infrastructure) - ✅ COMPLETE
└─ bd-r896: Kameo vs V3 Performance Analysis [CLOSED]
└─ bd-o6c7: V4 SCPI Actor → V3 Migration [CLOSED]
└─ bd-ifxt: Fix V3 Import Consolidation [CLOSED]

PHASE 2 (V3 Instrument Migrations) - 🔄 IN PROGRESS (33% complete)
├─ bd-95pj: ESP300 → V3 MotionController [OPEN] - ✅ READY (Jules-10 completed MotionController trait)
├─ bd-l7vs: MaiTai + Newport → V3 [OPEN] - ✅ READY
└─ bd-e18h: PVCAM V3 Camera Trait Fix [OPEN] - ✅ READY

PHASE 3 (Data Layer Cleanup) - ⏳ READY TO START
├─ bd-op7v: Standardize Measurement Enum [OPEN] - ✅ READY
├─ bd-9cz0: Fix Trait Signature Mismatches [OPEN] - ✅ READY
├─ bd-rcxa: Arrow Batching in DataDistributor [OPEN] - ✅ READY
└─ bd-vkp3: HDF5 Storage + Arrow Integration [OPEN] - ✅ READY

PHASE 4 (Scripting Layer) - 🚧 CRITICAL DEPENDENCY CHAIN
└─ bd-hqy6: Define ScriptEngine Trait [OPEN] ⚠️ BLOCKER
    ├─→ BLOCKS: bd-svlx (Jules-11: PyO3 Backend)
    ├─→ BLOCKS: bd-dxqi (Jules-12: V3 API Bindings)
    ├─→ BLOCKS: bd-ya3l (Jules-12: Rhai/Lua Alternative)
    └─→ BLOCKS: bd-u7hu (Jules-13: Hot-Swappable Logic)
```

## Task Details with Dependencies

### PHASE 1: Infrastructure (✅ COMPLETE)

#### bd-r896: Kameo vs V3 Direct Async Performance Analysis
- **Status**: ✅ CLOSED
- **Agent**: Completed
- **Dependencies**: None
- **Blocks**: bd-o6c7
- **Priority**: P0 (completed)

#### bd-o6c7: Migrate V4 SCPI Actor to V3 Pattern
- **Status**: ✅ CLOSED
- **Agent**: Completed
- **Dependencies**: bd-r896
- **Blocks**: None
- **Priority**: P0 (completed)

#### bd-ifxt: Fix V3 Import Consolidation After Deletion
- **Status**: ✅ CLOSED
- **Agent**: Completed
- **Dependencies**: None
- **Blocks**: None
- **Priority**: P1 (completed)

---

### PHASE 2: V3 Instrument Migrations (33% complete)

#### bd-95pj: ESP300 → V3 MotionController [Jules-10 DEPENDENCY]
- **Status**: 🟢 READY TO START (MotionController trait NOW EXISTS)
- **Agent**: Jules-2 OR available worker
- **File**: `crates/rust-daq-app/src/instruments_v2/esp300_v3.rs` (new)
- **Reference**: `src/instruments_v2/newport_1830c_v3.rs` (1,067 lines)
- **Dependencies**:
  - ✅ **bd-hqy6 (Jules-10)**: MotionController trait EXISTS at `src/core_v3.rs:501-523`
  - Jules session 10407563664786836449 already completed trait definition
- **Blocks**: None
- **Estimated LOC**: 800-1,000
- **Priority**: P0
- **Test Strategy**: MockSerialDevice, 6+ unit tests
- **Notes**: **CRITICAL UPDATE - Jules-10 (bd-hqy6) completed MotionController trait!**
  - Location: `src/core_v3.rs:501-523`
  - PR: #58 submitted
  - This task is NOW UNBLOCKED and ready for parallel execution

#### bd-l7vs: MaiTai + Newport → V3 Traits
- **Status**: 🟢 READY TO START
- **Agent**: Jules-3 OR Jules-4 (can split into 2 sessions)
- **Files**:
  - `crates/rust-daq-app/src/instruments_v2/maitai_v3.rs` (new)
  - `crates/rust-daq-app/src/instruments_v2/newport_powermeter_v3.rs` (refactor existing)
- **Dependencies**: None (independent)
- **Blocks**: None
- **Estimated LOC**: 600-800 each
- **Priority**: P0
- **Capability Traits**:
  - MaiTai: LaserController (wavelength, power, shutter)
  - Newport: PowerMeter (already implemented in 1830C V3)
- **Notes**: Can be parallelized as 2 separate Jules sessions

#### bd-e18h: PVCAM V3 Camera Trait Fix
- **Status**: 🟢 READY TO START
- **Agent**: Jules-5
- **File**: `crates/rust-daq-app/src/instruments_v2/pvcam.rs` (existing)
- **Dependencies**: None
- **Blocks**: None
- **Estimated LOC**: 200-300 (fixes only)
- **Priority**: P0
- **Issues**: Camera trait signature mismatches, incomplete implementation
- **Reference**: Existing PVCAM V2 implementation

---

### PHASE 3: Data Layer Cleanup (0% complete, all READY)

#### bd-op7v: Standardize on core_v3::Measurement Enum
- **Status**: 🟢 READY TO START
- **Agent**: Jules-6
- **Files**:
  - `crates/daq-core/src/core_v3.rs` (Measurement enum)
  - Multiple files across `src/gui/`, `src/data/`
- **Dependencies**: None (independent cleanup)
- **Blocks**: None
- **Estimated LOC**: 400-600 (refactoring across multiple files)
- **Priority**: P1
- **Strategy**: Replace JSON metadata workarounds with proper Measurement enum usage

#### bd-9cz0: Fix Trait Signature Type Mismatches
- **Status**: 🟢 READY TO START
- **Agent**: Jules-7
- **Files**: Trait implementations across `src/instruments_v2/`
- **Dependencies**: None
- **Blocks**: None
- **Estimated LOC**: 150-250 (targeted fixes)
- **Priority**: P1
- **Strategy**: Align return types, async signatures, error handling

#### bd-rcxa: Arrow Batching in DataDistributor
- **Status**: 🟢 READY TO START
- **Agent**: Jules-8
- **File**: `crates/rust-daq-app/src/data/data_distributor.rs` (existing)
- **Dependencies**: None
- **Blocks**: bd-vkp3 (HDF5 storage needs Arrow batches)
- **Estimated LOC**: 300-400
- **Priority**: P1
- **Strategy**: Implement RecordBatch batching with configurable window

#### bd-vkp3: HDF5 Storage + Arrow Integration
- **Status**: ⏳ BLOCKED (waiting on Arrow batching)
- **Agent**: Jules-9
- **File**: `crates/rust-daq-app/src/data/storage/hdf5_writer.rs` (existing)
- **Dependencies**: bd-rcxa (needs Arrow batches from DataDistributor)
- **Blocks**: None
- **Estimated LOC**: 250-350
- **Priority**: P1
- **Strategy**: Accept RecordBatch instead of raw DataPoint streams

---

### PHASE 4: Scripting Layer (0% complete, CRITICAL BLOCKER)

#### bd-hqy6: Define ScriptEngine Trait [⚠️ CRITICAL BLOCKER]
- **Status**: 🟢 READY TO START (Jules-10 assignment)
- **Agent**: **Jules-10** (HIGHEST PRIORITY)
- **File**: `crates/daq-core/src/scripting.rs` (new)
- **Dependencies**: None
- **Blocks**:
  - bd-svlx (Jules-11: PyO3 backend)
  - bd-dxqi (Jules-12: V3 API bindings)
  - bd-ya3l (Jules-12: Rhai/Lua alternative)
  - bd-u7hu (Jules-13: Hot-swappable logic)
- **Estimated LOC**: 200-300 (trait + common types)
- **Priority**: P0 (UNBLOCKS 4 TASKS)
- **Strategy**:
  - Define `ScriptEngine` trait (execute, register_function, load_module)
  - Create `ScriptContext` for instrument/config access
  - Design error handling (ScriptError enum)
- **Impact**: Unblocks all scripting backend implementations
- **URGENT**: Must start immediately to unblock Jules-11, Jules-12, Jules-13

#### bd-svlx: PyO3 ScriptEngine Backend [Jules-11]
- **Status**: 🔴 BLOCKED (waiting on ScriptEngine trait from Jules-10)
- **Agent**: **Jules-11**
- **File**: `crates/rust-daq-app/src/scripting/pyo3_backend.rs` (new)
- **Dependencies**:
  - ⚠️ **bd-hqy6 (Jules-10)**: ScriptEngine trait MUST exist first
- **Blocks**: None
- **Estimated LOC**: 600-800
- **Priority**: P1
- **Strategy**: Implement ScriptEngine trait using PyO3 for Python scripts
- **Test Strategy**: Python test scripts, mock instruments

#### bd-dxqi: Expose V3 APIs to Python via PyO3 [Jules-12]
- **Status**: 🔴 BLOCKED (waiting on ScriptEngine trait from Jules-10)
- **Agent**: **Jules-12**
- **Files**:
  - `crates/rust-daq-app/src/scripting/python_bindings.rs` (new)
  - PyO3 class wrappers for V3 instruments
- **Dependencies**:
  - ⚠️ **bd-hqy6 (Jules-10)**: ScriptEngine trait
  - ⚠️ **bd-svlx (Jules-11)**: PyO3 backend
- **Blocks**: bd-u7hu
- **Estimated LOC**: 800-1,000
- **Priority**: P1
- **Strategy**: PyO3 class wrappers for InstrumentV3, CommandV3, Measurement

#### bd-ya3l: Alternative Scripting Backend (Rhai/Lua) [Jules-12 continuation]
- **Status**: 🔴 BLOCKED (waiting on ScriptEngine trait from Jules-10)
- **Agent**: **Jules-12** (after bd-dxqi) OR separate Jules-14
- **File**: `crates/rust-daq-app/src/scripting/rhai_backend.rs` (new)
- **Dependencies**:
  - ⚠️ **bd-hqy6 (Jules-10)**: ScriptEngine trait
- **Blocks**: None
- **Estimated LOC**: 500-700
- **Priority**: P2
- **Strategy**: Implement ScriptEngine trait using Rhai (Rust-embedded scripting)
- **Notes**: Can run parallel to Jules-11/12 if separate agent assigned

#### bd-u7hu: Hot-Swappable Logic via Embedded Scripting [Jules-13]
- **Status**: 🔴 BLOCKED (waiting on V3 API bindings from Jules-12)
- **Agent**: **Jules-13**
- **Files**:
  - `crates/rust-daq-app/src/scripting/hot_reload.rs` (new)
  - File watcher integration
- **Dependencies**:
  - ⚠️ **bd-hqy6 (Jules-10)**: ScriptEngine trait
  - ⚠️ **bd-dxqi (Jules-12)**: V3 API bindings
- **Blocks**: None
- **Estimated LOC**: 400-600
- **Priority**: P2
- **Strategy**: File watcher + script reload without app restart

---

## Critical Path Analysis

### Longest Dependency Chain (CRITICAL)

```
bd-hqy6 (Jules-10: ScriptEngine trait)
  → bd-svlx (Jules-11: PyO3 backend)
    → bd-dxqi (Jules-12: V3 API bindings)
      → bd-u7hu (Jules-13: Hot-swappable logic)
```

**Total Sequential Steps**: 4
**Estimated Total Time**: 8-12 days if done sequentially
**Optimization**: Jules-10 MUST start immediately to unblock Jules-11, Jules-12, Jules-13

### Parallelization Opportunities

**Phase 2 Migrations (ALL READY NOW)**:
- Jules-2: ESP300 V3 (bd-95pj) ✅ UNBLOCKED
- Jules-3: MaiTai V3 (bd-l7vs)
- Jules-4: Newport V3 (bd-l7vs continuation)
- Jules-5: PVCAM V3 Fix (bd-e18h)

**Phase 3 Cleanups (ALL READY)**:
- Jules-6: Measurement Enum (bd-op7v)
- Jules-7: Trait Signatures (bd-9cz0)
- Jules-8: Arrow Batching (bd-rcxa)
- Jules-9: HDF5 Integration (bd-vkp3) - starts after Jules-8

**Phase 4 Scripting (SEQUENTIAL)**:
- Jules-10: ScriptEngine trait (bd-hqy6) ⚠️ START NOW
- Jules-11: PyO3 backend (bd-svlx) - waits for Jules-10
- Jules-12: V3 bindings (bd-dxqi) - waits for Jules-11
- Jules-14: Rhai backend (bd-ya3l) - waits for Jules-10 (can parallel with Jules-11/12)
- Jules-13: Hot-reload (bd-u7hu) - waits for Jules-12

**Maximum Parallelization**: 9 tasks can run simultaneously (Jules-2 through Jules-9, Jules-14)

---

## Agent Status Tracking

### Ready to Start (9 tasks)

| Agent | Task ID | Title | Priority | Dependencies |
|-------|---------|-------|----------|--------------|
| Jules-2 | bd-95pj | ESP300 V3 Migration | P0 | ✅ MotionController trait exists (Jules-10 completed) |
| Jules-3 | bd-l7vs | MaiTai V3 Migration | P0 | None |
| Jules-4 | bd-l7vs | Newport V3 Refactor | P0 | None |
| Jules-5 | bd-e18h | PVCAM V3 Fix | P0 | None |
| Jules-6 | bd-op7v | Measurement Enum | P1 | None |
| Jules-7 | bd-9cz0 | Trait Signatures | P1 | None |
| Jules-8 | bd-rcxa | Arrow Batching | P1 | None |
| **Jules-10** | **bd-hqy6** | **ScriptEngine Trait** | **P0** | **None - START IMMEDIATELY** |
| Jules-14 | bd-ya3l | Rhai Backend | P2 | Jules-10 |

### Blocked (4 tasks)

| Agent | Task ID | Title | Blocked By | Status |
|-------|---------|-------|------------|--------|
| Jules-9 | bd-vkp3 | HDF5 + Arrow | Jules-8 (bd-rcxa) | Waiting on Arrow batching |
| Jules-11 | bd-svlx | PyO3 Backend | **Jules-10 (bd-hqy6)** | ⚠️ CRITICAL BLOCKER |
| Jules-12 | bd-dxqi | V3 API Bindings | Jules-10 + Jules-11 | ⚠️ CRITICAL BLOCKER |
| Jules-13 | bd-u7hu | Hot-Reload | Jules-12 (bd-dxqi) | ⚠️ CRITICAL BLOCKER |

---

## Escalation Points

### Deadlock Detection
- **No circular dependencies detected** ✅
- All dependencies are acyclic
- Critical path is linear and clear

### Current Bottlenecks
1. **Jules-10 (bd-hqy6)**: ScriptEngine trait NOT STARTED - blocks 4 tasks
2. **Jules-8 (bd-rcxa)**: Arrow batching - blocks HDF5 integration (Jules-9)

### Recommended Actions
1. **IMMEDIATE**: Assign Jules-10 to bd-hqy6 (ScriptEngine trait definition)
2. **PARALLEL**: Launch Jules-2 through Jules-8 on ready Phase 2/3 tasks
3. **MONITOR**: Jules-10 completion to unblock Jules-11, Jules-12, Jules-13
4. **NOTIFY**: Alert when Jules-10 completes to start Jules-11 immediately

---

## Progress Metrics

### Overall Progress: 27% (4/15 tasks complete)

**Phase 1**: ✅ 100% (3/3 complete)
**Phase 2**: 🔄 0% (0/3 complete, all ready)
**Phase 3**: ⏳ 0% (0/4 complete, 3 ready, 1 blocked)
**Phase 4**: 🚧 0% (0/5 complete, 2 ready, 3 blocked)

### Velocity Tracking
- **Completed**: 4 tasks (Phase 1 infrastructure)
- **In Progress**: 0 tasks
- **Ready**: 9 tasks (can start immediately)
- **Blocked**: 4 tasks (waiting on Jules-10 + Jules-8)

### Estimated Timeline
- **Phase 2 (Migrations)**: 1-2 weeks (parallel execution)
- **Phase 3 (Cleanups)**: 1 week (parallel execution)
- **Phase 4 (Scripting)**: 2-3 weeks (sequential with Jules-10 bottleneck)
- **Total Remaining**: 4-6 weeks with optimal parallelization

---

## Coordination Protocol

### Daily Check-ins
1. Check Jules session status: `jules remote list --session`
2. Update this document with agent progress
3. Identify newly ready tasks when blockers complete
4. Escalate stuck sessions (>24h in Planning state)

### Agent Completion Workflow
When any Jules agent completes:
1. Update agent status in this document
2. Check "Blocks" field to identify newly ready tasks
3. Notify blocked agents that dependencies are satisfied
4. Record learnings in ByteRover with file:line specifics
5. Update beads tracker: `bd update <task-id> --status closed`

### Critical Path Monitoring
**Jules-10 (bd-hqy6) is CRITICAL**:
- Monitor every 6-12 hours
- If stuck in Planning >12h, provide reference:
  - Trait design patterns from `src/core_v3.rs`
  - Error handling patterns from `src/error.rs`
  - Example: PyO3 trait design from existing Rust-Python FFI

### Escalation Triggers
1. **Jules-10 stuck >24h**: Assign to Codex via `mcp__zen__clink`
2. **3+ agents stuck in same phase**: Request Gemini architectural review
3. **Circular dependency detected**: IMMEDIATE escalation to Claude Code
4. **PR merge conflicts in critical path**: Manual intervention required

---

## Success Criteria

### Phase Completion Gates
- **Phase 2 Complete**: All 3 V3 migrations merged, instruments functional
- **Phase 3 Complete**: All 4 data layer cleanups merged, tests passing
- **Phase 4 Complete**: All 5 scripting features merged, Python/Rhai scripts working

### Quality Metrics
- **Test Coverage**: >85% for all new code
- **CI Pass Rate**: >90% across all PRs
- **Jules Completion Rate**: >70% (match historical 71%)
- **PR Review Time**: <24h from submission to merge

### Integration Points
- Phase 2 → Phase 3: V3 instruments produce correct Measurement enum data
- Phase 3 → Phase 4: Arrow batches flow to HDF5 storage and scripting layer
- Phase 4: Python scripts can control V3 instruments and process Arrow data

---

## Contact and Coordination

**Dependency Coordinator**: Jules-17
**Orchestrator**: Claude Code (you)
**Strategic Advisor**: Gemini (via `mcp__zen__clink`)
**Deep Implementer**: Codex (via `mcp__zen__clink` for complex tasks)

**Notification Channels**:
- Beads tracker: `bd update <id> --notes "Jules-X completed, Jules-Y now ready"`
- ByteRover: `brv add -s "Lessons Learned" -c "Jules-X completed bd-XXX, unblocked Jules-Y"`
- This document: Update agent status table after each completion

---

**Last Updated**: 2025-11-20 by Jules-17
**Next Review**: Check daily for Jules-10 progress (CRITICAL)
**Escalation Needed**: None (all blockers identified, no deadlocks)
