# Session Summary: Architectural Analysis and V2 Migration Planning

**Date**: 2025-11-03
**Session Focus**: Address bd-7e51, bd-de55, bd-9f85, bd-f301

## Work Completed

### 1. Beads Onboarding
- ✅ Completed `bd onboard` workflow
- ✅ Verified AGENTS.md and CLAUDE.md already contain beads documentation
- ✅ Confirmed understanding of beads workflow

### 2. Issue Analysis
Analyzed four interconnected issues:
- **bd-7e51** (P0): Architectural fragmentation V1/V2/V3
- **bd-de55** (P1): V2InstrumentAdapter performance bottleneck
- **bd-9f85** (P1): High coupling due to mixed architectures
- **bd-f301** (P2): Inconsistent error handling

### 3. Comprehensive Architectural Analysis
Created **`docs/ARCHITECTURAL_ANALYSIS_2025-11-03.md`** documenting:

- Three coexisting architectures (V1, V2, V3)
- Data loss in V2InstrumentAdapter
- Blocking operations in DaqApp
- Module boundary confusion
- Dependency relationships

**Key Findings**:
- **V1** (Legacy): 8 instruments in `src/instrument/`, synchronous API
- **V2** (Modern): 9 instruments in `src/instruments_v2/`, async API with daq-core crate
- **V3** (Premature): 7 *_v3.rs files mixing actor pattern before V2 complete
- **V2InstrumentAdapter**: Lossy conversion causing data loss (Images → statistics)
- **DaqApp**: Blocking compatibility layer negating async benefits

**Recommendation**: **HALT V3 development**, complete V2 migration, remove all compatibility layers.

### 4. V2 Migration Roadmap
Created **`docs/V2_MIGRATION_ROADMAP.md`** with detailed 4-phase plan:

#### Phase 1: Freeze V3, Stabilize V2 (Week 1-2)
- Revert *_v3.rs files to V2
- Create VISA V2 implementation
- Update helper modules for V2

#### Phase 2: Update Core Infrastructure (Week 2-3)
- Update InstrumentRegistry to accept V2 trait
- Update DaqManagerActor for V2 Measurement enum
- Update GUI for V2 measurements (Scalar/Image/Spectrum)
- Remove app.rs blocking layer

#### Phase 3: Remove Legacy Code (Week 3)
- Delete V1 instrument implementations
- Delete V1 trait definitions (src/core.rs)
- Delete V2InstrumentAdapter
- Delete V1 measurement types

#### Phase 4: Cleanup and Documentation (Week 4)
- Clean module structure
- Unify error handling (daq_core::DaqError)
- Increase test coverage to 80%+
- Update documentation

### 5. Issue Tracking in Beads

Created migration phase issues:
- **bd-42c4** (P0): Phase 1 - Freeze V3, stabilize V2
- **bd-555d** (P0): Phase 2 - Update core infrastructure
- **bd-09b9** (P1): Phase 3 - Remove V1 legacy code
- **bd-433d** (P1): Phase 4 - Cleanup and documentation

Created Phase 1 subtasks:
- **bd-cacd** (P0): Step 1.1 - Revert V3 files to V2
- **bd-20c4** (P1): Step 1.2 - Create VISA V2 implementation
- **bd-dbc1** (P1): Step 1.3 - Update helper modules for V2

Updated original issues with analysis notes:
- **bd-7e51**: Added link to analysis docs and phase issues
- **bd-de55**: Added V2InstrumentAdapter removal plan (Phase 3)
- **bd-9f85**: Added module refactoring plan (Phase 4.1)
- **bd-f301**: Added error handling unification plan (Phase 4.2)

## Key Insights

### Problem: Three Coexisting Architectures

```
GUI (egui)
    ↓ blocking_send() ⚠️
DaqApp (compatibility layer)
    ↓ blocking_send() ⚠️
DaqManagerActor (V3 actor)
    ↓
┌──────────────┬──────────────┬──────────────┐
│  V1 Legacy   │  V2 Modern   │  V3 Premature│
│              │ (w/ adapter) │              │
└──────────────┴──────────────┴──────────────┘
```

### Impact of V2InstrumentAdapter

**Before Adapter** (V2 Instrument produces):
```rust
Measurement::Image {
    pixels: PixelBuffer::U16(2048x2048),  // 8.4 MB
    width: 2048,
    height: 2048,
}
```

**After Adapter** (Converted to V1):
```rust
DataPoint {
    channel: "pvcam_mean",
    value: 1234.5,  // JUST THE MEAN! Image data LOST
}
```

**Result**: Image viewing in GUI impossible with V2 instruments until adapter removed.

### Solution Architecture

**Target state after migration**:
```
GUI (egui)
    ↓ async channels
DaqManagerActor (owns state)
    ↓ Arc<Measurement>
V2 Instruments (daq-core::Instrument)
    ↓ Measurement enum
    ├── Scalar(DataPoint)
    ├── Spectrum(SpectrumData)
    └── Image(ImageData with PixelBuffer)
```

Benefits:
- **No data loss**: Full Image/Spectrum data preserved
- **No blocking**: Async throughout
- **Single architecture**: One way to do things
- **Highly testable**: Actor pattern enables mocking
- **Clear boundaries**: daq-core (traits), instruments_v2/ (impl), app_actor (state), gui (view)

## Next Steps

### Immediate (This Week)
1. **Review documents** with team
2. **Approve migration plan** or suggest changes
3. **Create feature branch**: `git checkout -b v2-migration`
4. **Begin Phase 1.1**: Revert V3 files to V2 (bd-cacd)

### Phase 1 (Weeks 1-2)
- Revert all *_v3.rs files
- Create VISA V2 implementation
- Update helper modules
- All code compiles with V2

### Phase 2 (Weeks 2-3)
- Update InstrumentRegistry for V2
- Update DaqManagerActor for V2
- Update GUI for Measurement enum
- Remove blocking DaqApp layer

### Phase 3 (Week 3)
- Delete all V1 code
- Delete V2InstrumentAdapter
- Zero compatibility layers

### Phase 4 (Week 4)
- Clean architecture
- Unify error handling
- 80%+ test coverage
- Update documentation

## Files Created

1. **`docs/ARCHITECTURAL_ANALYSIS_2025-11-03.md`**
   - Comprehensive analysis of V1/V2/V3 fragmentation
   - Problem identification and impact
   - Recommended solution
   - 53KB detailed document

2. **`docs/V2_MIGRATION_ROADMAP.md`**
   - 4-phase migration plan
   - Step-by-step instructions
   - Testing strategy
   - Success criteria
   - 41KB detailed roadmap

3. **`docs/SESSION_SUMMARY_2025-11-03.md`**
   - This document
   - Session overview
   - Key findings
   - Next steps

## Beads Issues Created/Updated

### Created (7 new issues)
- bd-42c4: Phase 1 - Freeze V3, stabilize V2
- bd-555d: Phase 2 - Update core infrastructure
- bd-09b9: Phase 3 - Remove V1 legacy code
- bd-433d: Phase 4 - Cleanup and documentation
- bd-cacd: Step 1.1 - Revert V3 files to V2
- bd-20c4: Step 1.2 - Create VISA V2 implementation
- bd-dbc1: Step 1.3 - Update helper modules for V2

### Updated (4 existing issues)
- bd-7e51: Added analysis summary and phase links
- bd-de55: Added V2InstrumentAdapter removal plan
- bd-9f85: Added module refactoring plan
- bd-f301: Added error handling unification plan

## Dependency Graph

```
bd-7e51 (P0) - Architectural fragmentation
    ├── blocks → bd-de55 (P1) - V2InstrumentAdapter bottleneck
    ├── blocks → bd-9f85 (P1) - High coupling
    └── blocks → bd-f301 (P2) - Inconsistent error handling

bd-42c4 (P0) - Phase 1: Freeze V3
    ├── parent → bd-cacd (P0) - Step 1.1: Revert V3
    ├── parent → bd-20c4 (P1) - Step 1.2: VISA V2
    └── parent → bd-dbc1 (P1) - Step 1.3: Helpers V2
    ├── blocks → bd-de55 (P1)
    └── blocks → bd-9f85 (P1)

bd-555d (P0) - Phase 2: Core infrastructure
    ├── blocks → bd-de55 (P1)
    └── blocks → bd-9f85 (P1)

bd-09b9 (P1) - Phase 3: Remove V1
    └── blocks → bd-de55 (P1)

bd-433d (P1) - Phase 4: Cleanup
    └── blocks → bd-f301 (P2)
```

## Recommendations

### Critical Priority
1. **HALT all V3 development** - Do not create more *_v3.rs files
2. **Do not start new features** - Focus on migration
3. **Review migration plan** - Get team buy-in
4. **Allocate 1-2 developers** - Full-time for 4 weeks

### High Priority
1. Create feature branch for migration
2. Set up CI/CD for feature branch
3. Plan testing strategy
4. Communicate timeline to users

### Medium Priority
1. Gather test hardware if available
2. Document current system behavior
3. Create rollback plan
4. Update project status

## Risk Assessment

### High Risk (If NOT Fixed)
- Continued data loss in camera/spectrum instruments
- Performance degradation under load
- Maintenance nightmare with three APIs
- New contributors confused
- Technical debt compounds

### Medium Risk (During Migration)
- 4 weeks of development time
- Temporary instability
- Need thorough testing
- Configuration updates needed

### Mitigation Strategies
- Work in feature branch
- Incremental migration
- Extensive testing
- Keep old code until stable
- Feature flags if needed

## Success Metrics

Migration complete when:
- [ ] Zero V1 code in main branch
- [ ] Zero V3 code in main branch
- [ ] Zero compatibility/adapter layers
- [ ] Single Measurement enum throughout
- [ ] No blocking operations in async code
- [ ] 80%+ test coverage
- [ ] All tests passing
- [ ] Documentation updated
- [ ] Performance equal or better

## Conclusion

The rust-daq project has reached a **critical juncture**. Three coexisting architectures (V1, V2, V3) are causing:
- Data loss
- Performance issues
- High complexity
- Poor maintainability

The solution is clear but requires **focused effort**:
1. HALT V3 development
2. Complete V2 migration
3. Remove all compatibility layers
4. Establish clean architecture

**Estimated Timeline**: 4 weeks with 1-2 developers

**Expected Outcome**: Clean, performant, maintainable V2 architecture with no data loss, no blocking operations, and clear module boundaries.

## References

- Main analysis: `docs/ARCHITECTURAL_ANALYSIS_2025-11-03.md`
- Migration plan: `docs/V2_MIGRATION_ROADMAP.md`
- daq-core crate: `crates/daq-core/src/lib.rs`
- V1 traits: `src/core.rs`
- V2 adapter: `src/instrument/v2_adapter.rs`
- V2 instruments: `src/instruments_v2/`
- App actor: `src/app_actor.rs`

---

**End of Session Summary**
