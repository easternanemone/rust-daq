# Documentation Structure Audit - November 20, 2025

## Executive Summary

This audit was conducted to assess the current documentation state for the rust-daq project as part of the V5 architecture migration. The project has undergone multiple architectural iterations (V1→V2→V3→V4→V5), and documentation consistency is critical for maintaining development velocity.

## Current Documentation State

### Directory Structure

```
docs/
├── architecture/          # Architecture documentation (5 files)
│   ├── ADDITIONAL_LIBRARY_RESEARCH.md
│   ├── ARCHITECTURAL_FLAW_ANALYSIS.md
│   ├── hdf5_actor_design.md
│   ├── RUST_LIBRARY_RECOMMENDATIONS.md
│   └── V5_OPTIMIZATION_STRATEGIES.md
├── archive/              # Archived/deprecated docs
├── examples/             # Example scripts and code (3 files)
│   ├── HARDWARE_DRIVERS_EXAMPLE.md
│   ├── phase4_ring_buffer_example.rs
│   └── verify_hdf5_output.py
├── headless/             # Headless architecture docs (2 files)
│   └── phase2_scripting_engine.md
├── guides/               # User guides
│   ├── ci_cd/
│   ├── testing/
│   └── deployment/
├── instruments/          # Instrument-specific docs
├── project_management/   # Project management docs
│   └── agents/
├── pvcam-sdk/           # External SDK documentation
├── v4/                  # V4-specific documentation
├── external/            # External documentation
│   └── python/
└── reports/             # Status reports
```

### Key Findings

#### 1. Architecture Documentation Confusion

**Issue**: Multiple architecture versions referenced without clear migration paths

- **README.md** claims "V4 Architecture" but references outdated Kameo actor model
- **ARCHITECTURE.md** describes V4 with Kameo/Arrow/HDF5
- **V5_OPTIMIZATION_STRATEGIES.md** correctly describes current V5 Headless-First architecture
- No comprehensive V5_ARCHITECTURE.md exists

**Documents Found**:
- V3-related: 11 files (mostly in archive/)
- V4-related: Multiple files in docs/v4/ and root
- V5-related: Only V5_OPTIMIZATION_STRATEGIES.md and scattered status reports

#### 2. Missing Documentation Directories

**ScriptEngine Documentation**:
- No dedicated `docs/scripting/` directory
- ScriptEngine docs scattered across:
  - `docs/headless/phase2_scripting_engine.md`
  - `docs/task-d-scripting-completion.md`
  - Examples in README.md

**Arrow Batching Architecture**:
- No dedicated documentation for Arrow data pipeline
- Mentioned in V5_OPTIMIZATION_STRATEGIES.md but not detailed
- Critical for understanding data flow in V5

#### 3. Hardware Documentation Status

**HARDWARE_INVENTORY.md**:
- Location: `/Users/briansquires/code/rust-daq/docs/HARDWARE_INVENTORY.md`
- Last updated: November 20, 2025
- Status: Needs verification against current driver implementations

**Hardware Communication Reference**:
- `/Users/briansquires/code/rust-daq/docs/HARDWARE_COMMUNICATION_REFERENCE.md`
- Comprehensive 29KB reference for all devices
- Recently updated (November 19, 2025)

**Hardware Examples**:
- `/Users/briansquires/code/rust-daq/docs/examples/HARDWARE_DRIVERS_EXAMPLE.md`
- Recent addition showing V5 capability trait implementation

#### 4. V3 Migration Status

**V3-Related Files** (Need consolidation):
1. `docs/PVCAM_V3_GEMINI_REVIEW.md`
2. `docs/PVCAM_V3_COMPLETION.md`
3. `docs/V3_TO_V2_MERGE_ANALYSIS.md`
4. `docs/archive/PVCAM_V3_GEMINI_REVIEW.md` (duplicate)
5. `docs/archive/PVCAM_V3_COMPLETION.md` (duplicate)
6. `docs/archive/ESP300_V3_CODE_REVIEW.md`
7. `docs/archive/ELLIPTEC_V3_COMPLETION.md`
8. `docs/archive/NEWPORT_1830C_V3_COMPLETION.md`
9. `docs/archive/SCPI_V3_COMPLETION.md`
10. `docs/archive/V3_TO_V2_MERGE_ANALYSIS.md` (duplicate)
11. Plus SDK references in pvcam-sdk/

**Action Required**: Create migration guide explaining V2/V3/V4→V5 path

#### 5. Root-Level Documentation Clutter

**Files in Project Root** (should be in docs/):
- `KAMEO_INTEGRATION_PLAN.md`
- `HDF5_QUICK_REFERENCE.md`
- `V4_GUI_IMPLEMENTATION_SUMMARY.md`
- `V4_GUI_INDEX.md`
- `AGENT_DEFENITIONS.md` (typo: should be DEFINITIONS)
- `ARCHITECTURE.md` (outdated)
- `IMPLEMENTATION_SUMMARY.md`
- `CHANGELOG.md` (appropriate for root)
- `PHASE1_SUMMARY.md`

**Note**: Per project guidelines, working files should NOT be saved to root folder

#### 6. Documentation Inconsistencies

**README.md Issues**:
- Claims "V4 Architecture Overview" (line 13)
- References Kameo actors (deprecated in V5)
- Missing V5 Headless-First architecture description
- Missing ScriptEngine capabilities overview
- Example scripts show Rhai but architecture description is V4

**ARCHITECTURE.md Issues**:
- Titled "V4 System Architecture"
- Describes Kameo actor model (not used in V5)
- No mention of Capability Traits (core V5 pattern)
- No mention of Headless-First design
- No mention of gRPC remote control

## Recommendations

### Immediate Actions

1. **Create V5_ARCHITECTURE.md** in `docs/architecture/`
   - Comprehensive V5 architecture guide
   - Headless-First design principles
   - Capability Trait system
   - gRPC API architecture
   - Arrow/HDF5 data pipeline
   - ScriptEngine integration

2. **Create docs/scripting/** directory with:
   - `SCRIPTING_OVERVIEW.md` - High-level ScriptEngine concepts
   - `RHAI_API_REFERENCE.md` - Complete API reference for Rhai scripts
   - `SCRIPTING_EXAMPLES.md` - Common patterns and examples
   - `ASYNC_BRIDGE_GUIDE.md` - How sync Rhai calls async Rust

3. **Create docs/architecture/DATA_PIPELINE.md**
   - Arrow RecordBatch architecture
   - FrameProducer trait details
   - Zero-copy strategies
   - HDF5 integration via FFI
   - Network serialization (gRPC)

4. **Create Migration Guide** in `docs/architecture/MIGRATION_GUIDE.md`
   - V2→V3 changes and lessons learned
   - V3→V4 changes and lessons learned
   - V4→V5 changes (current migration)
   - Clear explanation of what was deprecated and why
   - Forward compatibility guidelines

### Organization Tasks

5. **Consolidate V3 Documentation**
   - Move all V3-specific docs to `docs/archive/v3/`
   - Remove duplicates between docs/ and docs/archive/
   - Create `docs/archive/v3/README.md` explaining V3 deprecation

6. **Update Top-Level Files**
   - Update README.md to accurately reflect V5 architecture
   - Update ARCHITECTURE.md or replace with link to V5_ARCHITECTURE.md
   - Move root-level docs to appropriate subdirectories
   - Fix typo: AGENT_DEFENITIONS.md → AGENT_DEFINITIONS.md

7. **Update HARDWARE_INVENTORY.md**
   - Verify against current driver implementations
   - Add V5 capability trait information
   - Cross-reference with HARDWARE_COMMUNICATION_REFERENCE.md
   - Note which drivers are V5-ready vs legacy

8. **Create Inline Documentation Standards**
   - Document requirement for inline docs in CONTRIBUTING.md
   - Create examples of well-documented V5 drivers
   - Add rustdoc examples for all public traits

### Directory Structure Changes

**Proposed New Structure**:

```
docs/
├── architecture/
│   ├── V5_ARCHITECTURE.md (NEW - comprehensive)
│   ├── DATA_PIPELINE.md (NEW - Arrow/HDF5/gRPC)
│   ├── MIGRATION_GUIDE.md (NEW - V2/V3/V4→V5)
│   ├── ARCHITECTURAL_FLAW_ANALYSIS.md (existing)
│   ├── RUST_LIBRARY_RECOMMENDATIONS.md (existing)
│   ├── V5_OPTIMIZATION_STRATEGIES.md (existing)
│   └── hdf5_actor_design.md (existing)
├── scripting/ (NEW)
│   ├── SCRIPTING_OVERVIEW.md
│   ├── RHAI_API_REFERENCE.md
│   ├── SCRIPTING_EXAMPLES.md
│   └── ASYNC_BRIDGE_GUIDE.md
├── hardware/
│   ├── HARDWARE_INVENTORY.md (moved from docs/)
│   ├── HARDWARE_COMMUNICATION_REFERENCE.md (existing)
│   ├── DRIVER_IMPLEMENTATION_GUIDE.md (NEW)
│   └── examples/ (link to docs/examples/HARDWARE_DRIVERS_EXAMPLE.md)
├── archive/
│   ├── v3/ (NEW - consolidate V3 docs here)
│   │   ├── README.md (explain V3 deprecation)
│   │   ├── PVCAM_V3_COMPLETION.md
│   │   ├── ESP300_V3_CODE_REVIEW.md
│   │   ├── ELLIPTEC_V3_COMPLETION.md
│   │   ├── NEWPORT_1830C_V3_COMPLETION.md
│   │   ├── SCPI_V3_COMPLETION.md
│   │   └── V3_TO_V2_MERGE_ANALYSIS.md
│   └── v4/ (existing, may need cleanup)
├── examples/
│   ├── HARDWARE_DRIVERS_EXAMPLE.md (existing)
│   ├── phase4_ring_buffer_example.rs (existing)
│   └── verify_hdf5_output.py (existing)
├── guides/
│   ├── getting_started/
│   ├── ci_cd/
│   ├── testing/
│   └── deployment/
└── reports/ (status reports, keep recent only)
```

## Priority Matrix

| Task | Priority | Impact | Effort |
|------|----------|--------|--------|
| Create V5_ARCHITECTURE.md | P0 | High | High |
| Update README.md for V5 | P0 | High | Medium |
| Create docs/scripting/ with API docs | P0 | High | High |
| Create MIGRATION_GUIDE.md | P1 | Medium | High |
| Consolidate V3 docs to archive/v3/ | P1 | Medium | Low |
| Create DATA_PIPELINE.md | P1 | High | Medium |
| Update HARDWARE_INVENTORY.md | P2 | Medium | Low |
| Move root-level docs to docs/ | P2 | Low | Low |
| Create inline doc standards | P2 | Medium | Medium |

## Documentation Gaps Identified

### Critical Gaps
1. **No comprehensive V5 architecture document**
2. **No ScriptEngine user documentation**
3. **No migration guide for V2/V3/V4→V5**
4. **No Arrow data pipeline documentation**

### Important Gaps
5. **No driver implementation guide for V5 capability traits**
6. **No gRPC API documentation**
7. **No client-side architecture documentation**
8. **Outdated README.md and ARCHITECTURE.md**

### Minor Gaps
9. **No contributing guidelines for documentation**
10. **No rustdoc examples in source code**
11. **Inconsistent file organization**

## Metrics

- **Total markdown files**: 80+ (excluding node_modules, .git)
- **Root-level docs that should move**: 8 files
- **V3-related duplicates**: 3 pairs
- **Architecture docs**: 5 files (1 current, 4 legacy/support)
- **Missing critical docs**: 4 (V5_ARCHITECTURE, MIGRATION_GUIDE, scripting/, DATA_PIPELINE)

## Conclusion

The documentation is in a **transitional state** reflecting the ongoing V4→V5 migration. The most critical issue is the lack of authoritative V5 architecture documentation, causing confusion about current vs legacy patterns.

**Recommended First Steps**:
1. Create V5_ARCHITECTURE.md immediately
2. Update README.md to reflect V5 reality
3. Create docs/scripting/ directory with ScriptEngine docs
4. Archive V3 docs with clear deprecation notice

**Timeline Estimate**:
- P0 tasks: 2-3 days (V5_ARCHITECTURE, README, scripting/)
- P1 tasks: 2-3 days (MIGRATION_GUIDE, DATA_PIPELINE, archive cleanup)
- P2 tasks: 1-2 days (HARDWARE_INVENTORY update, file moves, standards)

**Total effort**: 5-8 days for complete documentation reorganization

## Next Steps

1. Share this audit with development team
2. Get approval for proposed structure changes
3. Begin P0 documentation creation
4. Schedule documentation review meeting
5. Update project management board with documentation tasks
