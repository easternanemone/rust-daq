# daq-driver-pvcam

PVCAM camera driver crate used by rust-daq. Supports mock mode by default and hardware mode when the PVCAM SDK is available.

## Features

- `mock` (default): builds without the PVCAM SDK, uses mock acquisition.
- `pvcam_hardware`: enables PVCAM SDK bindings (requires env vars).
- `arrow_tap`: exposes an Arrow tap channel to receive frames as `UInt16Array`.

## Environment (hardware)

Set before building and running hardware tests/examples:

```bash
# CRITICAL: PVCAM_VERSION is required at runtime or you get Error 151
export PVCAM_VERSION=7.1.1.118

# SDK and library locations
export PVCAM_SDK_DIR=/opt/pvcam/sdk
export PVCAM_LIB_DIR=/opt/pvcam/library/x86_64

# LIBRARY_PATH for linker (required for cargo build)
export LIBRARY_PATH=$PVCAM_LIB_DIR:$LIBRARY_PATH

# LD_LIBRARY_PATH for runtime (required for execution)
export LD_LIBRARY_PATH=/opt/pvcam/drivers/user-mode:$PVCAM_LIB_DIR:$LD_LIBRARY_PATH
```

**Quick setup:** Source the PVCAM profile, then add linker path:
```bash
source /etc/profile.d/pvcam.sh
export LIBRARY_PATH=/opt/pvcam/library/x86_64:$LIBRARY_PATH
```

For detailed setup and troubleshooting (especially for "Failed to start acquisition" errors), see [PVCAM Setup & Troubleshooting](../../docs/troubleshooting/PVCAM_SETUP.md).

## Testing

### Mock Tests

```bash
cargo test -p daq-driver-pvcam --no-default-features
```

### Hardware Smoke Tests

Comprehensive hardware validation suite (requires `PVCAM_SMOKE_TEST=1`):

```bash
source /etc/profile.d/pvcam.sh
export LIBRARY_PATH=/opt/pvcam/library/x86_64:$LIBRARY_PATH
export PVCAM_SMOKE_TEST=1

cargo test -p daq-driver-pvcam --test hardware_smoke --features pvcam_hardware -- --nocapture --test-threads=1
```

| Test | Description |
|------|-------------|
| `pvcam_smoke_test` | Basic connectivity, exposure, single frame |
| `pvcam_camera_info_test` | Camera resolution verification |
| `pvcam_multiple_frames_test` | Acquire 5 consecutive frames |
| `pvcam_streaming_test` | Continuous streaming for 1 second |
| `pvcam_exposure_range_test` | Test various exposure times (1-500ms) |
| `pvcam_frame_statistics_test` | Validate pixel data statistics |

## Examples

- Arrow tap (hardware + Arrow):

  ```bash
  # First set up environment (see above), then:
  cargo run -p daq-driver-pvcam --example arrow_tap --features "pvcam_hardware,arrow_tap" -- PrimeBSI
  ```
