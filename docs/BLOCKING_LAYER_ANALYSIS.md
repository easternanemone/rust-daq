# DaqApp Blocking Layer Analysis

**Date**: 2025-11-03
**Issue**: bd-cd89 (Phase 2.4 - Remove blocking compatibility layer)
**Author**: Refactor Worker

## Executive Summary

The `DaqApp` wrapper in `src/app.rs` uses `blocking_send()` in async contexts, creating a performance bottleneck that negates the benefits of the actor-based architecture. This analysis documents the blocking operations and proposes a refactoring strategy to eliminate them.

## Current Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         main.rs                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Creates DaqApp<M> wrapper                           │   │
│  │  Passes to Gui::new()                                │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       DaqApp (src/app.rs)                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  BLOCKING LAYER (compatibility wrapper)              │   │
│  │  • command_tx: mpsc::Sender<DaqCommand>             │   │
│  │  • Uses blocking_send() for all operations          │   │
│  │  • Uses blocking_recv() for responses               │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼ (spawned in Runtime)
┌─────────────────────────────────────────────────────────────┐
│              DaqManagerActor (src/app_actor.rs)              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  ASYNC ACTOR (owns all state)                        │   │
│  │  • Runs in dedicated Tokio task                      │   │
│  │  • Processes commands sequentially                   │   │
│  │  • Responds via oneshot channels                     │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼ (broadcasts)
┌─────────────────────────────────────────────────────────────┐
│                    DataDistributor                           │
│  • Broadcasts Measurement to subscribers                     │
│  • GUI subscribes via mpsc::Receiver<Arc<Measurement>>      │
└─────────────────────────────────────────────────────────────┘
```

## Blocking Operations Identified

### 1. Instrument Spawning (app.rs:75-82)

```rust
for id in instrument_ids {
    let (cmd, rx) = DaqCommand::spawn_instrument(id.clone());
    if command_tx.blocking_send(cmd).is_ok() {  // ❌ BLOCKS
        if let Ok(result) = rx.blocking_recv() {  // ❌ BLOCKS
            if let Err(e) = result {
                log::error!("Failed to spawn instrument '{}': {}", id, e);
            }
        }
    }
}
```

**Impact**: Startup time scales linearly with number of instruments. Each spawn blocks until actor responds.

### 2. Shutdown (app.rs:100-109)

```rust
pub fn shutdown(&self) -> Result<()> {
    let (cmd, rx) = DaqCommand::shutdown();
    self.command_tx
        .blocking_send(cmd)  // ❌ BLOCKS
        .map_err(|_| anyhow::anyhow!("Failed to send shutdown command"))?;
    rx.blocking_recv()  // ❌ BLOCKS
        .map_err(|_| anyhow::anyhow!("Failed to receive shutdown response"))?
        .map_err(|e| anyhow::anyhow!("Shutdown error: {}", e))?;
    info!("Application shutdown complete");
    Ok(())
}
```

**Impact**: Main thread blocks during entire shutdown sequence (5s timeout per instrument).

### 3. Session Operations (app.rs:113-130)

```rust
pub fn save_session(&self, path: &Path, gui_state: session::GuiState) -> Result<()> {
    let (cmd, rx) = DaqCommand::save_session(path.to_path_buf(), gui_state);
    self.command_tx
        .blocking_send(cmd)  // ❌ BLOCKS
        .map_err(|_| anyhow::anyhow!("Failed to send save session command"))?;
    rx.blocking_recv()  // ❌ BLOCKS
        .map_err(|_| anyhow::anyhow!("Failed to receive save session response"))?
}

pub fn load_session(&self, path: &Path) -> Result<session::GuiState> {
    let (cmd, rx) = DaqCommand::load_session(path.to_path_buf());
    self.command_tx
        .blocking_send(cmd)  // ❌ BLOCKS
        .map_err(|_| anyhow::anyhow!("Failed to send load session command"))?;
    rx.blocking_recv()  // ❌ BLOCKS
        .map_err(|_| anyhow::anyhow!("Failed to receive load session response"))?
}
```

**Impact**: GUI freezes during file I/O operations (serialization, loading).

### 4. DaqAppCompat Layer (app.rs:164-326)

The entire `DaqAppCompat` struct uses `blocking_send()` for backwards compatibility:

- `spawn_instrument()` (line 186)
- `stop_instrument()` (line 197)
- `send_instrument_command()` (line 209)
- `set_storage_format()` (line 239)
- `start_recording()` (line 247)
- `stop_recording()` (line 256)
- `subscribe()` in DaqDataSender (line 274)
- `len()`, `keys()`, `contains_key()` in DaqInstruments (lines 292-311)
- `clone()` in DaqStorageFormat (line 324)

**Impact**: Every test using `with_inner()` blocks async contexts, preventing concurrent operations.

## Performance Impact Analysis

### Current Behavior

1. **Sequential Operations**: All GUI interactions wait for actor responses
2. **Thread Blocking**: `blocking_send()` may block if channel is full (capacity 32)
3. **Latency Amplification**: Network of blocking calls creates cascading delays
4. **Poor Scalability**: Adding more instruments increases startup latency linearly

### Example Scenarios

#### Scenario 1: Spawning 10 Instruments

**Current** (with blocking):
- 10 × (connect time + blocking_send + blocking_recv)
- If each instrument takes 200ms to connect: **2+ seconds blocked**

**After Refactoring** (async):
- Concurrent spawning via `futures::future::join_all()`
- Parallel connections: **200ms total** (fastest instrument)

#### Scenario 2: Recording + Parameter Updates

**Current**:
- User clicks "Start Recording" → GUI blocks
- Actor processes command → blocks while starting storage writer
- Meanwhile, parameter updates queue up or drop

**After Refactoring**:
- User clicks "Start Recording" → GUI remains responsive
- Commands processed asynchronously
- Parameter updates processed concurrently

## Proposed Refactoring Strategy

### Phase 1: Remove DaqApp Wrapper

**Goal**: Eliminate `src/app.rs` entirely, have GUI communicate directly with actor.

#### Changes Required

1. **main.rs**
   - Remove `DaqApp::new()` call
   - Create actor and command channel directly
   - Spawn actor task
   - Pass `command_tx` directly to GUI

2. **gui/mod.rs**
   - Replace `DaqApp<M>` field with `mpsc::Sender<DaqCommand>`
   - Add `Arc<Runtime>` field for spawning tasks
   - Add `Arc<Settings>` field for configuration access
   - Add `LogBuffer` field (already exists)
   - Add `Arc<InstrumentRegistry<M>>` for available channels

3. **Async GUI Methods**
   - Convert all `with_inner()` calls to async message sends
   - Use `ctx.repaint_after()` for async result handling
   - Implement non-blocking UI feedback during operations

### Phase 2: Instrument Spawning Strategy

**Option A**: Concurrent Spawning (Recommended)

```rust
// In actor initialization (before starting event loop)
let spawn_futures: Vec<_> = instrument_ids
    .into_iter()
    .map(|id| async move {
        let (cmd, rx) = DaqCommand::spawn_instrument(id.clone());
        command_tx.send(cmd).await?;
        rx.await?
    })
    .collect();

// Spawn all concurrently
let results = futures::future::join_all(spawn_futures).await;
```

**Option B**: Sequential Async (Simpler Migration)

```rust
for id in instrument_ids {
    let (cmd, rx) = DaqCommand::spawn_instrument(id);
    command_tx.send(cmd).await?;
    rx.await?;
}
```

### Phase 3: GUI Async Integration

#### Challenge: egui is Synchronous

egui's `update()` method is synchronous, but we need async operations. Solutions:

**Solution 1**: Spawn Background Tasks

```rust
// In GUI button handler
if ui.button("Start Recording").clicked() {
    let cmd_tx = self.command_tx.clone();
    self.runtime.spawn(async move {
        let (cmd, rx) = DaqCommand::start_recording();
        if cmd_tx.send(cmd).await.is_ok() {
            let _ = rx.await;
        }
    });
}
```

**Solution 2**: Polling Pattern (Current Approach)

```rust
// GUI tracks pending operations
struct PendingOperation {
    rx: oneshot::Receiver<Result<()>>,
    status: &'static str,
}

// In update() loop
for (op_id, pending) in &mut self.pending_operations {
    if let Ok(result) = pending.rx.try_recv() {
        // Operation completed, update UI
        self.pending_operations.remove(op_id);
    }
}
```

## Migration Checklist

### Step 1: Update main.rs

- [x] Analyze current structure
- [ ] Remove DaqApp wrapper
- [ ] Create command channel directly
- [ ] Spawn actor task explicitly
- [ ] Pass components to GUI individually

### Step 2: Update Gui struct

- [ ] Replace `app: DaqApp<M>` with:
  - `command_tx: mpsc::Sender<DaqCommand>`
  - `runtime: Arc<Runtime>`
  - `settings: Arc<Settings>`
  - `instrument_registry: Arc<InstrumentRegistry<M>>`
- [ ] Add pending operations tracking
- [ ] Update `new()` constructor signature

### Step 3: Convert GUI Operations

- [ ] `spawn_instrument()` → async send
- [ ] `stop_instrument()` → async send
- [ ] `start_recording()` → async send
- [ ] `stop_recording()` → async send
- [ ] `save_session()` → async send
- [ ] `load_session()` → async send
- [ ] `send_instrument_command()` → async send

### Step 4: Update Tests

- [ ] Refactor tests using `with_inner()`
- [ ] Create async test helpers
- [ ] Update integration tests

### Step 5: Remove Compatibility Layer

- [ ] Delete `DaqApp` struct (src/app.rs)
- [ ] Delete `DaqAppCompat` struct
- [ ] Delete helper structs (DaqDataSender, DaqInstruments, DaqStorageFormat)
- [ ] Verify no references remain

## Performance Expectations

### Before (Blocking)

- Startup: **2-5 seconds** (10 instruments)
- GUI freeze during operations: **Up to 5 seconds**
- Throughput: **Sequential only**

### After (Async)

- Startup: **<500ms** (concurrent spawning)
- GUI freeze: **Never** (non-blocking operations)
- Throughput: **Full async pipeline**

## Risks and Mitigation

### Risk 1: Breaking Test Suite

**Mitigation**:
- Run `cargo test` after each change
- Update tests incrementally
- Maintain backwards compatibility temporarily

### Risk 2: GUI Responsiveness Regressions

**Mitigation**:
- Implement polling for pending operations
- Add loading indicators
- Test with slow instruments (simulated delays)

### Risk 3: Actor Overload

**Mitigation**:
- Monitor command channel capacity
- Implement backpressure if needed
- Add metrics for queue depth

## Next Steps

1. **Validate Analysis**: Review with team, confirm approach
2. **Create Feature Branch**: `feature/remove-blocking-layer`
3. **Implement Phase 1**: Update main.rs and Gui struct
4. **Run Tests**: Ensure no regressions
5. **Implement Phase 2**: Async instrument spawning
6. **Implement Phase 3**: GUI async integration
7. **Performance Testing**: Measure improvements
8. **Documentation**: Update CLAUDE.md and architecture docs
9. **Cleanup**: Remove deprecated code

## References

- **bd-cd89**: Remove blocking DaqApp compatibility layer
- **src/app.rs**: Current blocking implementation
- **src/app_actor.rs**: Actor-based state management
- **src/gui/mod.rs**: GUI implementation
- **src/messages.rs**: Command definitions
