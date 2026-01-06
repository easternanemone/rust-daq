# Quick Start: Beads Cleanup

## TL;DR - Safe Execution

```bash
cd /Users/briansquires/code/rust-daq/.beads/scripts

# Run all safe operations (Steps 1-3)
./01-verify-deletions.sh
./02-label-dead-architectures.sh
./03-label-v5-active.sh

# Review the output, then proceed with destructive operations
# (Each will ask for confirmation)
./04-close-dead-issues.sh
./05-run-compaction.sh

# Check results
./06-maintenance-schedule.sh
```

## What This Does

**Problem:**
- 433 issues, but only 29 have labels (6.7%)
- Mix of V1, V2, V4, and V5 architecture issues
- 64 old closed issues eligible for compaction
- Hard to find relevant work

**Solution:**
1. **Label** dead architecture issues (V1, V2, V4) → `arch:v4-dead`
2. **Label** active V5 work → `arch:v5-active`, `priority:critical`
3. **Close** obsolete V4/GUI issues
4. **Compact** old closed issues (30+ days)
5. **Result:** 80%+ labeled, easy filtering, ~100KB savings

## Safety Guarantees

✅ Every destructive operation creates automatic backup
✅ Confirmation required before changes
✅ Clear output of what changed
✅ Rollback instructions provided
✅ Read-only verification steps first

## Key Queries After Cleanup

```bash
# See active V5 work
bd list --label arch:v5-active

# See critical blocking issues
bd list --label priority:critical

# See what's obsolete
bd list --label arch:v4-dead

# See driver migration work
bd list --label component:driver

# See safety-critical work
bd list --label priority:safety

# Find unlabeled issues (should be <20%)
bd list --all --json | jq '.[] | select(.labels | length == 0) | .id'
```

## Expected Timeline

- Steps 1-3 (safe labeling): ~2 minutes
- Step 4 (close issues): ~1 minute + review time
- Step 5 (compaction): ~1 minute
- Total: **~5-10 minutes** for complete cleanup

## What Gets Closed

**V4 Architecture (8 issues):**
- All Kameo actor system work
- V4 production deployment tasks
- V4 performance validation

**Obsolete GUI (1-2 issues):**
- GUI is separate app in V5 headless

**Total:** ~10 issues closed, ~60 issues compacted

## What's Preserved

✅ All V5 active work
✅ Driver migration tasks
✅ Capability trait issues
✅ Headless architecture work
✅ All closed issue history (in compressed form)

## Rollback

If anything goes wrong:
```bash
# List backups
ls -la .beads-backup-*/

# Restore
mv .beads .beads-broken
mv .beads-backup-TIMESTAMP .beads
bd daemon &
```

## Next Steps After Cleanup

1. **Verify health:**
   ```bash
   bd list | head -20
   bd list --label arch:v5-active
   ```

2. **Review unlabeled:**
   ```bash
   bd list --all --json | jq '.[] | select(.labels | length == 0) | .id'
   ```

3. **Add missing labels:**
   ```bash
   bd label add <issue-id> component:driver
   bd label add <issue-id> arch:v5-active
   ```

4. **Monthly maintenance:**
   ```bash
   ./06-maintenance-schedule.sh
   ```

## Questions?

- Full plan: `.beads/CLEANUP_PLAN.md`
- Script details: `.beads/scripts/README.md`
- Architectural analysis: See original CLAUDE.md context
