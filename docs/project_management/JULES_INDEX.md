# Jules Dependency Coordination - Documentation Index

**Jules-17: Dependency Coordinator**

This directory contains all documentation for coordinating the 14+ Jules coding agent tasks working in parallel on the rust-daq project.

## Quick Start

**New here?** Start with the Quick Reference for immediate action items:
- [JULES_QUICK_REFERENCE.md](JULES_QUICK_REFERENCE.md)

**Need the big picture?** Read the Executive Report:
- [JULES_COORDINATOR_REPORT.md](JULES_COORDINATOR_REPORT.md)

**Want details?** Check the complete Dependency Map:
- [JULES_DEPENDENCY_MAP.md](JULES_DEPENDENCY_MAP.md)

## Document Overview

### Core Documentation

| Document | Purpose | When to Use |
|----------|---------|-------------|
| [JULES_QUICK_REFERENCE.md](JULES_QUICK_REFERENCE.md) | Command cheat sheet and status at a glance | Every session start, quick checks |
| [JULES_COORDINATOR_REPORT.md](JULES_COORDINATOR_REPORT.md) | Executive summary with action plans | Strategic planning, weekly reviews |
| [JULES_DEPENDENCY_MAP.md](JULES_DEPENDENCY_MAP.md) | Complete task breakdown with dependencies | Detailed planning, agent assignments |
| [JULES_DEPENDENCY_GRAPH.mermaid](JULES_DEPENDENCY_GRAPH.mermaid) | Visual dependency graph | Understanding relationships |
| [JULES_NOTIFICATIONS.md](JULES_NOTIFICATIONS.md) | Real-time dependency notifications | When dependencies complete |

### Tools

| Tool | Purpose | Usage |
|------|---------|-------|
| [jules_dependency_monitor.sh](../../scripts/jules_dependency_monitor.sh) | Automated dependency monitoring | `./scripts/jules_dependency_monitor.sh --once` |

## Critical Information

### IMMEDIATE ACTION REQUIRED

**Jules-10 (bd-hqy6)** - Define ScriptEngine Trait
- Status: OPEN (Priority P0, Critical)
- Impact: Blocks 4 downstream tasks (27% of remaining work)
- **Must start immediately**

See [JULES_COORDINATOR_REPORT.md](JULES_COORDINATOR_REPORT.md) for launch commands.

### Current Status

- **Total Tasks**: 15
- **Completed**: 4 (27%)
- **Ready**: 9 (60%)
- **Blocked**: 4 (27%)
- **Maximum Parallelization**: 9 concurrent sessions

### Critical Path

```
Jules-10 (ScriptEngine trait)
  → Jules-11 (PyO3 backend)
    → Jules-12 (V3 API bindings)
      → Jules-13 (Hot-reload)
```

## Daily Workflow

1. **Morning**: Check status
   ```bash
   ./scripts/jules_dependency_monitor.sh --once
   cat docs/project_management/JULES_NOTIFICATIONS.md
   ```

2. **During Work**: Monitor sessions
   ```bash
   jules remote list --session
   gh pr list --repo TheFermiSea/rust-daq
   ```

3. **After Completion**: Update tracking
   ```bash
   BEADS_DB=.beads/daq.db bd update bd-XXX --status closed
   brv add -s "Lessons Learned" -c "Jules-X completed bd-XXX..."
   brv push -y
   ```

4. **Evening**: Check for newly ready tasks
   ```bash
   ./scripts/jules_dependency_monitor.sh --once
   ```

## Document Relationships

```
JULES_INDEX.md (you are here)
  │
  ├─ JULES_QUICK_REFERENCE.md
  │  └─ Quick commands, status summary
  │
  ├─ JULES_COORDINATOR_REPORT.md
  │  ├─ Executive summary
  │  ├─ Action plans
  │  └─ Launch sequences
  │
  ├─ JULES_DEPENDENCY_MAP.md
  │  ├─ Task details
  │  ├─ Agent assignments
  │  └─ Dependency chains
  │
  ├─ JULES_DEPENDENCY_GRAPH.mermaid
  │  └─ Visual dependency graph
  │
  ├─ JULES_NOTIFICATIONS.md
  │  └─ Auto-generated notifications
  │
  └─ scripts/jules_dependency_monitor.sh
     └─ Automated monitoring tool
```

## Agent Assignments

| Agent | Task ID | Title | Status | Priority |
|-------|---------|-------|--------|----------|
| **Jules-10** | **bd-hqy6** | **ScriptEngine Trait** | **Ready** | **P0** 🔴 |
| Jules-2 | bd-95pj | ESP300 V3 Migration | Ready | P0 |
| Jules-3 | bd-l7vs | MaiTai V3 Migration | Ready | P0 |
| Jules-4 | bd-l7vs | Newport V3 Refactor | Ready | P0 |
| Jules-5 | bd-e18h | PVCAM V3 Fix | Ready | P0 |
| Jules-6 | bd-op7v | Measurement Enum | Ready | P1 |
| Jules-7 | bd-9cz0 | Trait Signatures | Ready | P1 |
| Jules-8 | bd-rcxa | Arrow Batching | Ready | P1 |
| Jules-9 | bd-vkp3 | HDF5 Integration | Blocked | P1 |
| Jules-11 | bd-svlx | PyO3 Backend | Blocked | P1 |
| Jules-12 | bd-dxqi | V3 Bindings | Blocked | P1 |
| Jules-13 | bd-u7hu | Hot-Reload | Blocked | P2 |
| Jules-14 | bd-ya3l | Rhai Backend | Ready | P2 |

## Phase Progress

### Phase 1: Infrastructure ✅ (100%)
- All foundation tasks complete
- Kameo analysis done
- SCPI V3 migration complete
- Import fixes complete

### Phase 2: V3 Instrument Migrations 🔄 (0%)
- 3 tasks ready (ESP300, MaiTai, PVCAM)
- All can run in parallel
- Target: 1-2 weeks

### Phase 3: Data Layer Cleanup ⏳ (0%)
- 3 tasks ready (Measurement, Traits, Arrow)
- 1 task blocked (HDF5, waiting on Arrow)
- Target: 1 week

### Phase 4: Scripting Layer 🚧 (0%)
- 2 tasks ready (ScriptEngine, Rhai)
- 3 tasks blocked (PyO3, Bindings, Hot-reload)
- Jules-10 is CRITICAL blocker
- Target: 2-3 weeks (sequential)

## Key Metrics

### Target Performance
- **Completion Rate**: >70% (historical: 71%)
- **PR Submission**: >60%
- **CI Pass Rate**: >80%
- **Beads Sync**: 100%

### Timeline Estimates
- **Phase 2**: 1-2 weeks (parallel)
- **Phase 3**: 1 week (parallel)
- **Phase 4**: 2-3 weeks (sequential, Jules-10 bottleneck)
- **Total Remaining**: 4-6 weeks with optimal parallelization

## Escalation Contacts

- **Dependency Coordinator**: Jules-17 (this agent)
- **Orchestrator**: Claude Code
- **Strategic Advisor**: Gemini (via `mcp__zen__clink`)
- **Deep Implementer**: Codex (via `mcp__zen__clink`)

## Integration with Project Management

### Beads Tracker
```bash
BEADS_DB=.beads/daq.db bd ready --json
BEADS_DB=.beads/daq.db bd show <task-id>
```

### ByteRover Memory
```bash
brv retrieve -q "topic"
brv add -s "Section" -c "file:line - details"
brv push -y
```

### Git Integration
```bash
git add .beads/issues.jsonl docs/project_management/
git commit -m "chore: update Jules coordination docs"
```

## Automated Monitoring

### One-Time Check
```bash
./scripts/jules_dependency_monitor.sh --once
```

### Continuous Monitoring
```bash
./scripts/jules_dependency_monitor.sh --daemon  # Every 5 minutes
```

### Notifications
Auto-written to: `docs/project_management/JULES_NOTIFICATIONS.md`

## Success Criteria

### Overall Goals
- [ ] All 15 tasks completed
- [ ] All PRs merged
- [ ] CI passing on all changes
- [ ] Integration tests passing
- [ ] Documentation updated

### Phase Gates
- [ ] Phase 2: All V3 migrations functional
- [ ] Phase 3: Data layer standardized on Arrow/Measurement
- [ ] Phase 4: Python and Rhai scripts can control instruments

## Resources

### Internal Documentation
- [AGENTS.md](AGENTS.md) - Multi-agent coordination patterns
- [CLAUDE.md](CLAUDE.md) - Development conventions
- [GEMINI.md](GEMINI.md) - Strategic advisor guidelines
- [BD_JULES_INTEGRATION.md](BD_JULES_INTEGRATION.md) - Jules + Beads workflow

### External References
- Jules Platform: https://jules.google.com
- Beads Tracker: https://github.com/steveyegge/beads
- ByteRover: Project memory system
- rust-daq repo: https://github.com/TheFermiSea/rust-daq

---

**Maintained by**: Jules-17 (Dependency Coordinator)
**Created**: 2025-11-20
**Last Updated**: 2025-11-20
**Status**: Active Coordination
**Next Review**: Daily or on request
