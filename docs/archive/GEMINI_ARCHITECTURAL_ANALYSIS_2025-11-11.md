# Gemini Architectural Analysis - November 11, 2025

## Executive Summary

Gemini Codebase Investigator identified **critical architectural drift** in rust-daq project. The codebase has evolved into a complex hybrid V1/V2/V3 architecture that deviates from the original goal of creating a robust experiment framework similar to DynExp, PyMoDAQ, Qudi, and ScopeFoundry.

## Key Findings

### 1. Architectural Drift - Hybrid Architecture Complexity
- **Issue**: Coexistence of V1 (legacy actor), V2 (partial async), and V3 (modern async-first) patterns
- **Impact**: Performance bottlenecks, maintenance burden, unclear migration path
- **Evidence**: Multiple instrument trait definitions, dual data pipeline approaches

### 2. Actor Model Bottleneck (Critical)
- **Component**: `DaqManagerActor` in `src/app_actor.rs`
- **Problem**: Legacy actor-based orchestration is primary performance limiter
- **Status**: Being phased out but still deeply integrated
- **Recommendation**: Complete removal prioritized in migration plan

### 3. Incomplete V3 Migration
- **V3 Strengths**:
  - Well-designed async-first instrument traits (`src/core_v3.rs`)
  - Clean separation of concerns
  - Modern Tokio-based architecture
- **Gap**: Only partially adopted, hybrid patterns prevent full performance gains

### 4. Experiment Configuration System Gap
- **Current State**: TOML config in `src/config.rs` focuses on instrument parameters
- **Missing**:
  - Experiment template definitions
  - Workflow sequencing configuration
  - High-level orchestration primitives
- **Comparison**: PyMoDAQ's module system more mature for complex experiments

## Comparison to Reference Frameworks

| Feature | rust-daq V3 | DynExp | PyMoDAQ | Qudi | Assessment |
|---------|-------------|---------|----------|------|------------|
| Instrument Abstraction | ✅ Strong | ✅ | ✅ | ✅ | V3 traits competitive |
| Experiment Sequencing | ⚠️ Basic | ✅ | ✅ | ✅ | Primitives exist but underutilized |
| Configuration System | ⚠️ Instrument-focused | ✅ | ✅ | ✅ | Lacks experiment templates |
| Task Orchestration | ⚠️ Manual | ✅ Auto | ✅ | ✅ | No high-level workflow engine |
| Plugin Architecture | ⚠️ Planned | ✅ | ✅ | ✅ | V3 supports, not implemented |
| GUI Integration | ⚠️ Coupled to V1 | ✅ | ✅ | ✅ | Needs V3 decoupling |

## Critical Recommendations (Priority Order)

### 1. Complete V3 Migration (HIGHEST PRIORITY)
- **Action**: Follow plan in `docs/ARCHITECTURAL_REDESIGN_2025.md`
- **Target**: 100% V3 adoption for all new code
- **Benefit**: Unlock full async performance, simplify architecture

### 2. Remove Actor Model
- **Action**: Eliminate `DaqManagerActor` completely
- **Replace With**: Direct V3 `InstrumentManager` usage
- **Benefit**: Remove primary performance bottleneck

### 3. Extend Experiment Configuration
```toml
# Proposed addition to config system
[[experiments]]
name = "wavelength_scan"
type = "parameter_sweep"
instrument = "maitai_laser"
parameter = "wavelength"
start = 700.0
stop = 900.0
step = 10.0
duration_per_point = 1.0

[[experiments.measurements]]
instrument = "newport_1830c"
channel = 0
```

### 4. Refactor Data Pipeline to V3
- **Current**: Dual broadcast (V1 actor + V2 distributor)
- **Target**: Single V3 broadcast channel architecture
- **Benefit**: Reduced latency, simplified data flow

### 5. Decouple GUI from Legacy Actor
- **Action**: Move GUI to direct V3 instrument interaction
- **Benefit**: Improved responsiveness, cleaner separation

### 6. Implement Experiment Orchestration Layer
- **Missing Component**: High-level workflow engine
- **Reference**: DynExp's task system, PyMoDAQ's workflow module
- **Proposal**: `src/experiment/orchestrator.rs` with declarative workflow DSL

## Architecture Alignment Matrix

**Current Focus**: ❌ Over-emphasis on low-level instrument drivers
**Needed Shift**: ✅ High-level experiment workflow primitives

| Layer | Current State | Target State |
|-------|---------------|--------------|
| Experiment Workflows | Minimal | Rich template system |
| Instrument Coordination | Manual | Automated sync/triggers |
| Configuration | Instrument params | Experiment definitions |
| Data Processing | Per-instrument | Experiment-aware pipelines |

## Immediate Action Items

1. **Stop New V1/V2 Development**: All new features V3-only
2. **Document V3 Migration Path**: Clear guide for each subsystem
3. **Prototype Experiment Config**: Extend TOML with experiment section
4. **Benchmark V3 Performance**: Prove actor removal benefits
5. **GUI Refactor Spike**: Test V3-direct GUI integration

## Technical Debt Quantification

- **V1 Legacy Code**: ~30% of codebase (actor model, old traits)
- **V2 Transition Code**: ~20% (adapters, hybrid patterns)
- **V3 Modern Code**: ~40% (async instruments, new traits)
- **Experiment Framework**: ~10% (basic primitives)

**Target Allocation** (Post-Migration):
- V3 Modern: 70%
- Experiment Framework: 25%
- Legacy/Compat: 5%

## Conclusion

The rust-daq project has **strong technical foundations** in V3 architecture but suffers from **incomplete migration** and **insufficient experiment-level abstractions**. Following Gemini's recommendations will realign the project with original goals of creating a DynExp/PyMoDAQ-class experiment framework.

**Critical Success Factor**: Complete V3 migration before adding more features. Current hybrid state is primary blocker to both performance and feature development.

---

**Analysis Date**: 2025-11-11
**Analyzer**: Gemini 2.5 Pro (via Zen MCP)
**Session Duration**: 6.2 minutes
**Files Analyzed**: 19 core architecture files
**Token Usage**: 597,733 tokens (gemini-2.5-pro)
