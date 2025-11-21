# Jules Integration Testing Checklist
## Jules-20 Integration Coordinator

**Purpose**: Comprehensive testing checklist for Jules multi-branch integration
**Usage**: Check off items as integration progresses through each phase

---

## Pre-Integration Verification

### Repository State
- [ ] Main branch is clean (no uncommitted changes)
- [ ] All Jules branches identified (14 branches total)
- [ ] Dependency map reviewed and understood
- [ ] Integration plan document created
- [ ] Beads tracker synced with latest status

### Environment Setup
- [ ] Rust toolchain up to date (`rustup update`)
- [ ] All system dependencies installed (HDF5, libusb, etc.)
- [ ] cargo-nextest installed (optional but recommended)
- [ ] recurse.ml CLI available (`~/.rml/rml/rml --version`)
- [ ] Sufficient disk space (>5GB for build artifacts)

### Branch Preparation
- [ ] All Jules branches rebased on main
- [ ] No merge conflicts remaining in individual branches
- [ ] Each branch builds independently (`cargo check`)
- [ ] Integration branch created (`jules-integration`)
- [ ] Rollback tag created (`jules-integration-start`)

---

## Phase 1: Infrastructure (Already Complete ✅)

**Branch**: jules-1/fix-v3-imports (bd-ifxt)

- [x] Already merged to main at commit `44c6fff0`
- [x] V3 import consolidation verified
- [x] No action required

**Phase 1 Status**: ✅ Complete

---

## Phase 2: V3 Instrument Migrations

### Jules-2: ESP300 V3 Migration (bd-95pj)

**Pre-Merge Checks**:
- [ ] Branch rebased on main
- [ ] `cargo check` passes
- [ ] MotionController trait present in `src/core_v3.rs`
- [ ] ESP300 V3 implementation complete

**Merge Execution**:
- [ ] Merged into integration branch
- [ ] No conflicts or conflicts resolved
- [ ] Build check after merge (`cargo check`)

**Post-Merge Verification**:
- [ ] ESP300 V3 tests passing (`cargo test esp300`)
- [ ] MockSerialDevice tests working
- [ ] MotionController trait implemented correctly
- [ ] Config registration working
- [ ] No clippy warnings for ESP300 files

### Jules-3: MaiTai + Newport V3 (bd-l7vs)

**Pre-Merge Checks**:
- [ ] Branch rebased on main
- [ ] `cargo check` passes
- [ ] MaiTai V3 implementation complete
- [ ] Newport 1830-C V3 implementation complete
- [ ] **Conflict check**: Verify no PVCAM changes (that's Jules-4)

**Merge Execution**:
- [ ] Merged into integration branch
- [ ] **CRITICAL**: Resolve PVCAM conflict with Jules-4 (keep Jules-4 changes)
- [ ] Build check after merge

**Post-Merge Verification**:
- [ ] MaiTai V3 tests passing (`cargo test maitai`)
- [ ] Newport V3 tests passing (`cargo test newport`)
- [ ] LaserController trait implemented
- [ ] PowerMeter trait implemented
- [ ] Config registration working
- [ ] No clippy warnings

### Jules-4: PVCAM V3 Camera Fix (bd-e18h)

**Pre-Merge Checks**:
- [ ] Branch rebased on main
- [ ] `cargo check` passes
- [ ] Camera trait fixes complete
- [ ] PVCAM V3 implementation correct

**Merge Execution**:
- [ ] Merged into integration branch
- [ ] **CRITICAL**: This has priority over Jules-3 PVCAM changes
- [ ] Build check after merge

**Post-Merge Verification**:
- [ ] PVCAM V3 tests passing (`cargo test pvcam`)
- [ ] Camera trait signature correct
- [ ] Frame acquisition working
- [ ] PixelBuffer support functional
- [ ] Config registration working
- [ ] No clippy warnings

### Phase 2 Checkpoint

**Build Verification**:
- [ ] `cargo clean && cargo build --all-features` succeeds
- [ ] All instrument V3 tests passing (`cargo test instrument::`)
- [ ] InstrumentManagerV3 registration complete
- [ ] Mock instruments working

**Tag Checkpoint**:
- [ ] Git tag created: `jules-integration-phase2`
- [ ] Tag pushed to remote

**Phase 2 Status**: ⏳ Pending

---

## Phase 3: Data Layer Cleanup

### Jules-5: Standardize Measurement Enum (bd-op7v)

**Pre-Merge Checks**:
- [ ] Branch rebased on main
- [ ] `cargo check` passes
- [ ] core_v3::Measurement enum usage standardized
- [ ] JSON metadata workarounds removed

**Merge Execution**:
- [ ] Merged into integration branch
- [ ] Build check after merge

**Post-Merge Verification**:
- [ ] Measurement enum tests passing
- [ ] All instruments use core_v3::Measurement
- [ ] No legacy InstrumentMeasurement usage
- [ ] Data serialization working
- [ ] No clippy warnings

### Jules-6: Fix Trait Signatures (bd-9cz0)

**Pre-Merge Checks**:
- [ ] Branch rebased on main
- [ ] `cargo check` passes
- [ ] Trait signature mismatches identified
- [ ] Return types aligned

**Merge Execution**:
- [ ] Merged into integration branch
- [ ] Build check after merge

**Post-Merge Verification**:
- [ ] All trait implementations compile
- [ ] Async signatures consistent
- [ ] Error handling aligned (DaqError)
- [ ] No trait orphan rule violations
- [ ] No clippy warnings

### Jules-7: Arrow Batching (bd-rcxa) ⚠️ CRITICAL

**Pre-Merge Checks**:
- [ ] Branch rebased on main
- [ ] `cargo check` passes
- [ ] Arrow RecordBatch batching implemented
- [ ] DataDistributor updated

**Merge Execution**:
- [ ] Merged into integration branch
- [ ] Build check after merge

**Post-Merge Verification**:
- [ ] Arrow batching tests passing (`cargo test arrow`)
- [ ] RecordBatch generation working
- [ ] Configurable batch window functional
- [ ] DataDistributor broadcasting correctly
- [ ] **CRITICAL**: Ready for Jules-9 dependency
- [ ] No clippy warnings

**Checkpoint Before Jules-9**:
- [ ] Jules-7 fully tested and verified
- [ ] Arrow feature flag working (`--features storage_arrow`)
- [ ] Git tag: `jules-integration-arrow-ready`

### Jules-9: HDF5 + Arrow Integration (bd-vkp3)

**Pre-Merge Checks**:
- [ ] **DEPENDENCY**: Jules-7 merged and verified ✅
- [ ] Branch rebased on integration branch (includes Jules-7)
- [ ] `cargo check` passes
- [ ] HDF5 writer accepts RecordBatch

**Merge Execution**:
- [ ] Merged into integration branch
- [ ] Build check after merge

**Post-Merge Verification**:
- [ ] HDF5 + Arrow tests passing (`cargo test --features storage_hdf5,storage_arrow`)
- [ ] RecordBatch → HDF5 conversion working
- [ ] File format correct and readable
- [ ] Storage writer integration complete
- [ ] No clippy warnings

### Phase 3 Checkpoint

**Build Verification**:
- [ ] `cargo clean && cargo build --all-features` succeeds
- [ ] All data layer tests passing (`cargo test data::`)
- [ ] Measurement enum standardized across codebase
- [ ] Arrow + HDF5 integration functional

**Functional Testing**:
- [ ] Run data pipeline integration test
- [ ] Verify HDF5 file generation
- [ ] Verify Arrow RecordBatch batching
- [ ] Check memory usage (no leaks)

**Tag Checkpoint**:
- [ ] Git tag created: `jules-integration-phase3`
- [ ] Tag pushed to remote

**Phase 3 Status**: ⏳ Pending

---

## Phase 4: Scripting Layer (STRICTLY SEQUENTIAL)

### Jules-10: ScriptEngine Trait (bd-hqy6) ⚠️ BLOCKER

**Pre-Merge Checks**:
- [ ] Branch rebased on main
- [ ] `cargo check` passes
- [ ] ScriptEngine trait defined
- [ ] ScriptContext created
- [ ] ScriptError enum defined

**Merge Execution**:
- [ ] Merged into integration branch
- [ ] Build check after merge

**Post-Merge Verification**:
- [ ] ScriptEngine trait compiles
- [ ] Trait methods well-defined (execute, register_function, load_module)
- [ ] ScriptContext accessible
- [ ] ScriptError variants comprehensive
- [ ] **CRITICAL**: Trait ready for Jules-11/14 implementations
- [ ] No clippy warnings

**Checkpoint: Scripting Unblocked**:
- [ ] Jules-10 fully tested and verified
- [ ] Git tag: `jules-integration-script-trait`
- [ ] **Ready to proceed with Jules-11/14 (parallel) and Jules-12/13 (sequential)**

### Jules-11: PyO3 ScriptEngine Backend (bd-svlx)

**Pre-Merge Checks**:
- [ ] **DEPENDENCY**: Jules-10 merged and verified ✅
- [ ] Branch rebased on integration branch (includes Jules-10)
- [ ] `cargo check` passes
- [ ] PyO3 backend implements ScriptEngine trait

**Merge Execution**:
- [ ] Merged into integration branch
- [ ] Build check after merge

**Post-Merge Verification**:
- [ ] PyO3 engine tests passing (`cargo test pyo3`)
- [ ] ScriptEngine trait implemented correctly
- [ ] Python script execution working
- [ ] Function registration functional
- [ ] Module loading working
- [ ] No clippy warnings

### Jules-14: Rhai Backend (bd-ya3l) (Parallel with Jules-11)

**Pre-Merge Checks**:
- [ ] **DEPENDENCY**: Jules-10 merged and verified ✅
- [ ] Branch rebased on integration branch (includes Jules-10)
- [ ] `cargo check` passes
- [ ] Rhai backend implements ScriptEngine trait

**Merge Execution**:
- [ ] Merged into integration branch
- [ ] Build check after merge

**Post-Merge Verification**:
- [ ] Rhai engine tests passing (`cargo test rhai`)
- [ ] ScriptEngine trait implemented correctly
- [ ] Rhai script execution working
- [ ] Function registration functional
- [ ] Module loading working
- [ ] No clippy warnings

**Checkpoint: Scripting Engines Ready**:
- [ ] Both PyO3 and Rhai engines functional
- [ ] Git tag: `jules-integration-script-engines`

### Jules-12: script_runner CLI (bd-6huu)

**Pre-Merge Checks**:
- [ ] **DEPENDENCIES**: Jules-10 ✅ + Jules-11 ✅
- [ ] Branch rebased on integration branch
- [ ] `cargo check` passes
- [ ] CLI binary compiles

**Merge Execution**:
- [ ] Merged into integration branch
- [ ] Build check after merge

**Post-Merge Verification**:
- [ ] CLI compiles (`cargo build --bin script_runner`)
- [ ] Python script execution via CLI works
- [ ] Rhai script execution via CLI works
- [ ] Command-line argument parsing correct
- [ ] Help text comprehensive
- [ ] No clippy warnings

### Jules-13: PyO3 V3 Bindings (bd-dxqi)

**Pre-Merge Checks**:
- [ ] **DEPENDENCIES**: Jules-10 ✅ + Jules-11 ✅
- [ ] Branch rebased on integration branch
- [ ] `cargo check` passes
- [ ] PyO3 class wrappers for V3 APIs complete

**Merge Execution**:
- [ ] Merged into integration branch
- [ ] Build check after merge

**Post-Merge Verification**:
- [ ] Python bindings compile
- [ ] V3 instruments accessible from Python
- [ ] CommandV3 callable from Python
- [ ] Measurement enum available in Python
- [ ] Python tests passing (`pytest` if present)
- [ ] Type stubs generated (`.pyi` files)
- [ ] No clippy warnings

### Phase 4 Checkpoint

**Build Verification**:
- [ ] `cargo clean && cargo build --all-features` succeeds
- [ ] All scripting tests passing (`cargo test scripting::`)
- [ ] Both PyO3 and Rhai engines functional
- [ ] CLI binary working

**Functional Testing**:
- [ ] Run Python example script
- [ ] Run Rhai example script
- [ ] Test hot-reload (if implemented)
- [ ] Test V3 instrument control from Python
- [ ] Verify ScriptContext access

**Tag Checkpoint**:
- [ ] Git tag created: `jules-integration-phase4`
- [ ] Tag pushed to remote

**Phase 4 Status**: ⏳ Pending

---

## Final Integration Verification

### Build Checks
- [ ] `cargo clean` to clear all artifacts
- [ ] `cargo build` (debug) succeeds
- [ ] `cargo build --release` succeeds
- [ ] `cargo build --all-features` succeeds
- [ ] `cargo build --no-default-features` succeeds
- [ ] Build time reasonable (<10 minutes on modern hardware)
- [ ] Binary sizes acceptable

### Test Suite
- [ ] `cargo test` (all tests) passing
- [ ] `cargo test --all-features` passing
- [ ] `cargo test --features storage_hdf5` passing
- [ ] `cargo test --features storage_arrow` passing
- [ ] `cargo test instrument::` passing
- [ ] `cargo test data::` passing
- [ ] `cargo test scripting::` passing
- [ ] Integration tests passing (`cargo test --test integration_test`)
- [ ] Test coverage ≥85% (if measured)

### Code Quality
- [ ] `cargo clippy --all-features` no warnings
- [ ] `cargo clippy --all-features -- -D warnings` passes (treats warnings as errors)
- [ ] `cargo fmt --check` passes (code formatted)
- [ ] `~/.rml/rml/rml` analysis clean (no critical issues)
- [ ] No TODO comments in critical paths
- [ ] No println! or dbg! in production code

### Documentation
- [ ] All public APIs documented
- [ ] Examples compile and run
- [ ] README updated with new features
- [ ] CHANGELOG updated
- [ ] Integration report generated

### Feature Flags
- [ ] Default features build (`cargo build`)
- [ ] Full features build (`cargo build --features full`)
- [ ] Individual features work (`--features storage_hdf5`, etc.)
- [ ] No feature build works (`cargo build --no-default-features`)

### Platform Testing (if applicable)
- [ ] macOS build succeeds
- [ ] Linux build succeeds (if CI available)
- [ ] Windows build succeeds (if CI available)

---

## PR Creation and Review

### PR Preparation
- [ ] Integration branch pushed to remote
- [ ] All commits follow conventional commit format
- [ ] Commit messages descriptive
- [ ] No merge commits (use rebase workflow)

### PR Content
- [ ] PR title descriptive: "feat: Jules multi-branch integration (14 branches)"
- [ ] PR description comprehensive (see integration plan template)
- [ ] All phases listed with checkmarks
- [ ] Files changed summary included
- [ ] Testing summary included
- [ ] Breaking changes noted (if any)

### CI Checks
- [ ] All GitHub Actions checks passing
- [ ] Clippy check passing
- [ ] Format check passing
- [ ] Test check passing
- [ ] Build check passing (all platforms)

### Review Process
- [ ] Self-review completed
- [ ] No debug code left in
- [ ] No commented-out code
- [ ] All TODOs addressed or tracked in beads
- [ ] Performance regression checked (if applicable)

---

## Merge to Main

### Pre-Merge Final Checks
- [ ] All CI checks green ✅
- [ ] All review comments addressed
- [ ] No conflicts with main (rebase if needed)
- [ ] Beads tracker prepared for bulk update
- [ ] ByteRover ready for learnings

### Merge Execution
- [ ] Squash merge to main (preserves clean history)
- [ ] Merge commit message comprehensive
- [ ] Integration branch deleted after merge
- [ ] Remote integration branch deleted

### Post-Merge Verification
- [ ] Main branch builds (`cargo build --all-features`)
- [ ] Main branch tests pass (`cargo test --all-features`)
- [ ] No regressions introduced
- [ ] CI passing on main

---

## Post-Integration Cleanup

### Beads Tracker Update
- [ ] bd-95pj closed (ESP300 V3)
- [ ] bd-l7vs closed (MaiTai + Newport V3)
- [ ] bd-e18h closed (PVCAM V3)
- [ ] bd-op7v closed (Measurement enum)
- [ ] bd-9cz0 closed (Trait signatures)
- [ ] bd-rcxa closed (Arrow batching)
- [ ] bd-vkp3 closed (HDF5 + Arrow)
- [ ] bd-hqy6 closed (ScriptEngine trait)
- [ ] bd-svlx closed (PyO3 backend)
- [ ] bd-ya3l closed (Rhai backend)
- [ ] bd-6huu closed (script_runner CLI)
- [ ] bd-dxqi closed (V3 Python bindings)
- [ ] Beads updates committed to `.beads/issues.jsonl`

### ByteRover Knowledge Capture
- [ ] Integration strategy recorded
- [ ] Conflict resolution patterns recorded
- [ ] Performance learnings recorded
- [ ] Testing insights recorded
- [ ] Pushed to shared memory (`brv push -y`)

### Branch Cleanup
- [ ] jules-2 deleted locally
- [ ] jules-3 deleted locally
- [ ] jules-4 deleted locally
- [ ] jules-5 deleted locally
- [ ] jules-6 deleted locally
- [ ] jules-7 deleted locally
- [ ] jules-9 deleted locally
- [ ] jules-10 deleted locally
- [ ] jules-11 deleted locally
- [ ] jules-12 deleted locally
- [ ] jules-13 deleted locally
- [ ] jules-14 deleted locally
- [ ] jules-integration deleted locally
- [ ] All remote Jules branches deleted (optional)

### Documentation Updates
- [ ] Integration report generated
- [ ] Integration plan marked complete
- [ ] Project README updated
- [ ] CHANGELOG updated with version bump
- [ ] Status document updated

---

## Success Criteria Final Check

### Quantitative Metrics
- [ ] **Build Success**: 100% (all feature combinations)
- [ ] **Test Pass Rate**: 100% (all tests passing)
- [ ] **Test Coverage**: ≥85% (target)
- [ ] **CI Pass Rate**: 100% (all checks green)
- [ ] **Beads Closure Rate**: 100% (13/13 issues closed)
- [ ] **Integration Time**: <4 hours (target)

### Qualitative Metrics
- [ ] **Code Quality**: No clippy warnings, recurse.ml clean
- [ ] **Documentation**: Complete and up-to-date
- [ ] **Knowledge Transfer**: ByteRover learnings recorded
- [ ] **Rollback Safety**: All checkpoint tags created
- [ ] **Team Readiness**: Clear handoff, no blockers

---

## Integration Complete ✅

**Final Sign-Off**:
- [ ] All phases merged successfully
- [ ] All tests passing
- [ ] All beads issues closed
- [ ] All documentation updated
- [ ] All cleanup complete
- [ ] Integration report generated

**Integration Status**: ⏳ In Progress

**Coordinator**: Jules-20
**Date Started**: _____________
**Date Completed**: _____________
**Total Duration**: _____ hours

---

🤖 Checklist created by Jules-20 (Integration Coordinator)
📅 2025-11-20

**Usage**: Check off items as integration progresses. Update status at each phase checkpoint.
