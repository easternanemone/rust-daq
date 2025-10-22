# Rollback Instructions for Actor Model (Phase 1)

This document explains how to rollback from the actor-based implementation to the previous Arc<Mutex<>> implementation if issues are discovered.

## Quick Rollback

If you need to quickly revert to the old implementation:

```bash
# 1. Restore the old app.rs
mv src/app.rs src/app_actor_version.rs
mv src/app_v1_backup.rs src/app.rs

# 2. Remove actor-specific files
rm src/app_actor.rs
rm src/messages.rs

# 3. Update lib.rs to remove actor modules
# Remove these lines from src/lib.rs:
#   pub mod app_actor;
#   pub mod messages;

# 4. Verify compilation
cargo check

# 5. Run tests
cargo test
```

## What Changed in Phase 1

The Phase 1 actor model refactoring (bd-61) replaced:

- **Old**: `Arc<Mutex<DaqAppInner>>` - shared mutable state with explicit locking
- **New**: `DaqManagerActor` + message-passing - actor owns state, GUI sends commands via mpsc channels

### Files Modified

1. **src/app.rs** (replaced, backup at `src/app_v1_backup.rs`)
   - Changed from Arc<Mutex<>> wrapper to message-passing wrapper
   - Backwards-compatible `with_inner()` method preserved

2. **src/app_actor.rs** (new)
   - Contains `DaqManagerActor` struct
   - Async event loop processing commands

3. **src/messages.rs** (new)
   - `DaqCommand` enum with 13 command variants
   - Helper methods for creating commands with oneshot channels

4. **src/lib.rs** (updated)
   - Added `pub mod app_actor;` (line 34)
   - Added `pub mod messages;` (line 42)

### GUI Compatibility

The GUI continues to work without changes because:
- `DaqApp::with_inner()` method preserved
- `DaqAppCompat` provides same interface as old `DaqAppInner`
- All field access routes through message-passing transparently

## If Performance Degrades

If you notice performance issues after the actor migration:

1. **Run benchmarks** to confirm regression:
   ```bash
   cargo test --release -- --nocapture | grep "fps\|latency"
   ```

2. **Check actor queue depth**:
   - Look for "Failed to send" errors in logs
   - Increase channel capacity in `src/app.rs` line 59: `mpsc::channel(256)` → `mpsc::channel(1024)`

3. **Profile blocking operations**:
   - GUI uses `blocking_send`/`blocking_recv` which may cause frame drops
   - Consider switching GUI to async if fps < 55

## Phase 1 Acceptance Criteria

Before considering this migration stable, verify:

- [ ] GUI maintains >55 fps with 10 active instruments
- [ ] Successfully spawn/stop 20 instruments concurrently
- [ ] 100/100 session save/load cycles complete without errors
- [ ] No mutex poisoning errors in logs
- [ ] Integration tests pass (bd-60)

## Related Issues

- bd-61: Phase 1 Actor Model migration
- bd-60: Integration test harness
- bd-62: Phase 2 V2 Native Integration (depends on bd-61)
