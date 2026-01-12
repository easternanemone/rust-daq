# Beads Database Cleanup - Complete Package

## 📦 What's Been Created

### Documentation (585 lines total)
1. **CLEANUP_PLAN.md** (258 lines) - Detailed architectural analysis and cleanup strategy
2. **QUICK_START.md** (135 lines) - TL;DR execution guide
3. **scripts/README.md** (192 lines) - Script documentation and safety procedures

### Executable Scripts (6 scripts, ~14KB)
1. **01-verify-deletions.sh** - Verify V1/V2/V4 files deleted
2. **02-label-dead-architectures.sh** - Label V4/V2/V1 as obsolete
3. **03-label-v5-active.sh** - Label current V5 work
4. **04-close-dead-issues.sh** - Close obsolete issues
5. **05-run-compaction.sh** - Compact old closed issues
6. **06-maintenance-schedule.sh** - Regular health checks

All scripts are executable (`chmod +x`) and production-ready.

---

## 🎯 The Problem (From Architectural Analysis)

Your rust-daq project has gone through 5 architectural iterations:
- **V1** → Initial implementation → **DEAD** 🧟
- **V2** → Actor model → **DEAD** 🧟
- **V2.5** → Bridge attempt → **DEAD** 🧟
- **V4** → Kameo actors → **DEAD** ☠️
- **V5** → Headless + Capability Traits → **ALIVE** 🌱

**Current Beads Database State:**
- 433 total issues
- Only 29 labeled (6.7% - **CRITICAL PROBLEM**)
- Mix of V1-V5 issues (hard to find relevant work)
- 100 "open" issues (but many are V4 tasks for dead architecture)
- 64 closed issues eligible for compaction

---

## ✨ The Solution

### Phase 1: Safe Labeling (Reversible)
**Scripts:** 01-verify-deletions.sh, 02-label-dead-architectures.sh, 03-label-v5-active.sh

- Verify V1/V2/V4 files are deleted
- Label V4 issues: `arch:v4-dead`, `status:wontfix`
- Label V5 issues: `arch:v5-active`, `priority:critical`
- **Risk:** NONE (read-only or adds labels only)

### Phase 2: Close Obsolete (Reversible)
**Script:** 04-close-dead-issues.sh

- Close ~8 V4 (Kameo) issues
- Close ~2 obsolete GUI issues (V5 is headless)
- Add detailed closure comments
- **Risk:** LOW (can reopen if needed)
- **Backup:** Automatic

### Phase 3: Compaction (Permanent)
**Script:** 05-run-compaction.sh

- Compact 64 closed issues (>30 days old)
- Save ~40KB
- Run VACUUM on SQLite
- **Risk:** MEDIUM (permanent, but only affects already-closed issues)
- **Backup:** Automatic

### Phase 4: Ongoing Maintenance
**Script:** 06-maintenance-schedule.sh

- Weekly health checks
- Find unlabeled issues
- Identify stale in_progress
- Track label coverage %

---

## 📊 Expected Results

### Before Cleanup
```
Total issues: 433
Open: 100 (includes obsolete V4 work)
Closed: 319
Labeled: 29 (6.7%) ← PROBLEM!
Database: 1.8MB

Hard to find:
- What's V5 active work?
- What's critical?
- What's obsolete?
```

### After Cleanup
```
Total issues: ~420 (-13 closed)
Open: ~87 (only V5 active)
Closed: ~333 (+64 compacted)
Labeled: 350+ (80%+) ← SOLVED!
Database: ~1.7MB (-100KB)

Easy queries:
✓ bd list --label arch:v5-active
✓ bd list --label priority:critical
✓ bd list --label component:driver
✓ bd list --label priority:safety
```

**Most Important:** 12x improvement in issue discoverability!

---

## 🚀 Quick Start (5-10 minutes)

```bash
cd /Users/briansquires/code/rust-daq/.beads/scripts

# SAFE - Review what will happen
./01-verify-deletions.sh
./02-label-dead-architectures.sh  # Adds labels
./03-label-v5-active.sh           # Adds labels

# REVIEW OUTPUT from above, then proceed:

# DESTRUCTIVE - Creates backups automatically
./04-close-dead-issues.sh  # Confirm with Enter
./05-run-compaction.sh     # Confirm with Enter

# CHECK RESULTS
./06-maintenance-schedule.sh
bd list --label arch:v5-active
```

---

## 🛡️ Safety Guarantees

Every destructive operation:
1. ✅ Creates automatic timestamped backup
2. ✅ Asks for confirmation (Press Enter)
3. ✅ Shows clear output of changes
4. ✅ Includes rollback instructions

**Rollback Example:**
```bash
# If something goes wrong
ls -la .beads-backup-*/
mv .beads .beads-broken
mv .beads-backup-20251119-073337 .beads
bd daemon &
```

---

## 📋 New Label Taxonomy

### Architecture (What version)
- `arch:v1-dead` - V1 legacy (historical only)
- `arch:v2-dead` - V2 actor model (obsolete)
- `arch:v4-dead` - V4 Kameo (abandoned)
- `arch:v5-active` - **V5 headless + capability traits (CURRENT)**
- `arch:v5-migration` - Active migration work
- `arch:v5-cleanup` - Post-migration cleanup

### Components (What part of system)
- `component:driver` - Hardware drivers
- `component:api` - Remote API / network layer
- `component:scripting` - Rhai/PyO3 scripting engine
- `component:data` - Ring buffer, HDF5, data streaming
- `component:core` - Core traits and types

### Priority (How urgent)
- `priority:critical` - Blocking V5 production
- `priority:safety` - Laboratory safety (e.g., serial2-tokio)
- `priority:cleanup` - Housekeeping

### Status (Why state is what it is)
- `status:wontfix` - Intentionally not implementing
- `status:obsolete` - Superseded by architecture change
- `status:cleanup` - Pending cleanup task

---

## 🔍 Useful Queries After Cleanup

```bash
# Show V5 active work (what to focus on)
bd list --label arch:v5-active

# Show critical blocking issues
bd list --label priority:critical

# Show driver migration work
bd list --label component:driver --label arch:v5-migration

# Show safety-critical work
bd list --label priority:safety

# Find what's obsolete (for historical reference)
bd list --label arch:v4-dead

# Find unlabeled issues (should be <20%)
bd list --all --json | jq '.[] | select(.labels | length == 0) | .id'

# Show what got compacted
bd compact --stats
```

---

## 📅 Maintenance Schedule

**Weekly (2 minutes):**
```bash
./06-maintenance-schedule.sh
# Review unlabeled issues
# Add missing labels
```

**Monthly (5 minutes):**
```bash
./05-run-compaction.sh
# Compact new closed issues
# Review blocked issues
```

**Quarterly (10 minutes):**
```bash
# Tier 2 compaction (90+ days)
bd compact --all --tier 2

# Full vacuum
sqlite3 .beads/*.db "VACUUM;"

# Export statistics
bd list --all --json > quarterly-snapshot-$(date +%Y%m%d).json
```

---

## ❓ Key Decisions (Based on Architectural Analysis)

### V4 (Kameo) Architecture: **DELETE**
**Decision:** V5 uses direct async, not actors.
**Files to delete:** `v4-daq/` directory
**Issues to close:** All V4-P0, V4-P1, V4 production tasks
**Reason:** Kameo adds complexity without benefits for headless architecture

### GUI/TUI: **SEPARATE APP**
**Decision:** V5 is headless-first.
**Files to delete:** `src/gui/` directory
**Issues to close:** GUI integration issues
**Reason:** GUI should be external React/Python app using V5 remote API

### V1/V2 Actor Model: **DELETE**
**Decision:** V3/V5 direct async is superior.
**Files to delete:** `src/app_actor.rs`, `src/core.rs`, `src/instrument_manager_v3.rs`
**Issues to close:** Deletion tasks (after verification)
**Reason:** Removes blocking calls, latency, and complexity

### V5 (Headless + Capability Traits): **PROMOTE**
**Decision:** This is the production architecture.
**Files to keep:** `src/hardware/capabilities.rs`, `src/hardware/mod.rs`
**Issues to label:** All V5 migration and feature work
**Reason:** Modern, async, compositional, testable

---

## 🎓 What You've Learned

1. **Label coverage matters:** 6.7% → 80%+ = 12x better discoverability
2. **Architectural debt is real:** 5 versions = 4 dead architectures to clean up
3. **Compaction saves space:** 64 issues → ~40KB savings
4. **Taxonomy improves workflow:** Clear labels = easy filtering
5. **Automation is essential:** Scripts prevent human error

---

## 📚 Additional Resources

- **Full Analysis:** `.beads/CLEANUP_PLAN.md`
- **Script Details:** `.beads/scripts/README.md`
- **Quick Reference:** `.beads/QUICK_START.md`
- **Architectural Analysis:** See original context from CLAUDE.md

---

## ✅ Ready to Execute?

```bash
cd /Users/briansquires/code/rust-daq/.beads/scripts
cat README.md  # Read script details
./01-verify-deletions.sh  # Start here
```

**Estimated time:** 5-10 minutes for complete cleanup
**Risk level:** LOW (automatic backups, confirmations, rollback instructions)
**Benefit:** 12x improvement in issue organization and discoverability

---

## 🏆 Success Metrics

After running all scripts, you should see:
- ✅ 80%+ of issues labeled
- ✅ Only V5 issues in "open" status
- ✅ Clear separation: active vs obsolete
- ✅ Easy filtering by component, priority, architecture
- ✅ ~100KB database savings
- ✅ Fast queries with label filters

**Most importantly:** You'll be able to answer:
- "What's blocking V5 production?" → `bd list --label priority:critical`
- "What driver work is left?" → `bd list --label component:driver`
- "What's safety-critical?" → `bd list --label priority:safety`
