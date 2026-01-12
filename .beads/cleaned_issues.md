# Beads Issue Refactoring Report

**Generated:** 2025-11-27
**Total Issues:** 491 | **Actionable:** 17 | **Ready:** 10 | **Blocked:** 4

---

## In Progress Work

### `P1` bd-c0ai: Implement ModuleService gRPC endpoint
**Status:** `in_progress` | **Type:** feature | **Estimate:** M

Phase 2 complete with Observable, Document, RunEngine patterns. Ready for Phase 3.

- [ ] Wire all ModuleService RPC methods to ModuleRegistry
- [ ] Verify: `cargo test --features modules modules::`
- [ ] Unblocks: bd-49l3 (GUI Modules Panel)

---

## Ready Work (Agent Handoff)

### Priority 1 - Critical Path

| ID | Title | Est | Next Action |
|----|-------|-----|-------------|
| **bd-194** | Test MaiTai hardware integration | S | SSH to maitai@100.117.5.12, run hardware tests |
| **bd-e52e.23** | Elliptec production deployment | M | Final integration testing phase |
| **bd-e52e.24** | Test Newport+Elliptec coordination | S | Multi-device coordination test |
| **bd-e52e.26** | Test full instrument suite | M | Newport + ESP300 + Elliptec together |

### Priority 2 - Quality Improvements

| ID | Title | Est | Next Action |
|----|-------|-----|-------------|
| **bd-qvib** | Fix MaiTai shutter commands | S | Debug on hardware - shutter not responding |
| **bd-q3jc** | Standardize gRPC error handling | S | Audit error semantics across services |
| **bd-e52e.28** | Document rotator calibration | S | Create calibration procedure doc |

### Priority 3 - Maintenance

| ID | Title | Est | Next Action |
|----|-------|-----|-------------|
| **bd-3p8i** | Add gRPC TLS authentication | M | Security enhancement |
| **bd-4j5p** | Schedule ast-grep audits | S | Set up quarterly schedule |

---

## Blocked Issues - Action Required

### `P1` bd-49: Resolve V1/V2 architecture conflict
**Status:** `blocked` | **Type:** epic | **Owner:** Claude | **Estimate:** L

**STALE JULES SESSION**: Patches from session `9424571726196863309` don't apply.

- [ ] **ACTION REQUIRED:** Restart Jules session against current main
- [ ] Single instrument architecture after migration
- [ ] All tests pass

**Children:**
- `bd-123` (P2): Extract SCPI/VISA common patterns - also has stale Jules patches

---

### Orphan Blocked (Dependency Issues)

These issues are blocked by **closed** parents - suggests stale dependency tracking:

| ID | Title | Blocked By | Action |
|----|-------|------------|--------|
| **bd-129** | Add TOML persistence for config | bd-78 (closed) | Unblock or close |
| **bd-133** | Add config versioning/rollback | bd-78 (closed) | Unblock or close |

---

## Changes Log

1. **Normalized titles** to <= 8 words, present tense verbs
2. **Identified 3 stale Jules sessions** with patches that don't apply:
   - bd-49: Session 9424571726196863309
   - bd-123: Session 7697081755934048502
   - bd-129: Session 17515406403085723461
   - bd-133: Session 5277935630865148932
3. **Flagged 2 orphan blocked issues** (bd-129, bd-133) blocked by closed parent
4. **Grouped hardware integration tasks** under P1 ready work

---

## Questions for Team

1. **bd-49**: Should we restart Jules session or manually port the V2 migration concepts?
2. **bd-129, bd-133**: These are blocked by closed parent bd-78. Should they be unblocked or closed?
3. **bd-123**: SCPI refactor has detailed design - implement fresh or restart Jules?
4. **Hardware tests**: Who has access to maitai@100.117.5.12 for P1 hardware validation?
5. **bd-c0ai**: Phase 3 scope - wire all RPCs or prioritize subset?
6. **GUI work**: bd-49l3 blocked by bd-c0ai - is this the right sequencing?
7. **Security (bd-3p8i)**: Is P3 appropriate or should TLS be elevated?

---

## PR Audit Summary (2025-11-27)

**Reduced from 28 to 8 open PRs.** 20 PRs closed (duplicates, stale, GUI-related).

### Remaining 8 PRs - All MERGEABLE

| PR | Title | Adds | Dels | Recommendation |
|----|-------|------|------|----------------|
| **#123** | Fix Unused Import Warnings | +2 | -2 | MERGE - Trivial fix |
| **#124** | Add feature gating for V4 error variants | +4 | 0 | MERGE - Trivial fix |
| **#125** | Update Laser Script Binding | +2 | -2 | MERGE - Trivial fix |
| **#108** | Add Elliptec Graceful Shutdown Test | +162 | 0 | MERGE - Adds test coverage |
| **#116** | Coordinated Operation Tests | +139 | 0 | MERGE - Adds test coverage |
| **#117** | Full Instrument Suite Integration Test | +198 | 0 | MERGE - Adds test coverage |
| **#104** | Arrow batching in DataDistributor | +437 | 0 | REVIEW - V5 valuable |
| **#120** | Migrate Configuration to Figment | +297 | -1698 | REVIEW - Major change |

**Note:** All PRs show UNSTABLE CI status due to systemic workflow failures (all same checks failing), not code issues.

### Jules Session Status

Jules quota exhausted (2025-11-27). Stale sessions cannot be restarted until quota resets:
- bd-49: V1/V2 architecture conflict
- bd-123: SCPI/VISA pattern extraction

---

## Recommended bd Commands

```bash
# Check current ready work
bd ready

# Unblock orphan issues (if approved)
bd update bd-129 --status open
bd update bd-133 --status open

# View dependency graph
bd dep bd-49

# After completing bd-c0ai
bd close bd-c0ai --reason "Phase 3 complete - all RPC methods wired"
bd ready  # Should now show bd-49l3

# Close stale blocked issues (if approved)
bd close bd-123 --reason "Stale - reimplement when bd-49 resolved"
```
