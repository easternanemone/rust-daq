# PVCAM gRPC Real-World Hardware Scenario

This document defines the production-like scenario and pass/fail metrics for
end-to-end PVCAM gRPC validation on the maitai hardware host. The intent is to
exercise the full gRPC streaming path (daemon + hardware driver + client) under
conditions that resemble real usage, and to capture consistent metrics for
regression tracking.

## Scope

- Real Photometrics camera on maitai (Prime BSI today, Prime 95B when available).
- gRPC HardwareService streaming path (StartStream, StreamFrames, StopStream).
- Driver + daemon behavior under continuous streaming and client churn.
- Performance and stability metrics: FPS, latency, drops, errors, resource usage.

## Preconditions

- Host: `maitai@100.117.5.12`
- PVCAM SDK installed in `/opt/pvcam`
- Environment:
  - `PVCAM_SDK_DIR=/opt/pvcam/sdk`
  - `PVCAM_LIB_DIR=/opt/pvcam/library/x86_64`
  - `PVCAM_UMD_PATH=/opt/pvcam/drivers/user-mode`
  - `LD_LIBRARY_PATH=/opt/pvcam/library/x86_64:$LD_LIBRARY_PATH`
- Camera connected and visible to PVCAM CLI tools.
- Daemon built with real PVCAM support:
  - `--features 'instrument_photometrics,pvcam_hardware'`

## Target Scenarios

### Scenario A: Reliability Baseline (Primary)

Goal: Verify the gRPC stream stays alive for a long duration at a stable,
moderate frame rate and full sensor resolution.

- Camera: Prime BSI
- ROI: full sensor (2048 x 2048)
- Binning: 1x1
- Exposure: 100 ms (nominal ~10 FPS)
- Trigger mode: timed (internal)
- Duration: 30 minutes
- Clients: 1 gRPC stream subscriber

Expected behavior:
- StartStream succeeds; stream produces frames continuously.
- No stream termination unless StopStream is called.
- No PVCAM errors or gRPC error statuses during run.

### Scenario B: Stress Throughput (Secondary)

Goal: Ensure higher throughput does not stall callbacks or crash the stream.
Drops are acceptable, stalls are not.

- Camera: Prime BSI
- ROI: full sensor (2048 x 2048) or 512 x 512 for higher FPS runs
- Binning: 1x1
- Exposure: 10 ms or 5 ms (nominal 100 to 200 FPS, expect drops)
- Trigger mode: timed (internal)
- Duration: 5 minutes
- Clients: 1 gRPC stream subscriber

Expected behavior:
- Stream remains active (no sustained stall > 2 seconds).
- Frame drops may occur, but stream does not terminate unexpectedly.

### Scenario C: Multi-Client and Disconnect Behavior

Goal: Validate gRPC behavior when multiple subscribers connect/disconnect.

- Same configuration as Scenario A (use moderate exposure).
- Clients: 2 gRPC stream subscribers.
- Actions:
  - Start stream with client A connected.
  - Add client B mid-run.
  - Disconnect client B, then A.

Expected behavior (current implementation):
- Client disconnect triggers server-side stop_stream for the device.
- If this behavior is undesirable, record it and file a follow-up issue.

### Scenario D: Parameter Changes While Streaming

Goal: Ensure basic parameter updates do not crash the stream or corrupt state.

- Same configuration as Scenario A.
- During streaming, perform:
  - Exposure change (100 ms -> 50 ms -> 100 ms).
  - ROI change to quarter sensor and back.
  - Binning change to 2x2 and back.

Expected behavior:
- Parameter updates either succeed with continuous streaming or fail with a
  clear, recoverable error.
- Stream does not silently stop.

## Metrics and Pass Criteria

### Common Metrics

- Frames received: total count from gRPC client.
- StreamingMetrics:
  - `current_fps`
  - `frames_sent`
  - `frames_dropped`
  - `avg_latency_ms`
- Client observed latency:
  - `now_ns - frame.timestamp_ns`
- Errors:
  - gRPC status errors
  - timeouts waiting for frames
  - driver errors in daemon logs
- Resource usage:
  - CPU, memory, load averages on maitai during run.

### Pass Criteria

Scenario A:
- Effective FPS within 20 percent of expected for the exposure setting.
- Frame drop rate under 5 percent of frames_sent.
- No stall longer than 2 seconds.
- No PVCAM error codes or gRPC errors.

Scenario B:
- Stream remains active for duration; no sustained stall > 2 seconds.
- Drops are acceptable but should be captured and reported.

Scenario C:
- Document current behavior on disconnect and verify it is consistent.
- If a disconnect stops acquisition for all clients, log it as expected
  behavior or file follow-up if undesired.

Scenario D:
- Parameter update operations do not crash or deadlock streaming.
- If updates are unsupported while streaming, the error must be explicit and
  streaming should continue if possible.

## Output Artifacts

- Harness summary (text + structured metrics file).
- Daemon logs with `RUST_LOG=info,daq_pvcam=debug,daq_server=info`.
- If failures occur, capture the exact command line, parameters, and timestamps.

## Notes

- Use the existing PVCAM streaming stability knowledge: 100 ms exposure is the
  most reliable baseline. High FPS runs are expected to drop frames but must not
  stall.
- If the real-world run exposes gRPC throughput bottlenecks, capture raw
  metrics before optimizing to preserve reproducibility.
