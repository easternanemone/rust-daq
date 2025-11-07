# Phase 1: Remove DaqApp Blocking Wrapper

**Issue**: bd-cd89
**Date**: 2025-11-03
**Status**: In Progress

## Goal

Remove `src/app.rs` blocking compatibility layer and have GUI communicate directly with `DaqManagerActor` via async channels.

## Current Call Pattern

```
GUI → DaqApp::with_inner() → DaqAppCompat → blocking_send() → Actor
                                              blocking_recv() ←
```

## Target Call Pattern

```
GUI → Runtime::spawn() → async send() → Actor
                         async recv() ←
      Update UI via polling/callbacks
```

## Step-by-Step Implementation

### Step 1: Update main.rs Structure

**Current** (main.rs):
```rust
let app = DaqApp::new(settings, registry, ...)?;
let app_clone = app.clone();
Gui::new(cc, app_clone)
```

**After**:
```rust
// Create actor
let actor = DaqManagerActor::new(settings.clone(), ...)?;

// Create command channel
let (command_tx, command_rx) = mpsc::channel(settings.application.command_channel_capacity);

// Spawn actor task
let actor_handle = runtime.spawn(async move {
    actor.run(command_rx).await;
});

// Pass components to GUI
Gui::new(
    cc,
    command_tx,
    runtime.clone(),
    settings,
    instrument_registry,
    log_buffer,
)
```

### Step 2: Update Gui Struct

**Current** (gui/mod.rs:138-172):
```rust
pub struct Gui<M> {
    app: DaqApp<M>,
    data_receiver: mpsc::Receiver<Arc<Measurement>>,
    // ... other fields
}
```

**After**:
```rust
pub struct Gui<M> {
    // Direct actor communication
    command_tx: mpsc::Sender<DaqCommand>,
    runtime: Arc<Runtime>,

    // Configuration access (read-only)
    settings: Arc<Settings>,
    instrument_registry: Arc<InstrumentRegistry<M>>,

    // Data and logging
    data_receiver: mpsc::Receiver<Arc<Measurement>>,
    log_buffer: LogBuffer,

    // Pending async operations
    pending_operations: HashMap<String, PendingOperation>,

    // ... existing UI state fields
}

struct PendingOperation {
    rx: oneshot::Receiver<Result<()>>,
    description: String,
    started_at: Instant,
}
```

### Step 3: Update Gui::new() Constructor

**Current** (gui/mod.rs:180-205):
```rust
pub fn new(_cc: &eframe::CreationContext<'_>, app: DaqApp<M>) -> Self {
    let (data_receiver, log_buffer) =
        app.with_inner(|inner| (inner.data_sender.subscribe(), inner.log_buffer.clone()));
    // ...
}
```

**After**:
```rust
pub fn new(
    _cc: &eframe::CreationContext<'_>,
    command_tx: mpsc::Sender<DaqCommand>,
    runtime: Arc<Runtime>,
    settings: Arc<Settings>,
    instrument_registry: Arc<InstrumentRegistry<M>>,
    log_buffer: LogBuffer,
) -> Self {
    // Subscribe to data stream via command
    let data_receiver = {
        let (cmd, rx) = DaqCommand::subscribe_to_data();
        command_tx.blocking_send(cmd).ok(); // Temporary blocking for initialization
        rx.blocking_recv().unwrap_or_else(|_| {
            let (tx, rx) = mpsc::channel(1);
            drop(tx);
            rx
        })
    };

    Self {
        command_tx,
        runtime,
        settings,
        instrument_registry,
        data_receiver,
        log_buffer,
        pending_operations: HashMap::new(),
        // ... initialize other fields
    }
}
```

### Step 4: Convert GUI Operations to Async

#### Pattern A: Fire-and-Forget (No Response Needed)

```rust
// Old (blocking)
app.with_inner(|inner| inner.stop_instrument(id));

// New (async fire-and-forget)
let cmd_tx = self.command_tx.clone();
let id = id.to_string();
self.runtime.spawn(async move {
    let (cmd, rx) = DaqCommand::stop_instrument(id);
    if cmd_tx.send(cmd).await.is_ok() {
        let _ = rx.await; // Discard result
    }
});
```

#### Pattern B: Track Operation (Show Status/Errors)

```rust
// Old (blocking)
app.with_inner(|inner| {
    if let Err(e) = inner.spawn_instrument(id) {
        error!("Failed to spawn: {}", e);
    }
});

// New (async with tracking)
if ui.button("Start").clicked() {
    let cmd_tx = self.command_tx.clone();
    let id = id.to_string();
    let op_id = format!("spawn_{}", id);

    let (cmd, rx) = DaqCommand::spawn_instrument(id);
    if cmd_tx.try_send(cmd).is_ok() {
        // Track pending operation
        self.pending_operations.insert(op_id, PendingOperation {
            rx,
            description: format!("Starting {}", id),
            started_at: Instant::now(),
        });
    }
}
```

#### Pattern C: Synchronous Read (Temporary Blocking)

For read-only operations during initialization:

```rust
// Acceptable temporary blocking during GUI initialization
let instruments = {
    let (cmd, rx) = DaqCommand::get_instrument_list();
    self.command_tx.blocking_send(cmd).ok();
    rx.blocking_recv().unwrap_or_default()
};
```

### Step 5: Add Pending Operations Polling

In `Gui::update()`:

```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // Poll pending operations (non-blocking)
    let mut completed = Vec::new();
    for (op_id, pending) in &mut self.pending_operations {
        match pending.rx.try_recv() {
            Ok(Ok(())) => {
                info!("Operation '{}' completed successfully", pending.description);
                completed.push(op_id.clone());
            }
            Ok(Err(e)) => {
                error!("Operation '{}' failed: {}", pending.description, e);
                completed.push(op_id.clone());
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                // Still pending, check timeout
                if pending.started_at.elapsed() > Duration::from_secs(30) {
                    error!("Operation '{}' timed out", pending.description);
                    completed.push(op_id.clone());
                }
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                error!("Operation '{}' actor closed channel", pending.description);
                completed.push(op_id.clone());
            }
        }
    }

    // Remove completed operations
    for op_id in completed {
        self.pending_operations.remove(&op_id);
    }

    // ... rest of update logic
}
```

### Step 6: Update Instrument Panel

**Current** (gui/mod.rs:660-674):
```rust
if ui.button("Stop").clicked() {
    app.with_inner(|inner| inner.stop_instrument(id));
}
// ...
if ui.button("Start").clicked() {
    app.with_inner(|inner| {
        if let Err(e) = inner.spawn_instrument(id) {
            error!("Failed to start instrument '{}': {}", id, e);
        }
    });
}
```

**After**:
```rust
if ui.button("Stop").clicked() {
    let cmd_tx = self.command_tx.clone();
    let id = id.to_string();
    self.runtime.spawn(async move {
        let (cmd, rx) = DaqCommand::stop_instrument(id);
        if cmd_tx.send(cmd).await.is_ok() {
            let _ = rx.await;
        }
    });
}

if ui.button("Start").clicked() {
    let cmd_tx = self.command_tx.clone();
    let id = id.to_string();
    let op_id = format!("spawn_{}", id);

    let (cmd, rx) = DaqCommand::spawn_instrument(id.clone());
    if cmd_tx.try_send(cmd).is_ok() {
        self.pending_operations.insert(op_id, PendingOperation {
            rx,
            description: format!("Starting {}", id),
            started_at: Instant::now(),
        });
    }
}
```

### Step 7: Update Instrument Control Panels

Instrument control panels need access to command_tx and runtime:

```rust
// In DockTabViewer
struct DockTabViewer<'a, M> {
    command_tx: &'a mpsc::Sender<DaqCommand>,
    runtime: &'a Arc<Runtime>,
    available_channels: Vec<String>,
    data_cache: &'a HashMap<String, Arc<Measurement>>,
}
```

Then update each control panel's `ui()` method to accept these parameters.

### Step 8: Handle Instrument List Retrieval

**Current** (gui/mod.rs:460-468):
```rust
let (instruments, available_channels) = self.app.with_inner(|inner| {
    let instruments: Vec<(String, toml::Value, bool)> = inner
        .settings
        .instruments
        .iter()
        .map(|(k, v)| (k.clone(), v.clone(), inner.instruments.contains_key(k)))
        .collect();
    (instruments, inner.get_available_channels())
});
```

**After**:
```rust
// Use local settings for instrument config
let instruments: Vec<(String, toml::Value, bool)> = self
    .settings
    .instruments
    .iter()
    .map(|(k, v)| {
        // Query running status via command
        let is_running = {
            let (cmd, rx) = DaqCommand::get_instrument_list();
            self.command_tx.blocking_send(cmd).ok();
            rx.blocking_recv()
                .map(|list| list.contains(k))
                .unwrap_or(false)
        };
        (k.clone(), v.clone(), is_running)
    })
    .collect();

// Use local registry for available channels
let available_channels: Vec<String> = self.instrument_registry.list().collect();
```

**Optimization**: Cache instrument list and refresh periodically instead of querying every frame.

### Step 9: Optimize with Caching

Add to Gui struct:

```rust
// Cached instrument state (refreshed every N frames)
instrument_status_cache: HashMap<String, bool>, // id → is_running
cache_refresh_counter: u32,
const CACHE_REFRESH_INTERVAL: u32 = 60; // Refresh every 60 frames (~1 second)
```

In update():

```rust
// Refresh instrument status cache periodically
self.cache_refresh_counter = self.cache_refresh_counter.wrapping_add(1);
if self.cache_refresh_counter % Self::CACHE_REFRESH_INTERVAL == 0 {
    let cmd_tx = self.command_tx.clone();
    let cache = Arc::new(Mutex::new(self.instrument_status_cache.clone()));

    self.runtime.spawn(async move {
        let (cmd, rx) = DaqCommand::get_instrument_list();
        if cmd_tx.send(cmd).await.is_ok() {
            if let Ok(list) = rx.await {
                let mut cache_guard = cache.lock().unwrap();
                cache_guard.clear();
                for id in list {
                    cache_guard.insert(id, true);
                }
            }
        }
    });
}
```

## Testing Strategy

### Unit Tests

1. Test pending operations polling
2. Test timeout handling
3. Test cache refresh logic

### Integration Tests

1. Test GUI startup with actor
2. Test instrument spawn/stop through GUI
3. Test recording start/stop
4. Test session save/load

### Manual Testing

1. Start application, verify no blocking
2. Spawn multiple instruments, check concurrent startup
3. Click rapidly on buttons, verify no freezing
4. Monitor actor command queue depth

## Rollback Plan

If issues arise:

1. Keep `src/app.rs` but mark as `#[deprecated]`
2. Add feature flag `legacy-blocking-gui`
3. Implement both code paths
4. Switch based on feature flag

## Success Criteria

- ✓ No `blocking_send()` calls in GUI update loop
- ✓ No `blocking_recv()` calls in GUI update loop
- ✓ Startup time < 500ms (10 instruments)
- ✓ GUI remains responsive during all operations
- ✓ All tests pass
- ✓ No performance regressions

## Implementation Order

1. Update `main.rs` to create actor directly
2. Update `Gui::new()` signature and constructor
3. Add pending operations tracking
4. Convert instrument panel operations
5. Convert control panel operations
6. Add caching layer
7. Remove `src/app.rs`
8. Update all tests

## Next Steps

Begin implementation of Step 1 (main.rs refactoring).
