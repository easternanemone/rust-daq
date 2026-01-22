# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2025-01-22)

**Core value:** Scientists can design and interactively run experiments without writing code, while power users retain full programmatic control
**Current focus:** Phase 3 complete, ready for Phase 4

## Current Position

Phase: 4 of 10 (Sequences and Control Flow) - PLANNED
Plan: 0 of 3 planned
Status: Ready for execution
Last activity: 2026-01-22 - Planned Phase 4 (3 plans in 2 waves, verification passed)

Progress: [███░░░░░░░] 30%

## Performance Metrics

**Velocity:**
- Total plans completed: 11
- Average duration: 7.0min
- Total execution time: 1.4 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan | Status |
|-------|-------|-------|----------|--------|
| 01 | 3 | 17min | 5.7min | ✓ Complete |
| 02 | 4 | 36min | 9.0min | ✓ Complete |
| 03 | 4 | 28min | 7.0min | ✓ Complete |

**Recent Trend:**
- Last 5 plans: 03-01 (7min), 03-02 (8min), 03-03 (5min), 03-04 (8min)
- Trend: Consistent mid-range (5-8min)

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Node-based visual editor chosen as primary interface (standard scientific workflow paradigm)
- One-way code generation established (visual is source of truth, code is export only)
- Parameter injection for live edits (RunEngine Checkpoint-based, structure immutable during execution)
- Context menu as primary node-add UX (more reliable than drag-drop with coordinate transforms)
- Unified GraphEdit enum for undo/redo (undo::Record<E> requires single E type)
- .expgraph file extension for experiment graphs (distinct from generic JSON)
- Validation errors in status bar + property inspector (avoids egui-snarl API complexity)
- Kahn's algorithm for cycle detection (standard, efficient O(V+E))
- Display cycle errors on first node (avoids overwhelming user with multiple errors)
- Skip per-node validation when cycle detected (cycles make execution impossible)
- Channel-based async communication for gRPC calls (non-blocking UI)
- ExecutionState cloned to viewer before render (cheap, avoids lifetime issues)
- Visual highlighting infrastructure ready (pending egui-snarl API support)
- Separate RuntimeParameterEditor from parameter_editor.rs (different use case: runtime vs device introspection)

### Pending Todos

None yet.

### Blockers/Concerns

- Pre-existing test failure in graph::serialization::tests::test_version_check (not introduced by 03-01)
- egui-snarl lacks custom header color API (visual node highlighting infrastructure ready but not applied)

### Phase 3 Verification Notes

Human approved with 3/4 success criteria fully verified. Known gaps documented for future work:
1. GraphPlan not sent to server (UI-side translation complete, server integration deferred)
2. Visual node highlighting not activated (egui-snarl API limitation)

See: .planning/phases/03-plan-translation-and-execution/03-VERIFICATION.md

## Session Continuity

Last session: 2026-01-22
Stopped at: Phase 4 planned (3 plans in 2 waves)
Resume file: None
Next action: Execute Phase 4 (Sequences and Control Flow)
