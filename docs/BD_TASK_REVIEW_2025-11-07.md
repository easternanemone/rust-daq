# Beads Task Review & Workflow Analysis
**Date**: 2025-11-07
**Scope**: Highest priority P2 ast-grep tasks

## Executive Summary

**Status**: ✅ All 3 proposed P2 tasks are well-scoped and ready for implementation **with modifications**

**Key Finding**: The tasks are correctly prioritized, but implementation order and scope need adjustment for maximum efficiency and minimal disruption.

---

## Task-by-Task Deep Review

### 1. bd-ic14: Add ast-grep pre-commit hook and CI integration (P2)
**Status**: ✅ Ready - Should be FIRST
**Priority Adjustment**: ⬆️ Should be P1 (highest within P2 group)

#### Why This Should Go First:
1. **Foundation for Quality**: Enables continuous enforcement before other refactoring work
2. **Prevents Regression**: Once bd-wyqo and bd-ltd3 are complete, prevents reintroduction
3. **Fast Implementation**: 1-2 hours vs 2-4 hours for other tasks
4. **Existing CI/CD**: `.github/workflows/ci.yml` already exists - just add ast-grep step
5. **Low Risk**: Non-invasive, doesn't touch production code

#### Proposed Workflow:
```bash
# Phase 1: CI Integration (30 min)
1. Update .github/workflows/ci.yml
   - Add ast-grep installation step
   - Add ast-grep scan step with JSON output
   - Configure to fail on ERROR severity violations
   - Allow WARNING/HINT violations (informational only)

# Phase 2: Pre-commit Hook (30 min)
2. Create .git/hooks/pre-commit script
   - Run ast-grep on staged *.rs files only
   - Fast feedback (seconds, not minutes)
   - Optional: Use git stash to test clean working tree

# Phase 3: Documentation (30 min)
3. Update CLAUDE.md with ast-grep workflow
4. Add pre-commit hook installation instructions
5. Document CI/CD behavior
```

#### Recommended Changes to bd-ic14:
```bash
bd update bd-ic14 --priority 1 \
  --description "Add PRIORITY 1 note: Implement FIRST to establish quality baseline before refactoring work"
```

**Acceptance Criteria**:
- ✅ CI fails on ERROR severity ast-grep violations
- ✅ CI passes on current codebase (0 ERROR violations after bd-d617 fix)
- ✅ Pre-commit hook prevents ERROR violations locally
- ✅ Documentation updated with installation instructions

---

### 2. bd-wyqo: Create specific DaqError variants (P2)
**Status**: ⚠️ Needs Scope Refinement
**Priority Adjustment**: ➡️ Keep P2, implement SECOND

#### Issues Identified:

**Problem 1: Overly Broad Scope**
- **Current**: "Fix all 62 instances"
- **Reality**: 62 instances span 10+ files across multiple subsystems
- **Risk**: 2-4 hour estimate is optimistic for 62 changes
- **Impact**: May cause merge conflicts if other work is ongoing

**Problem 2: Missing Implementation Plan**
- No phased approach defined
- All-or-nothing makes rollback difficult
- Testing strategy not specified

**Problem 3: Error Type Design Not Validated**
Looking at src/error.rs (lines 38-65), current DaqError has:
```rust
pub enum DaqError {
    Instrument(String),     // Generic catch-all
    Processing(String),     // Generic catch-all
    Configuration(String),  // Generic catch-all
    // ... specific types ...
}
```

The proposed new variants mix concerns:
- Serial-specific: SerialPortNotConnected, SerialUnexpectedEof
- Module-specific: CameraNotAssigned, ModuleBusyDuringOperation
- Cross-cutting: ParameterNoSubscribers

#### Recommended Changes to bd-wyqo:

**Create Subtasks for Phased Implementation:**

```bash
# Create Phase 1: Serial Adapter Errors (HIGH priority - 5 instances)
bd create "Phase 1: Implement SerialAdapter specific errors" \
  --parent bd-wyqo \
  --priority 2 \
  --description "Add 3 variants to DaqError:
- SerialPortNotConnected
- SerialUnexpectedEof
- SerialFeatureDisabled

Replace 5 anyhow!() instances in src/adapters/serial_adapter.rs

Affected lines: 88, 118, 132, 139, 155, 198

Testing:
- Verify serial adapter connection failures
- Test EOF handling during reads
- Confirm feature flag error messages"

# Create Phase 2: Parameter Errors (MEDIUM priority - 4 instances)
bd create "Phase 2: Implement Parameter specific errors" \
  --parent bd-wyqo \
  --priority 2 \
  --description "Add 2 variants to DaqError:
- ParameterNoSubscribers
- ParameterReadOnly
- ParameterInvalidChoice

Replace 4 anyhow!() instances in src/parameter.rs

Affected lines: 77, 91, 329, 343

Testing:
- Verify read-only parameter protection
- Test parameter validation
- Confirm subscriber management"

# Create Phase 3: Module Errors (LOWER priority - 7 instances)
bd create "Phase 3: Implement Module specific errors" \
  --parent bd-wyqo \
  --priority 3 \
  --description "Add 3 variants to DaqError:
- ModuleOperationNotSupported(String)
- ModuleBusyDuringOperation
- CameraNotAssigned

Replace 7 anyhow!() instances in src/modules/*.rs

Testing:
- Verify module state transitions
- Test camera assignment logic
- Confirm operation validation"
```

**Update Parent Task:**
```bash
bd update bd-wyqo \
  --description "PARENT EPIC: Phased implementation of specific DaqError variants

This is now an epic with 3 phased subtasks:
1. Phase 1: Serial adapters (5 instances) - HIGHEST PRIORITY
2. Phase 2: Parameters (4 instances) - MEDIUM
3. Phase 3: Modules (7 instances) - LOWER

Remaining 46 instances in VISA, SCPI, instruments deferred to Phase 4.

Benefits:
- Incremental implementation reduces risk
- Each phase independently testable
- Can merge phases separately
- Easier code review

Original scope: 62 instances across 10+ files
Revised Phase 1-3 scope: 16 instances across 3 key subsystems

See: docs/AST_GREP_ANALYSIS_2025-11-07.md Section 2"
```

**Acceptance Criteria (Phase 1 only)**:
- ✅ 3 new DaqError variants added
- ✅ All 5 serial_adapter.rs anyhow!() replaced
- ✅ No compilation errors
- ✅ Existing tests pass
- ✅ ast-grep violations reduced by 5

---

### 3. bd-ltd3: Make timeouts configurable via config.toml (P2)
**Status**: ⚠️ Needs Design Validation
**Priority Adjustment**: ➡️ Keep P2, implement THIRD

#### Issues Identified:

**Problem 1: Config Structure Not Validated**
Current config.toml (lines 1-163) has:
```toml
[application]
broadcast_channel_capacity = 1024
command_channel_capacity = 32

[application.data_distributor]
subscriber_capacity = 1024
metrics_window_secs = 10

[storage]
default_path = "data_output"
```

Proposed `[timeouts]` section doesn't follow existing pattern. Should be:
```toml
[application.timeouts]  # Nested under application
serial_read_timeout_ms = 1000
...
```

**Problem 2: Timeout Categorization Unclear**
The 23 instances fall into categories:
1. **Serial I/O** (7 instances) - Per-instrument timeouts
2. **Network** (10 instances) - System-wide timeouts
3. **Instrument Management** (6 instances) - Operation timeouts

Should these be:
- Global defaults in `[application.timeouts]`?
- Per-instrument overrides in `[instruments.*.timeouts]`?
- Both?

**Problem 3: Migration Strategy Missing**
- How to handle existing deployments without timeout config?
- What are the default values if missing?
- Backward compatibility with old config files?

#### Recommended Changes to bd-ltd3:

**Add Design Phase:**
```bash
bd create "Design timeout configuration architecture" \
  --deps "blocks:bd-ltd3" \
  --priority 2 \
  --description "Design timeout configuration system BEFORE implementation.

Key decisions:
1. Config structure: [application.timeouts] vs [timeouts]
2. Inheritance model: global defaults + per-instrument overrides?
3. Timeout categories:
   - I/O timeouts (serial read/write)
   - Protocol timeouts (SCPI command)
   - Operation timeouts (connect/shutdown)
   - Network timeouts (client connections)

4. Migration strategy:
   - Default values if missing
   - Backward compatibility
   - Validation rules

5. Implementation phases:
   - Phase 1: Global defaults only (simple)
   - Phase 2: Per-instrument overrides (advanced)

Output:
- Updated config.toml with full timeout section
- Settings struct definition
- Migration plan
- Test cases

Estimated: 1 hour design, saves 2 hours refactoring"
```

**Update bd-ltd3:**
```bash
bd update bd-ltd3 \
  --deps "discovered-from:bd-d617" \
  --description "BLOCKED by design task: Make timeouts configurable via config.toml

MUST COMPLETE DESIGN PHASE FIRST before implementation.

Current scope: 23 hardcoded timeout values
Categories:
- Serial I/O: 7 instances (1s read/write)
- Network: 10 instances (5s, 10s operations)
- Instrument management: 6 instances (5s, 6s operations)

Proposed approach:
Phase 1: Global defaults in [application.timeouts]
Phase 2: Per-instrument overrides (future)

Benefits:
- Tunable per deployment
- Easy adjustment for slow hardware
- No recompilation needed
- Documented in one place

Implementation estimate: 2-3 hours AFTER design phase
Testing estimate: 1 hour (verify all timeout paths)

See: docs/AST_GREP_ANALYSIS_2025-11-07.md Section 3"
```

**Acceptance Criteria (After Design):**
- ✅ Design document approved
- ✅ Config structure validated
- ✅ Backward compatibility ensured
- ✅ Migration path defined

---

## Recommended Implementation Order

### Phase 1: Foundation (1-2 hours)
1. **bd-ic14**: CI/CD + pre-commit hooks
   - Establishes quality baseline
   - Fast, low-risk implementation
   - Prevents regression immediately

### Phase 2: Quick Win (2-3 hours)
2. **bd-wyqo Phase 1**: Serial adapter errors only
   - Small, focused scope (5 instances)
   - High-value subsystem (hardware I/O)
   - Tests existing immediately
   - Demonstrates pattern for future phases

### Phase 3: Design Work (1 hour)
3. **bd-ltd3 Design**: Timeout configuration architecture
   - Validate approach before coding
   - Prevent rework and refactoring
   - Define clear acceptance criteria

### Phase 4: Implementation (2-3 hours)
4. **bd-ltd3 Implementation**: Based on approved design
   - Clear specification from Phase 3
   - Reduced risk of scope creep
   - Straightforward implementation

### Phase 5: Completion (Optional)
5. **bd-wyqo Phase 2-3**: Parameter and module errors
   - If time permits
   - Lower priority than foundation work
   - Can be deferred to next sprint

---

## Risk Analysis

### High Risk Issues (Require Changes):

**Risk 1: bd-wyqo Scope Creep**
- **Problem**: 62 instances too broad for single task
- **Impact**: Delays, merge conflicts, difficult code review
- **Mitigation**: ✅ Split into phased subtasks (recommended above)

**Risk 2: bd-ltd3 Design Uncertainty**
- **Problem**: Config structure not validated
- **Impact**: Rework, backward compatibility issues
- **Mitigation**: ✅ Add design phase blocker (recommended above)

**Risk 3: Implementation Order**
- **Problem**: Refactoring before quality enforcement
- **Impact**: May reintroduce violations fixed in other tasks
- **Mitigation**: ✅ Do bd-ic14 FIRST (recommended above)

### Medium Risk Issues (Monitor):

**Risk 4: Testing Coverage**
- **Problem**: No test strategy defined for error variants
- **Impact**: Bugs may slip through
- **Mitigation**: Add explicit test requirements to acceptance criteria

**Risk 5: Documentation Lag**
- **Problem**: CLAUDE.md not updated with new patterns
- **Impact**: Future developers repeat old patterns
- **Mitigation**: Include documentation in each task's acceptance criteria

---

## Recommended bd Task Updates

### Summary of Changes:

1. **bd-ic14** (CI integration)
   - ⬆️ Priority 1 (highest in P2 group)
   - ➕ Note: "Implement FIRST"
   - Status: ✅ Ready to implement

2. **bd-wyqo** (Error variants)
   - 🔀 Convert to epic with 3 subtasks
   - ✂️ Reduce Phase 1 scope to 5 instances (serial only)
   - 📋 Add explicit test requirements
   - Status: ⚠️ Needs subtask creation

3. **bd-ltd3** (Configurable timeouts)
   - 🚧 Add blocker: Design phase required
   - 📐 Create design task with blocking dependency
   - ⏱️ Adjust estimate: +1 hour design, keep 2-3 hours implementation
   - Status: 🚫 Blocked until design complete

### Execution Commands:

```bash
# 1. Promote bd-ic14 to highest priority
bd update bd-ic14 --priority 1
bd note add bd-ic14 "IMPLEMENT FIRST - Foundation for quality enforcement"

# 2. Create bd-wyqo subtasks (3 phases)
bd create "Phase 1: Serial adapter specific errors" \
  --parent bd-wyqo \
  --priority 2 \
  --type task

bd create "Phase 2: Parameter specific errors" \
  --parent bd-wyqo \
  --priority 2 \
  --type task

bd create "Phase 3: Module specific errors" \
  --parent bd-wyqo \
  --priority 3 \
  --type task

bd update bd-wyqo --type epic

# 3. Create bd-ltd3 design blocker
bd create "Design timeout configuration architecture" \
  --priority 2 \
  --type task

bd dep add bd-ltd3 <new-design-task-id> --type blocks
```

---

## Conclusion

### ✅ Strengths of Current Plan:
- All 3 tasks address real code quality issues
- Priorities correctly aligned with impact
- ast-grep analysis provides solid foundation
- Scope is manageable (9-13 hours total)

### ⚠️ Required Changes:
1. **Implementation Order**: bd-ic14 → bd-wyqo Phase 1 → bd-ltd3 Design → bd-ltd3 Impl
2. **Scope Refinement**: Split bd-wyqo into phased approach
3. **Design Validation**: Add design blocker for bd-ltd3
4. **Testing Strategy**: Add explicit test requirements

### 🎯 Next Steps:
1. Apply recommended bd updates (above)
2. Start with bd-ic14 (1-2 hours, high impact)
3. Create subtasks for bd-wyqo phased approach
4. Create design task blocking bd-ltd3
5. Proceed with implementation in recommended order

**Total Revised Estimate**: 6-9 hours for foundation work (Phases 1-3), optional 4-6 hours for completion (Phases 4-5)

---

**Generated**: 2025-11-07
**Reviewed By**: Claude Code ast-grep analysis workflow
**Status**: Ready for implementation with modifications
