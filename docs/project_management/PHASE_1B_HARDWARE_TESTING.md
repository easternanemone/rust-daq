# Phase 1B: Hardware Validation Guide

**Epic:** bd-rbhv (V4 Architecture Implementation)
**Phase:** Phase 1B - Hardware Validation
**Status:** Ready for Testing
**Hardware Location:** `maitai@100.117.5.12`

## Overview

This guide provides step-by-step instructions for validating the V4 architecture vertical slice with real Newport 1830-C hardware on the remote maitai system.

## Prerequisites

1. **SSH Access** to `maitai@100.117.5.12`
2. **Newport 1830-C** power meter connected via RS-232/USB-Serial
3. **Rust toolchain** installed on maitai
4. **Code deployed** to maitai (via git or rsync)

## Hardware Setup

### 1. Identify Serial Port

```bash
ssh maitai@100.117.5.12

# List available serial ports
ls -la /dev/ttyUSB* /dev/ttyS* 2>/dev/null

# Check for USB-Serial adapters
dmesg | grep -i "usb serial\|ttyUSB\|ftdi"

# Verify permissions
groups  # Should include 'dialout' or equivalent
```

**Common ports:**
- `/dev/ttyUSB0` - USB-Serial adapter
- `/dev/ttyS0` - Native RS-232 port
- `/dev/ttyACM0` - Some USB devices

If permission denied:
```bash
sudo usermod -a -G dialout $USER
# Log out and back in
```

### 2. Verify Hardware Connection

Test basic serial communication:

```bash
# Install minicom if not available
sudo apt-get install minicom

# Configure minicom for Newport 1830-C
# Baud: 9600, Data bits: 8, Parity: None, Stop bits: 1
minicom -D /dev/ttyUSB0 -b 9600

# In minicom, type:
*IDN?<Enter>

# Should respond with instrument identification
# Ctrl-A X to exit minicom
```

## Testing Procedure

### Step 1: Build V4 Example

```bash
cd ~/rust-daq  # Or wherever you deployed the code

# Build with V4 and serial features
cargo build --example v4_newport_hardware_test --features v4,instrument_serial --release
```

### Step 2: Run Hardware Test

```bash
# Set environment variables
export NEWPORT_PORT=/dev/ttyUSB0  # Adjust to your port
export NEWPORT_BAUD=9600
export RUST_LOG=rust_daq=debug

# Run the hardware validation test
cargo run --example v4_newport_hardware_test --features v4,instrument_serial --release
```

### Expected Output

```
🔬 V4 Newport 1830-C Hardware Validation Test

Port: /dev/ttyUSB0
Baud: 9600

✓ Actor spawned with Kameo supervision

📝 Test 1: Configure Instrument
  Setting wavelength to 780 nm...
  ✓ Wavelength set
  Setting unit to Watts...
  ✓ Units set
  ✓ Configuration verified: 780 nm, Watts

📊 Test 2: Take 10 Measurements
  1. Power: 0.001523 W @ 780 nm (timestamp: 1699999999999999999)
  2. Power: 0.001521 W @ 780 nm (timestamp: 1699999999999999999)
  ...
  ✓ All measurements successful

📦 Test 3: Apache Arrow Data Format
  Schema:
    - timestamp: Timestamp(Nanosecond, None)
    - power: Float64
    - unit: Utf8
    - wavelength_nm: Float64
  Rows: 10
  Columns: 4
  ✓ Arrow conversion successful

🔄 Test 4: Runtime Configuration Change
  Changing to dBm units...
  New measurement: -28.174 dBm
  ✓ Configuration change successful

⚡ Test 5: Stress Test (100 rapid reads)
  Completed 100 reads in 10.23s (9.78 Hz)
  ✓ Stress test passed

✅ Hardware validation complete - all tests passed!

V4 vertical slice successfully validated with real hardware.
```

### Step 3: Verify Kameo Supervision

Test fault tolerance by simulating errors:

```bash
# While test is running, physically disconnect/reconnect serial cable
# Actor should recover gracefully with tracing warnings

# Check logs for:
# - "Hardware read failed, returning mock data: ..."
# - Actor automatic recovery
```

## Troubleshooting

### Error: "Failed to open serial port"

**Symptoms:**
```
Error: Failed to connect serial adapter
Caused by:
    Failed to open serial port '/dev/ttyUSB0' at 9600 baud
```

**Solutions:**
1. Verify port exists: `ls -la /dev/ttyUSB0`
2. Check permissions: `groups` (should include dialout)
3. Check if port is in use: `fuser /dev/ttyUSB0`
4. Try different port: `ls /dev/tty*`

### Error: "Serial read timeout"

**Symptoms:**
```
WARN rust_daq::actors::newport_1830c: Hardware read failed, returning mock data: Serial read timeout after 1s
```

**Solutions:**
1. Verify instrument is powered on
2. Check cable connections
3. Verify baud rate (should be 9600)
4. Test with minicom first
5. Check instrument settings (may need manual config)

### Error: "Failed to parse power value"

**Symptoms:**
```
Error: Failed to parse power value: 'ERROR 001'
```

**Solutions:**
1. Instrument may need initialization: Send `*RST` command
2. Check wavelength range (Newport 1830-C: 400-1100 nm)
3. Verify no error conditions on instrument display
4. Try manual command via minicom

## Success Criteria

✅ **Phase 1B Complete When:**

1. Hardware test runs without errors
2. All 5 test sections pass
3. Power measurements are realistic (not mock 1.5e-3)
4. Configuration changes apply to hardware
5. Arrow data format correct (4 columns with unit field)
6. Stress test achieves >5 Hz read rate
7. Actor supervision works (recovers from disconnections)

## Next Steps After Validation

Once hardware validation succeeds:

1. **Close bd-lsv6**: Hardware adapter implementation complete
2. **Document findings**: Any hardware-specific gotchas
3. **Proceed to Phase 1C:**
   - bd-ow2i: Implement Arrow data publishing
   - bd-ueja: Connect V4 to GUI visualization
   - bd-1925: Implement HDF5 storage actor
4. **Then Phase 2**: Multi-instrument migration using this as template

## Remote Access Tips

### SSH Multiplexing for Faster Connections

```bash
# In ~/.ssh/config
Host maitai
    HostName 100.117.5.12
    User maitai
    ControlMaster auto
    ControlPath ~/.ssh/cm-%r@%h:%p
    ControlPersist 10m
```

### Background Testing

```bash
# Run test in background
nohup cargo run --example v4_newport_hardware_test --features v4,instrument_serial --release > hardware_test.log 2>&1 &

# Monitor progress
tail -f hardware_test.log

# Check if still running
ps aux | grep v4_newport
```

### Copying Results Back

```bash
# From your local machine
scp maitai@100.117.5.12:~/rust-daq/hardware_test.log .
```

## Validation Checklist

- [ ] SSH access to maitai@100.117.5.12 confirmed
- [ ] Newport 1830-C powered on and connected
- [ ] Serial port identified (/dev/ttyUSB0 or similar)
- [ ] Permissions configured (dialout group)
- [ ] Basic serial communication tested (minicom)
- [ ] V4 example builds successfully
- [ ] Test 1 passed: Configure instrument
- [ ] Test 2 passed: Take measurements
- [ ] Test 3 passed: Arrow data format
- [ ] Test 4 passed: Runtime config change
- [ ] Test 5 passed: Stress test
- [ ] Supervision tested: Recovers from disconnection
- [ ] Results documented
- [ ] bd-lsv6 closed in beads tracker

---

**Document Version:** 1.0
**Last Updated:** 2025-11-16
**Contact:** maitai@100.117.5.12 (remote system)
