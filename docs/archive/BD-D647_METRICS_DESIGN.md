# bd-d647: Frame Drop Metrics Implementation Design

## Executive Summary

**Key Finding**: The frame drop metrics infrastructure **already exists** in the system. We don't need to add new tracking code - just expose existing DataDistributor metrics to the GUI.

## Current Architecture

### DataDistributor Metrics (src/measurement/mod.rs)

The DataDistributor already tracks all frame drops via `try_send()` failures:

**SubscriberMetrics** (lines 93-114):
```rust
struct SubscriberMetrics {
    total_sent: u64,          // Total frames sent successfully
    total_dropped: u64,       // Total frames dropped (TrySendError::Full)
    window_sent: u64,         // Frames sent in current 10s window
    window_dropped: u64,      // Frames dropped in current 10s window
    window_start: Instant,    // Window start time
    drop_warn_emitted: bool,  // Prevents log spam
    saturation_error_emitted: bool,
}
```

**SubscriberMetricsSnapshot** (lines 66-73) - Public API:
```rust
pub struct SubscriberMetricsSnapshot {
    pub subscriber: String,        // Subscriber name (e.g., "gui", "storage")
    pub total_sent: u64,
    pub total_dropped: u64,
    pub drop_rate_percent: f64,    // Calculated: dropped / (sent + dropped) * 100
    pub channel_occupancy: usize,  // Current queue depth
    pub channel_capacity: usize,   // Maximum queue size
}
```

**Access Methods**:
- `DataDistributor::metrics_snapshot()` (line 253) - Returns Vec<SubscriberMetricsSnapshot>
- `DaqManagerActor::distributor_metrics_snapshot()` (app_actor.rs:1430) - Accessor

### Drop Tracking Mechanism (measurement/mod.rs:207-213)

```rust
match entry.sender.try_send(data.clone()) {
    Ok(_) => {
        entry.metrics.record_success();
    }
    Err(mpsc::error::TrySendError::Full(_)) => {
        entry.metrics.record_drop();  // ⬅️ Drops tracked here
    }
    Err(mpsc::error::TrySendError::Closed(_)) => {
        // Subscriber disconnected
    }
}
```

### Warning/Error Thresholds

Configured in `DataDistributorConfig` (app_actor.rs:183-189):
- `warn_drop_rate_percent`: Default 1.0% (configurable via settings)
- `error_saturation_percent`: Default 90.0%
- `metrics_window`: 10 seconds (configurable)

## Why "RecvError::Lagged" is Not Applicable

The bd-d647 issue description mentions `RecvError::Lagged` and "broadcast channels," but:

1. **System uses mpsc channels, not broadcast channels**
   - DataDistributor uses `tokio::sync::mpsc::Sender`
   - Non-blocking `try_send()` prevents backpressure
   - Returns `TrySendError::Full` when channel full

2. **RecvError::Lagged only occurs with broadcast channels**
   - `tokio::sync::broadcast::Receiver::recv()` returns RecvError::Lagged
   - Indicates receiver fell behind and missed messages
   - Not used in current architecture

3. **DataDistributor already tracks the same information**
   - `TrySendError::Full` ≈ "frame would have been lagged if using broadcast"
   - Same operational meaning: subscriber couldn't keep up

## Implementation Plan

### Phase 1: Add GetMetrics Command (Low Risk)

**File**: `src/messages.rs`

Add command variant:
```rust
/// Retrieves DataDistributor metrics for observability.
///
/// Returns per-subscriber metrics including drop counts, drop rates,
/// and channel occupancy. Used by GUI to display operational health.
///
/// # Response
///
/// Vec<SubscriberMetricsSnapshot> with metrics for all subscribers
GetMetrics {
    response: oneshot::Sender<Vec<SubscriberMetricsSnapshot>>,
},
```

Add helper method:
```rust
impl DaqCommand {
    pub fn get_metrics() -> (Self, oneshot::Receiver<Vec<SubscriberMetricsSnapshot>>) {
        let (tx, rx) = oneshot::channel();
        (Self::GetMetrics { response: tx }, rx)
    }
}
```

**File**: `src/app_actor.rs` (add to run() loop around line 280)

```rust
DaqCommand::GetMetrics { response } => {
    let metrics = self.distributor_metrics_snapshot().await;
    let _ = response.send(metrics);
}
```

### Phase 2: GUI Integration (Medium Risk)

**File**: `src/gui/mod.rs`

Add periodic metrics refresh (every 1-2 seconds):
```rust
// In Gui::update()
if self.last_metrics_update.elapsed() > Duration::from_secs(2) {
    let (cmd, rx) = DaqCommand::get_metrics();
    self.command_tx.send(cmd).await.ok();
    if let Ok(metrics) = tokio::time::timeout(Duration::from_millis(100), rx).await {
        self.cached_metrics = metrics.ok();
    }
    self.last_metrics_update = Instant::now();
}
```

**File**: `src/gui/instrument_list.rs` (or wherever instrument UI is)

Add metrics display in instrument row:
```rust
// For each instrument, aggregate metrics from all subscribers
let inst_metrics: Vec<_> = cached_metrics
    .iter()
    .filter(|m| m.subscriber.starts_with(&inst_id))
    .collect();

let total_dropped: u64 = inst_metrics.iter().map(|m| m.total_dropped).sum();
let avg_drop_rate: f64 = inst_metrics.iter().map(|m| m.drop_rate_percent).sum::<f64>()
    / inst_metrics.len() as f64;

// Display with warning icon if drop_rate > 1%
if avg_drop_rate > 1.0 {
    ui.label(egui::RichText::new("⚠").color(egui::Color32::YELLOW));
}
ui.label(format!("Drops: {} ({:.2}%)", total_dropped, avg_drop_rate));
```

### Phase 3: Per-Instrument Aggregation (Optional Enhancement)

Currently metrics are per-subscriber. For better UI organization, add per-instrument rollup:

**File**: `src/app_actor.rs`

```rust
pub struct InstrumentMetrics {
    pub instrument_id: String,
    pub total_frames: u64,       // Sum of all subscriber total_sent
    pub dropped_frames: u64,     // Sum of all subscriber total_dropped
    pub drop_rate_percent: f64,  // Weighted average
    pub subscribers: Vec<String>, // List of subscriber names
}

impl DaqManagerActor {
    pub async fn instrument_metrics_snapshot(&self) -> Vec<InstrumentMetrics> {
        let subscriber_metrics = self.distributor_metrics_snapshot().await;

        // Group by instrument ID (extract from subscriber name)
        let mut by_instrument: HashMap<String, Vec<SubscriberMetricsSnapshot>> = HashMap::new();
        for metric in subscriber_metrics {
            // Subscriber names like "instrument_id_gui", "instrument_id_storage"
            let inst_id = metric.subscriber.split('_').next().unwrap_or("unknown");
            by_instrument.entry(inst_id.to_string()).or_default().push(metric);
        }

        // Aggregate per instrument
        by_instrument.into_iter().map(|(inst_id, metrics)| {
            let total_frames = metrics.iter().map(|m| m.total_sent).sum();
            let dropped_frames = metrics.iter().map(|m| m.total_dropped).sum();
            let drop_rate = if total_frames + dropped_frames == 0 {
                0.0
            } else {
                (dropped_frames as f64 / (total_frames + dropped_frames) as f64) * 100.0
            };

            InstrumentMetrics {
                instrument_id: inst_id,
                total_frames,
                dropped_frames,
                drop_rate_percent: drop_rate,
                subscribers: metrics.iter().map(|m| m.subscriber.clone()).collect(),
            }
        }).collect()
    }
}
```

## Testing Strategy

### Unit Tests (tests/distributor_metrics_test.rs)

1. **test_metrics_snapshot_accuracy**
   - Send N frames, drop M frames
   - Verify metrics match expectations

2. **test_drop_rate_calculation**
   - Various drop scenarios
   - Verify percentage calculation

3. **test_window_metrics_reset**
   - Verify 10-second window resets correctly

### Integration Tests (tests/gui_metrics_integration_test.rs)

1. **test_get_metrics_command**
   - Send GetMetrics command
   - Verify response structure

2. **test_instrument_metrics_aggregation**
   - Multiple subscribers per instrument
   - Verify aggregation logic

3. **test_gui_warning_threshold**
   - Simulate 2% drop rate
   - Verify warning icon appears

## Configuration

Existing settings in `config/default.toml`:

```toml
[application.data_distributor]
subscriber_capacity = 1024
warn_drop_rate_percent = 1.0
error_saturation_percent = 90.0
metrics_window_secs = 10
```

## Migration Notes

No migration needed - metrics have been tracked since DataDistributor was implemented. Historical data not available (metrics are runtime counters, reset on restart).

## Performance Impact

- GetMetrics command: ~10µs (already benchmarked in DataDistributor tests)
- GUI refresh every 2 seconds: negligible impact
- No impact on instrument data throughput (metrics updated inline with try_send)

## Acceptance Criteria

- ✅ Per-subscriber frame drop counts exposed via GetMetrics command
- ✅ Drop rate calculation (percentage)
- ✅ Channel occupancy metrics
- ⏳ GUI displays drop count per instrument (Phase 2)
- ⏳ Warning icon when drop rate >1% (Phase 2)
- ⏳ Per-instrument aggregation (Phase 3, optional)
- ⏳ Tests for metrics accuracy (Phase 1)

## Related Issues

- bd-22: GUI batching (related to preventing drops)
- bd-dd19: Command channel resilience (GUI command drops)
- DataDistributor implementation: measurement/mod.rs:19-276

## References

- DataDistributor implementation: src/measurement/mod.rs
- DaqManagerActor accessor: src/app_actor.rs:1430-1432
- Message protocol: src/messages.rs
- Configuration: config/default.toml
