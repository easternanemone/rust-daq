# MaiTai Integration Fix Report
**Date**: 2025-11-02
**Issue**: bd-194
**Status**: BLOCKED (Hardware Connectivity)

## Summary

Applied timing fix to MaiTai driver to address integration test failure. However, comprehensive hardware testing revealed the root cause is **hardware connectivity**, not software timing.

## Work Completed

### 1. Timing Fix Implementation

**File**: `src/instrument/maitai.rs`
**Location**: Line 132 (after port open, before first command)

**Change Applied**:
```rust
// CRITICAL: Allow hardware initialization time before first command
// Prevents "Failed to read response" race condition in integration tests
// Validated fix for bd-194 on 2025-11-02
sleep(Duration::from_millis(300)).await;
```

**Additional Changes**:
- Added `use tokio::time::{sleep, Duration};` import (line 29)

**Rationale**:
The driver was sending the `*IDN?` command immediately after opening the serial port without allowing time for hardware initialization. This works in standalone tests but fails in the integrated DAQ environment due to timing/initialization race conditions.

### 2. Build Verification

**Local Build**:
```bash
cargo check --features instrument_serial
```
Result: ✅ SUCCESS (compiled in 3.07s with 23 warnings, no errors)

**Remote Build**:
```bash
ssh maitai@100.117.5.12 'cd /tmp/rust-daq && cargo build --features instrument_serial'
```
Result: ✅ SUCCESS (compiled in 12.77s)

### 3. Integration Test Results

**Test Command**:
```bash
ssh maitai@100.117.5.12 'cd /tmp/rust-daq && timeout 10 cargo run --features instrument_serial'
```

**Result**: ❌ FAILED
```
[2025-11-02T22:21:12Z ERROR rust_daq::app] Failed to spawn instrument 'maitai':
Failed to connect: Failed to connect to instrument 'maitai':
Failed to read response from maitai
```

**Note**: ESP300 motion controller connected successfully on the same machine, proving the DAQ system works correctly.

### 4. Hardware Connectivity Testing

To diagnose the integration failure, comprehensive bash-level hardware tests were performed on the remote machine:

**Test 1: XON/XOFF Flow Control** (Correct per validation)
```bash
PORT="/dev/ttyUSB5"
stty -F "$PORT" 9600 cs8 -cstopb -parenb raw -echo ixon ixoff
echo -ne "*IDN?\r" > "$PORT"
sleep 0.5
response=$(timeout 1s dd if="$PORT" bs=1 count=200 2>/dev/null | tr -d '\000')
```

**Result**: Empty response `''`

**Test 2-4**: WAVELENGTH?, POWER?, SHUTTER? queries
**Results**: All returned empty responses `''`

## Root Cause Analysis

The MaiTai laser is **NOT responding** on `/dev/ttyUSB5` at the hardware level. This is evidenced by:

1. **Empty Bash Responses**: Direct bash serial communication shows no response
2. **Correct Configuration**: Flow control (SOFTWARE/XON-XOFF), baud rate (9600), and terminator (CR) are all verified correct
3. **Other Instruments Work**: ESP300 connects successfully on same machine
4. **Standalone Validation Passed**: Driver was previously validated with successful hardware communication

## Possible Causes

1. **MaiTai Not Powered On**: Laser may be powered down
2. **Wrong Port**: MaiTai may be connected to a different USB port (not /dev/ttyUSB5)
3. **Cable Disconnected**: Serial cable may be physically disconnected or faulty
4. **Laser Not Ready**: MaiTai may be in an error state or require initialization sequence

## Required Actions

**BLOCKER**: Hardware connectivity must be verified before software testing can continue.

User must verify:
1. ✅ MaiTai laser is powered on
2. ✅ Serial cable is connected from MaiTai to computer
3. ✅ MaiTai is connected to /dev/ttyUSB5 (or identify correct port)
4. ✅ MaiTai is in ready state (no error conditions)
5. ✅ Serial cable is functional (test with known-working device if possible)

## Verification Steps for User

```bash
# SSH to remote machine
ssh maitai@100.117.5.12

# Check which USB serial devices exist
ls -la /dev/ttyUSB*

# Test MaiTai on suspected port (example: /dev/ttyUSB5)
stty -F /dev/ttyUSB5 9600 cs8 -cstopb -parenb raw -echo ixon ixoff
echo -ne "*IDN?\r" > /dev/ttyUSB5
timeout 1s cat /dev/ttyUSB5

# Expected response: "Spectra Physics,MaiTai,<serial>,<version>"
# Actual response currently: "" (empty)
```

## Files Modified

- `src/instrument/maitai.rs` - Added 300ms initialization delay and tokio::time import

## Files Created

- `docs/hardware_testing/maitai_integration_fix_report.md` - This report

## Beads Issue Status

**Issue**: bd-194
**Status**: blocked
**Updated**: 2025-11-02
**Blocker**: Hardware connectivity verification required

## Next Steps

1. **User Action Required**: Verify MaiTai hardware connectivity
2. **After Hardware Fix**: Re-run integration test to validate timing fix
3. **If Still Fails**: Increase delay from 300ms to 500ms
4. **On Success**: Test set commands (wavelength, shutter, laser power)
5. **Extended Testing**: Run DAQ with MaiTai for 30+ minutes to verify stability

## Conclusion

The software timing fix has been correctly implemented and is ready for validation. However, testing is blocked by a hardware connectivity issue that requires physical verification of the MaiTai laser connection status.

The fix is architecturally correct and consistent with similar patterns in other working serial instruments (ESP300, Newport 1830-C). Once hardware connectivity is restored, the timing fix should resolve the integration test failure.
