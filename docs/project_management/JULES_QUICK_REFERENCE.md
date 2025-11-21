# Jules Dependency Coordination - Quick Reference

**Jules-17: Dependency Coordinator**

## TL;DR

- **Total Tasks**: 15 (4 complete, 9 ready, 4 blocked)
- **Critical Blocker**: Jules-10 (bd-hqy6) ScriptEngine trait - START IMMEDIATELY
- **Ready Tasks**: 9 (can run in parallel right now)
- **Maximum Parallelization**: 9 concurrent Jules sessions

## Critical Alert

🚨 **Jules-10 (bd-hqy6) MUST START NOW**

Blocks 4 downstream tasks (1,800+ lines of code, 27% of remaining work).

```bash
jules new --repo TheFermiSea/rust-daq "bd-hqy6: Define ScriptEngine trait"
```

## Quick Commands

### Check Dependencies
```bash
./scripts/jules_dependency_monitor.sh --once
```

### Continuous Monitoring
```bash
./scripts/jules_dependency_monitor.sh --daemon  # Checks every 5 min
```

### Check Jules Session Status
```bash
jules remote list --session
```

### Check Beads Status
```bash
BEADS_DB=.beads/daq.db bd ready --json
```

### View Dependency Map
```bash
cat docs/project_management/JULES_DEPENDENCY_MAP.md
cat docs/project_management/JULES_COORDINATOR_REPORT.md
```

## Task Status at a Glance

### Phase 1: Infrastructure ✅ (100%)
- bd-r896: Kameo Analysis [CLOSED]
- bd-o6c7: SCPI Migration [CLOSED]
- bd-ifxt: Import Fix [CLOSED]

### Phase 2: V3 Migrations 🔄 (0%, all ready)
- bd-95pj: ESP300 V3 [READY] - Jules-2
- bd-l7vs: MaiTai V3 [READY] - Jules-3
- bd-l7vs: Newport V3 [READY] - Jules-4
- bd-e18h: PVCAM Fix [READY] - Jules-5

### Phase 3: Data Layer ⏳ (0%, 3 ready, 1 blocked)
- bd-op7v: Measurement Enum [READY] - Jules-6
- bd-9cz0: Trait Signatures [READY] - Jules-7
- bd-rcxa: Arrow Batching [READY] - Jules-8
- bd-vkp3: HDF5 + Arrow [BLOCKED] - Jules-9 (needs Jules-8)

### Phase 4: Scripting 🚧 (0%, 2 ready, 3 blocked)
- **bd-hqy6: ScriptEngine Trait [READY] - Jules-10** 🔴 CRITICAL
- bd-svlx: PyO3 Backend [BLOCKED] - Jules-11 (needs Jules-10)
- bd-dxqi: V3 Bindings [BLOCKED] - Jules-12 (needs Jules-10+11)
- bd-ya3l: Rhai Backend [READY] - Jules-14 (needs Jules-10)
- bd-u7hu: Hot-Reload [BLOCKED] - Jules-13 (needs Jules-12)

## Dependency Chain

```
Jules-10 (ScriptEngine)
  ├→ Jules-11 (PyO3)
  │   └→ Jules-12 (V3 Bindings)
  │       └→ Jules-13 (Hot-Reload)
  └→ Jules-14 (Rhai)

Jules-8 (Arrow Batching)
  └→ Jules-9 (HDF5 Integration)
```

## Launch Sequence

**Step 1**: Launch Critical Blocker (NOW)
```bash
jules new --repo TheFermiSea/rust-daq "bd-hqy6: ScriptEngine trait"
```

**Step 2**: Launch Phase 2 Migrations (4 sessions)
```bash
jules new --repo TheFermiSea/rust-daq "bd-95pj: ESP300 V3"
sleep 10
jules new --repo TheFermiSea/rust-daq "bd-l7vs: MaiTai V3"
sleep 10
jules new --repo TheFermiSea/rust-daq "bd-e18h: PVCAM V3 Fix"
sleep 10
```

**Step 3**: Launch Phase 3 Cleanups (3 sessions)
```bash
jules new --repo TheFermiSea/rust-daq "bd-op7v: Measurement Enum"
sleep 10
jules new --repo TheFermiSea/rust-daq "bd-9cz0: Trait Signatures"
sleep 10
jules new --repo TheFermiSea/rust-daq "bd-rcxa: Arrow Batching"
sleep 10
```

**Step 4**: Monitor Jules-10 (every 6-12h)
- If complete → Launch Jules-11, Jules-14
- If stuck >24h → Escalate to Gemini/Codex

**Step 5**: Cascade Unblocking
- Jules-8 completes → Launch Jules-9
- Jules-11 completes → Launch Jules-12
- Jules-12 completes → Launch Jules-13

## Escalation Ladder

1. **Stuck in Planning (>12h)**: Provide reference implementations
2. **Stuck in Planning (>24h)**: Escalate to Gemini via `mcp__zen__clink`
3. **Implementation Failure (>48h)**: Escalate to Codex via `mcp__zen__clink`
4. **Circular Dependency**: IMMEDIATE escalation to Claude Code

## Progress Tracking

After each Jules session completes:

1. **Update Beads**:
   ```bash
   BEADS_DB=.beads/daq.db bd update bd-XXX --status closed --reason "Completed by Jules session YYYYY"
   git add .beads/issues.jsonl
   ```

2. **Record in ByteRover**:
   ```bash
   brv add -s "Lessons Learned" -c "Jules-X completed bd-XXX. Implementation: file:line details"
   brv push -y
   ```

3. **Update Dependency Map**:
   ```bash
   vim docs/project_management/JULES_DEPENDENCY_MAP.md
   # Mark task complete, update agent status
   ```

4. **Check for Newly Ready Tasks**:
   ```bash
   ./scripts/jules_dependency_monitor.sh --once
   cat docs/project_management/JULES_NOTIFICATIONS.md
   ```

## Success Metrics

- **Completion Rate**: Target >70% (current historical: 71%)
- **PR Submission**: Target >60%
- **CI Pass Rate**: Target >80%
- **Beads Sync**: Target 100%

## Documentation

- **Main Map**: `docs/project_management/JULES_DEPENDENCY_MAP.md`
- **Visual Graph**: `docs/project_management/JULES_DEPENDENCY_GRAPH.mermaid`
- **Executive Report**: `docs/project_management/JULES_COORDINATOR_REPORT.md`
- **Notifications**: `docs/project_management/JULES_NOTIFICATIONS.md`
- **This Guide**: `docs/project_management/JULES_QUICK_REFERENCE.md`

## Agent Assignments

| Agent | Task | Status | Priority |
|-------|------|--------|----------|
| Jules-2 | ESP300 V3 | Ready | P0 |
| Jules-3 | MaiTai V3 | Ready | P0 |
| Jules-4 | Newport V3 | Ready | P0 |
| Jules-5 | PVCAM Fix | Ready | P0 |
| Jules-6 | Measurement Enum | Ready | P1 |
| Jules-7 | Trait Signatures | Ready | P1 |
| Jules-8 | Arrow Batching | Ready | P1 |
| Jules-9 | HDF5 Integration | Blocked (needs Jules-8) | P1 |
| **Jules-10** | **ScriptEngine Trait** | **Ready** | **P0** 🔴 |
| Jules-11 | PyO3 Backend | Blocked (needs Jules-10) | P1 |
| Jules-12 | V3 Bindings | Blocked (needs Jules-10+11) | P1 |
| Jules-13 | Hot-Reload | Blocked (needs Jules-12) | P2 |
| Jules-14 | Rhai Backend | Ready (needs Jules-10) | P2 |

---

**Maintained by**: Jules-17 (Dependency Coordinator)
**Last Updated**: 2025-11-20
**Status**: Active
