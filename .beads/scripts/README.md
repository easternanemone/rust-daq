# Beads Database Cleanup Scripts
## Safe, Careful Cleanup Based on Architectural Analysis

### Overview
These scripts implement the cleanup plan from `.beads/CLEANUP_PLAN.md`.
They safely close obsolete issues from dead architectures (V1, V2, V4) while
preserving valuable V5 migration work.

### Execution Order

```bash
cd /Users/briansquires/code/rust-daq/.beads/scripts

# Step 1: Verify file deletions (SAFE - read-only)
./01-verify-deletions.sh

# Step 2: Label dead architectures (SAFE - only adds labels)
./02-label-dead-architectures.sh

# Step 3: Label active V5 work (SAFE - only adds labels)
./03-label-v5-active.sh

# Step 4: Close obsolete issues (DESTRUCTIVE - but reversible)
# Review output from steps 1-3 first!
./04-close-dead-issues.sh

# Step 5: Compact database (DESTRUCTIVE - permanent)
# Only after confirming step 4 went well
./05-run-compaction.sh

# Step 6: Run maintenance check (SAFE - informational)
./06-maintenance-schedule.sh
```

### Safety Features

**Every destructive operation:**
1. Creates automatic backup before proceeding
2. Requires confirmation (Press Enter to continue)
3. Provides clear output of what was changed
4. Can be reverted from backup

**Backups are stored in project root:**
```bash
ls -la ../.beads-backup-*
```

### Script Details

#### 01-verify-deletions.sh (SAFE)
- **Purpose:** Verify that V1/V2/V4 files have been deleted
- **Output:** Checklist of which deletion tasks can be closed
- **Risk:** None (read-only)

#### 02-label-dead-architectures.sh (SAFE)
- **Purpose:** Add `arch:v4-dead`, `status:wontfix` labels to obsolete issues
- **Changes:** Only adds labels, doesn't close anything
- **Risk:** None (reversible - labels can be removed)

#### 03-label-v5-active.sh (SAFE)
- **Purpose:** Add `arch:v5-active`, `priority:critical` to current work
- **Changes:** Only adds labels
- **Risk:** None (reversible)

#### 04-close-dead-issues.sh (DESTRUCTIVE)
- **Purpose:** Close V4, GUI, obsolete issues
- **Changes:**
  - Closes ~8 V4 issues
  - Closes ~1-2 GUI issues
  - Adds closure comments explaining why
- **Risk:** LOW (can reopen if needed)
- **Backup:** Automatic (`.beads-backup-TIMESTAMP/`)

#### 05-run-compaction.sh (DESTRUCTIVE - PERMANENT)
- **Purpose:** Compact closed issues >30 days old
- **Changes:**
  - Compresses 64 closed issues
  - Saves ~40KB
  - **PERMANENT - cannot be undone**
- **Risk:** MEDIUM (only run after verifying database health)
- **Backup:** Automatic (`.beads-backup-compaction-TIMESTAMP/`)

#### 06-maintenance-schedule.sh (SAFE)
- **Purpose:** Health check and recommendations
- **Output:**
  - Unlabeled issues count
  - Stale in_progress issues
  - Compaction opportunities
  - Label coverage %
- **Risk:** None (read-only)

### Rollback Procedures

**If you need to undo changes:**

```bash
# List available backups
ls -la .beads-backup-*/

# Restore from backup (EXAMPLE - use actual timestamp)
mv .beads .beads-broken
mv .beads-backup-20251119-073337 .beads

# Restart daemon
bd daemon &

# Verify restoration
bd list | head -10
```

### Expected Results

**Before Cleanup:**
- 433 total issues
- 100 open (many obsolete V4 issues)
- 6.7% labeled (29 of 433)
- Database: 1.8MB

**After Cleanup:**
- ~420 total issues (13 closed)
- ~87 open (only V5 active work)
- 80%+ labeled (350+)
- Database: ~1.7MB
- **12x improvement in issue discoverability**

### Label Taxonomy

**Architecture:**
- `arch:v1-dead` - V1 legacy (historical only)
- `arch:v2-dead` - V2 actor model (obsolete)
- `arch:v4-dead` - V4 Kameo (abandoned)
- `arch:v5-active` - V5 headless + capability traits (CURRENT)
- `arch:v5-migration` - Active migration to V5
- `arch:v5-cleanup` - Post-migration cleanup

**Components:**
- `component:driver` - Hardware drivers
- `component:api` - Remote API / network
- `component:scripting` - Rhai/PyO3 scripting
- `component:data` - Ring buffer, HDF5, streaming
- `component:core` - Core traits and types

**Priority:**
- `priority:critical` - Blocking V5 production
- `priority:safety` - Laboratory safety
- `priority:cleanup` - Housekeeping

**Status:**
- `status:wontfix` - Intentionally not implementing
- `status:obsolete` - Superseded by architecture change
- `status:cleanup` - Pending cleanup task

### Maintenance Schedule

**Weekly:**
```bash
./06-maintenance-schedule.sh
# Review output, add missing labels
```

**Monthly:**
```bash
./05-run-compaction.sh  # After first month
# Compact old closed issues
```

**Quarterly:**
```bash
# Run Tier 2 compaction (90+ days)
bd compact --all --tier 2

# Full database vacuum
sqlite3 .beads/*.db "VACUUM;"
```

### Troubleshooting

**"Issue doesn't exist" errors:**
- Likely already closed or compacted
- Not a problem - scripts are idempotent

**Database corruption:**
- Restore from backup (see Rollback Procedures)
- Run integrity check: `sqlite3 .beads/*.db "PRAGMA integrity_check;"`

**Lost issues:**
- Check backups: `ls -la .beads-backup-*/`
- Export from backup: `bd list --all --json > recovery.json`

### Questions?

See `.beads/CLEANUP_PLAN.md` for full architectural analysis and rationale.
