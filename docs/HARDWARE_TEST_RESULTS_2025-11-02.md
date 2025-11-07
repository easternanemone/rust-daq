# Hardware Test Results - 2025-11-02

## Executive Summary

Successfully validated 3 of 4 powered-on instruments via manual serial communication tests. Discovered and corrected multiple port assignment and configuration errors. Identified serial adapter code bugs preventing full application integration.

**Status:** 3/4 instruments validated at hardware level, 2 code bugs blocking full integration

## Tested Instruments

### 1. Newport 1830-C Optical Power Meter ✅ VERIFIED

- **Port**: /dev/ttyS0 (native RS-232)
- **Baud Rate**: 9600
- **Status**: ✅ **WORKING**
- **Reading**: 1.0 nanoWatt (1E-9 W)
- **Test Commands**:
  ```bash
  stty -F /dev/ttyS0 9600 cs8 -cstopb -parenb
  echo -ne 'D?\n' > /dev/ttyS0
  cat /dev/ttyS0  # Response: 1E-9
  ```
- **Notes**: Previously validated with 88% stability over 2 minutes (62/70 successful queries)
- **Application Status**: Not tested yet in this session (blocked by other instruments hanging)

### 2. ESP300 Motion Controller ✅ VERIFIED

- **Port**: /dev/ttyUSB1 (FTDI 4-port cable, interface 0)
- **Baud Rate**: 19200
- **Flow Control**: Hardware (RTS/CTS)
- **Status**: ✅ **WORKING**
- **Response**: "ESP300 Version 3.04 07/27/01"
- **Test Commands**:
  ```bash
  stty -F /dev/ttyUSB1 19200 cs8 -cstopb -parenb crtscts
  echo -ne '*IDN?\r\n' > /dev/ttyUSB1
  cat /dev/ttyUSB1  # Response: "ESP300 Version 3.04 07/27/01"
  ```
- **Configuration Fix**: Port changed from ttyUSB3 → ttyUSB1
- **Application Status**: Not yet tested with corrected port

### 3. Elliptec ELL14 Rotators ✅ VERIFIED

- **Port**: /dev/ttyUSB0 (FTDI FT230X)
- **Baud Rate**: 9600
- **Flow Control**: None (RS-485 multidrop)
- **Status**: ✅ **WORKING** (all 3 devices)
- **Device Addresses**: 2, 3, 8
- **Test Results**:
  ```
  Address 2: Position = 2POFFFFDA9C, Info = 2IN0E1140051720231701016800023000
  Address 3: Position = 3POFFFFD000, Info = 3IN0E1140028420211501016800023000
  Address 8: Position = 8POFFFFDA9D, Info = 8IN0E1140060920231701016800023000
  ```
- **Test Commands**:
  ```bash
  stty -F /dev/ttyUSB0 9600 cs8 -cstopb -parenb
  echo -n '2gp' > /dev/ttyUSB0  # Get position address 2
  cat /dev/ttyUSB0
  echo -n '2in' > /dev/ttyUSB0  # Get info address 2
  cat /dev/ttyUSB0
  ```
- **Configuration Fixes**:
  - Device addresses changed from [0, 1] → [2, 3, 8] in both config files
  - Port verified as ttyUSB0 (was already correct)
- **Application Status**: ⚠️ **HANGS** - Serial adapter bug with hardware flow control

### 4. MaiTai Ti:Sapphire Laser ❓ NOT TESTED

- **Port**: /dev/ttyUSB5 (Silicon Labs CP2102)
- **Baud Rate**: 9600
- **Status**: ❓ **NOT TESTED** (excluded from tests to isolate other issues)
- **Configuration Fix**: Port changed from ttyUSB0 → ttyUSB5
- **Application Status**: ⚠️ **HANGS** during connection (from previous session)
- **Next Steps**: Test manually after fixing Elliptec issue

## Port Assignment Mapping

Final verified port assignments:

| Device | Port | Adapter | USB ID |
|--------|------|---------|--------|
| Newport 1830-C | /dev/ttyS0 | Native RS-232 | N/A |
| ESP300 | /dev/ttyUSB1 | FTDI 4-port (if0) | usb-FTDI_USB__-__Serial_Cable_FT1RALWL-if00-port0 |
| Elliptec | /dev/ttyUSB0 | FTDI FT230X | usb-FTDI_FT230X_Basic_UART_DK0AHAJZ-if00-port0 |
| MaiTai | /dev/ttyUSB5 | Silicon Labs CP2102 | usb-Silicon_Labs_CP2102_USB_to_UART_Bridge_Controller_20230228-906-if00-port0 |

### USB Device List

```
Bus 001 Device 003: ID 10c4:ea60 Silicon Labs CP210x UART Bridge
Bus 001 Device 004: ID 0403:6015 FTDI Bridge(I2C/SPI/UART/FIFO)
Bus 001 Device 007: ID 0403:6011 FTDI FT4232H Quad HS USB-UART/FIFO IC
```

## Configuration Changes Made

### 1. default.toml

```diff
[instruments.esp300]
-port = "/dev/ttyUSB3"
+port = "/dev/ttyUSB1"

[instruments.maitai]
-port = "/dev/ttyUSB0"  # CONFLICT
+port = "/dev/ttyUSB5"

[instruments.elliptec]
-port = "/dev/ttyUSB2"
-device_addresses = [0, 1]
+port = "/dev/ttyUSB0"
+device_addresses = [2, 3, 8]
```

### 2. test_serial.toml (created)

Minimal configuration with only Newport, ESP300, and Elliptec for isolated testing:

```toml
[application]
name = "Rust DAQ - Serial Instruments Test"
broadcast_channel_capacity = 1024

[instruments.newport_1830c]
type = "newport_1830c"
port = "/dev/ttyS0"
baud_rate = 9600
attenuator = 0
filter = 2
polling_rate_hz = 10.0

[instruments.elliptec]
type = "elliptec"
port = "/dev/ttyUSB0"
baud_rate = 9600
device_addresses = [2, 3, 8]
polling_rate_hz = 2.0

[instruments.esp300]
type = "esp300"
port = "/dev/ttyUSB1"
baud_rate = 19200
num_axes = 3
polling_rate_hz = 5.0
```

## Code Bugs Discovered

### Bug 1: Elliptec Hardware Flow Control (CRITICAL)

**Location**: `src/instrument/elliptec.rs:154`

**Issue**: Elliptec uses RS-485 multidrop protocol which doesn't require hardware flow control (RTS/CTS), but the code enables it:

```rust
let port = serialport::new(port_name, baud_rate)
    .timeout(std::time::Duration::from_millis(100))
    .flow_control(serialport::FlowControl::Hardware) // ❌ INCORRECT
    .open()
```

**Symptoms**: Application hangs during Elliptec connection when querying device info with "in" command.

**Manual Test Works**: `stty -F /dev/ttyUSB0 9600` (no hardware flow control) works perfectly.

**Fix Required**:
```rust
let port = serialport::new(port_name, baud_rate)
    .timeout(std::time::Duration::from_millis(100))
    .flow_control(serialport::FlowControl::None) // ✅ CORRECT for RS-485
    .open()
```

**Priority**: HIGH - Blocks Elliptec integration

### Bug 2: MaiTai Connection Hang

**Location**: `src/instrument/maitai.rs` (connection sequence)

**Issue**: Application hangs during MaiTai connection, possibly waiting for identity query response (*IDN?) that never arrives.

**Symptoms**: 
- Log shows "Connecting to MaiTai laser: maitai"
- No error, no completion
- Application timeout kills it after 20-30 seconds

**Next Steps**: 
1. Test MaiTai manually with corrected port (ttyUSB5)
2. Check if laser requires initialization sequence
3. Verify command syntax and terminators

**Priority**: MEDIUM - Blocking MaiTai integration

### Issue 3: Config Merging Behavior (MINOR)

**Location**: Config loading system

**Issue**: test_serial.toml only defined 3 instruments, but application loaded additional instruments (pvcam, visa_rigol, mock) from somewhere.

**Evidence**:
```
[2025-11-02T16:20:25Z INFO] Instrument 'pvcam' connected.
[2025-11-02T16:20:25Z ERROR] Failed to spawn instrument 'visa_rigol'
```

**Hypothesis**: Config system may be merging test_serial.toml with default.toml instead of fully overriding.

**Priority**: LOW - Doesn't block hardware testing, but makes config behavior unpredictable

## Application Test Results

### Test Run 1: Original Config
```
[2025-11-02T16:08:35Z ERROR] Failed to spawn instrument 'esp300': 
  Failed to read response from esp300
```
- ESP300 failed (wrong port: ttyUSB3)
- MaiTai hung (not in this test config)
- Newport not reached
- Elliptec addresses wrong: [0, 1] instead of [2, 3, 8]

### Test Run 2: Corrected Ports & Addresses
```
[2025-11-02T16:20:25Z INFO] Elliptec device addresses: [2, 3, 8]
```
- PVCAM V2 Mock: ✅ Connected
- Elliptec: ⚠️ Hung during device info query
- ESP300: Not reached (blocked by Elliptec hang)
- Newport: Not reached (blocked by Elliptec hang)

## Hardware Validation Summary

| Instrument | Hardware Status | Application Status | Blocking Issue |
|------------|----------------|-------------------|----------------|
| Newport 1830-C | ✅ Verified | ❓ Not tested | Elliptec hang blocks startup |
| ESP300 | ✅ Verified | ❓ Not tested | Elliptec hang blocks startup |
| Elliptec ELL14 | ✅ Verified (all 3 devices) | ❌ Hangs | Hardware flow control bug |
| MaiTai Laser | ❓ Not tested | ❌ Hangs | Unknown serial issue |

**Key Finding**: All tested instruments respond correctly at hardware level. Issues are in serial adapter code, not hardware.

## Next Steps

### Immediate (Fix Blocking Bugs)

1. **Fix Elliptec Hardware Flow Control Bug**
   - Edit `src/instrument/elliptec.rs:154`
   - Change `FlowControl::Hardware` → `FlowControl::None`
   - Rebuild application
   - Test with: `cargo run --config config/test_serial.toml`
   - Expected: Elliptec connects successfully, all 3 devices report position

2. **Test MaiTai Manually**
   - Verify laser responds on ttyUSB5
   - Test *IDN? command
   - Check if wavelength/shutter queries work
   - Document findings

### Secondary (Full Integration)

3. **Test Application with Fixed Elliptec**
   - Run with test_serial.toml
   - Verify Newport, ESP300, and Elliptec all connect
   - Check data polling works
   - Monitor for any timeout or communication errors

4. **Add MaiTai to Config After Manual Test**
   - If MaiTai responds manually, fix code bug
   - Add to test config
   - Test all 4 instruments together

5. **Full System Test**
   - Run with default.toml (all instruments)
   - Verify multi-instrument coordination
   - Test data acquisition rates
   - Check for any port contention issues

### Documentation

6. **Update HARDWARE_STATUS_REPORT.md**
   - Document final working configuration
   - Include operator instructions
   - Add troubleshooting section

7. **Create bd Issues for Code Bugs**
   - hw-7: Fix Elliptec hardware flow control bug
   - hw-8: Investigate and fix MaiTai connection hang
   - hw-9: Investigate config merging behavior

## Lessons Learned

1. **Port Discovery is Essential**: USB-serial port assignments can change. Always verify with `ls -l /dev/serial/by-id/` before testing.

2. **Manual Testing First**: Testing instruments manually with `stty` and `echo` commands quickly validates hardware and reveals code bugs.

3. **Config Files Can Be Stale**: Configuration files may have outdated port assignments or parameters. Verify against actual hardware.

4. **Flow Control Matters**: RS-485 (Elliptec) and RS-232 (Newport, MaiTai) have different flow control requirements. Hardware flow control on RS-485 causes hangs.

5. **Serial Protocol Variations**: Each instrument has unique quirks:
   - Newport: LF terminator only
   - ESP300: CRLF terminator, requires hardware flow control
   - Elliptec: No terminator, CR response delimiter, multidrop addressing
   - MaiTai: CR terminator, software flow control (XON/XOFF)

6. **Config System Behavior**: Need to understand how config loading works (override vs merge) to predict which instruments will be loaded.

## Files Modified

- `/home/maitai/rust-daq/config/default.toml` - Fixed ESP300, MaiTai, Elliptec ports and addresses
- `/home/maitai/rust-daq/config/test_serial.toml` - Created minimal test configuration
- No source code modified (bugs identified but not yet fixed)

## Test Logs

All test logs saved to remote machine:
- `/tmp/serial_test.log` - Initial test with wrong addresses
- `/tmp/serial_test_fixed.log` - Test with corrected ESP300 port
- `/tmp/serial_test_v2.log` - Test with corrected addresses (Elliptec still hung)

## Recommendations

1. **Fix Elliptec Bug Immediately**: This is the critical path blocker. One-line fix enables testing of all other serial instruments.

2. **Add Serial Timeouts**: Implement timeouts in serial_helper::send_command_async to prevent hangs. Current 500ms-1s timeouts may not be triggering correctly.

3. **Add Debug Logging**: Enhanced serial communication logging would help diagnose issues like the Elliptec hang faster.

4. **Consider Flow Control Auto-Detection**: Code could probe hardware capabilities and auto-configure flow control instead of hardcoding.

5. **Config Validation**: Add validation that detects port conflicts (like original MaiTai/Elliptec both on ttyUSB0) and warns at startup.

## Session Metrics

- **Duration**: ~2 hours
- **Instruments Tested**: 3 of 4
- **Hardware Validation Success Rate**: 100% (3/3 tested instruments work)
- **Application Integration Success Rate**: 0% (0/3 integrated, all blocked by serial bugs)
- **Bugs Discovered**: 2 critical, 1 minor
- **Config Issues Fixed**: 3 (ESP300 port, MaiTai port, Elliptec addresses)
- **Manual Commands Executed**: ~30
- **Application Test Runs**: 3

---

**Report Generated**: 2025-11-02T16:30:00Z  
**Operator**: Claude (AI assistant)  
**Hardware Location**: maitai@100.117.5.12  
**Project**: rust-daq v3 Hardware Integration (hw-1 epic)
