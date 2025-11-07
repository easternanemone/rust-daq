# Hardware Finalization Plan

**Date**: 2025-11-01  
**Remote Hardware Location**: maitai@100.117.5.12  
**Epic**: hw-1 - Real Hardware Integration and End-to-End Testing

## Executive Summary

This document outlines the plan to finalize hardware integration for 5 real instruments. Based on code analysis:

- **PVCAM Camera** requires **significant code completion** (9 TODOs, currently uses simulated data)
- **Other 4 instruments** have **complete implementations** and require **hardware testing only**

## Priority Order

### Priority 1: PVCAM Camera (hw-2/bd-94) - CODE COMPLETION REQUIRED
**Estimated Effort**: 2-3 days  
**Status**: 9 TODOs, uses `simulate_frame_data()`  
**File**: `src/instrument/pvcam.rs`

This is the most complex task requiring actual code implementation before hardware testing.

### Priority 2-5: Hardware Testing (1-2 days each)
These instruments have complete code implementations and require validation with actual hardware:

2. **Newport 1830c Power Meter** (hw-4) - Simplest serial instrument
3. **MaiTai Laser** (hw-3) - Serial instrument with safety considerations
4. **Elliptec Rotators** (hw-5) - Multi-device RS-485 bus
5. **ESP300 Motion Controller** (hw-6) - 3-axis motion system

---

## Task 1: PVCAM Camera Integration (hw-2/bd-94)

### Current State
- V1 implementation at `src/instrument/pvcam.rs` 
- Broadcasts frame statistics (mean, min, max) as scalar DataPoints
- **Uses simulated data** via `simulate_frame_data()` (lines 60-75)
- **9 TODOs** for SDK integration

### TODOs to Complete

#### 1. SDK Initialization (connect method, lines 125-128)
```rust
// TODO: Initialize PVCAM SDK
// pl_pvcam_init()
// pl_cam_open()
// Configure ROI, binning, exposure time, etc.
```

**Actions**:
- [ ] Call `pl_pvcam_init()` to initialize SDK
- [ ] Enumerate cameras and verify "PrimeBSI" is detected
- [ ] Call `pl_cam_open()` with camera name
- [ ] Configure ROI from config (default: [0, 0, 2048, 2048])
- [ ] Configure binning from config (default: [1, 1])
- [ ] Set initial exposure time (default: 100ms)
- [ ] Validate all parameters with `pl_get_param()`
- [ ] Error handling for camera not found, SDK init failure

#### 2. Frame Acquisition Loop (acquisition task, lines 155-158)
```rust
// TODO: Acquire actual frame from PVCAM
// pl_exp_start_seq()
// pl_exp_check_status()
// pl_exp_get_latest_frame()
```

**Actions**:
- [ ] Replace `simulate_frame_data()` with real SDK calls
- [ ] Start sequence acquisition with `pl_exp_start_seq()`
- [ ] Poll for frame ready with `pl_exp_check_status()`
- [ ] Retrieve frame buffer with `pl_exp_get_latest_frame()`
- [ ] Implement frame buffer rotation for zero-copy
- [ ] Handle timeouts and acquisition errors
- [ ] Use `spawn_blocking` for blocking SDK calls
- [ ] Maintain frame counter and statistics calculation

#### 3. SDK Cleanup (disconnect method, lines 215-217)
```rust
// TODO: Cleanup PVCAM SDK
// pl_cam_close()
// pl_pvcam_uninit()
```

**Actions**:
- [ ] Stop any ongoing acquisition
- [ ] Call `pl_cam_close()` for camera handle
- [ ] Call `pl_pvcam_uninit()` to cleanup SDK
- [ ] Handle cleanup errors gracefully
- [ ] Ensure called even if errors occurred earlier

#### 4. Parameter Control - Exposure (line 234)
```rust
// TODO: Apply to camera hardware
```

**Actions**:
- [ ] Use `pl_set_param()` to set PARAM_EXPOSURE_TIME
- [ ] Convert milliseconds to SDK units (usually microseconds)
- [ ] Validate exposure range with `pl_get_param()` bounds
- [ ] Update hardware before storing in `self.exposure_ms`
- [ ] Handle errors (out of range, camera busy)

#### 5. Parameter Control - Gain (line 237)
```rust
// TODO: Set camera gain
```

**Actions**:
- [ ] Determine PVCAM gain parameter (PARAM_GAIN_INDEX or PARAM_GAIN_MULT_FACTOR)
- [ ] Validate gain value against camera capabilities
- [ ] Set via `pl_set_param()`
- [ ] Store current gain in struct for state tracking

#### 6. Parameter Control - Binning (line 241)
```rust
// TODO: Set camera binning
```

**Actions**:
- [ ] Parse binning value as array [x_bin, y_bin]
- [ ] Validate binning modes supported by camera
- [ ] Set PARAM_BINNING_SER and PARAM_BINNING_PAR
- [ ] May require stopping/restarting acquisition
- [ ] Update ROI calculations if binning changes

#### 7. Execute Command - Start Acquisition (line 253)
```rust
// TODO: Start continuous acquisition
```

**Actions**:
- [ ] Start continuous sequence mode with `pl_exp_start_seq()`
- [ ] Set acquisition mode to STROBED or TIMED
- [ ] Configure circular buffer for continuous frames
- [ ] Handle already-acquiring state

#### 8. Execute Command - Stop Acquisition (line 257)
```rust
// TODO: Stop acquisition
```

**Actions**:
- [ ] Check acquisition status with `pl_exp_check_status()`
- [ ] Abort acquisition if running with `pl_exp_abort()`
- [ ] Clean up any pending buffers
- [ ] Handle not-acquiring state gracefully

#### 9. Execute Command - Snap Frame (line 261)
```rust
// TODO: Acquire single frame
```

**Actions**:
- [ ] Stop continuous mode if running
- [ ] Start single-shot acquisition
- [ ] Wait for frame completion
- [ ] Return to continuous mode if it was active before
- [ ] Broadcast single frame statistics

### SDK Integration Requirements

**Dependencies**:
- PVCAM SDK must be installed on maitai@100.117.5.12
- Rust bindings may need to be created (check for existing crates)
- Consider using `pvcam-sys` or similar FFI crate

**Build System**:
- Add PVCAM SDK linking in `Cargo.toml` or `build.rs`
- May need `#[cfg(feature = "pvcam_hardware")]` feature flag
- Document SDK version requirements

### Testing Plan

**Phase 1: SDK Connectivity**
- [ ] SSH to maitai@100.117.5.12
- [ ] Verify camera is detected by system
- [ ] Test `pl_pvcam_init()` succeeds
- [ ] Enumerate cameras and verify "PrimeBSI" present
- [ ] Test `pl_cam_open()` with camera name

**Phase 2: Basic Acquisition**
- [ ] Configure minimum parameters (exposure, ROI, binning)
- [ ] Start single-shot acquisition
- [ ] Verify frame data received
- [ ] Check frame integrity (size, checksum if available)
- [ ] Test dark field (shutter closed) for noise characterization

**Phase 3: Continuous Acquisition**
- [ ] Start continuous streaming at 1 Hz
- [ ] Verify stable frame rate over 1 minute
- [ ] Monitor for frame drops or corruption
- [ ] Test with GUI running to verify data flow

**Phase 4: Parameter Control**
- [ ] Test exposure time changes while acquiring
- [ ] Test gain control
- [ ] Test binning changes (may require restart)
- [ ] Verify GUI reflects hardware state

**Phase 5: Stress Testing**
- [ ] 10+ minute continuous acquisition
- [ ] Rapid parameter changes
- [ ] Start/stop cycling
- [ ] Error injection (disconnect, invalid parameters)

**Phase 6: Documentation**
- [ ] Document correct TOML configuration
- [ ] Document SDK build requirements
- [ ] Create operator guide (startup, calibration, troubleshooting)
- [ ] Document known issues and workarounds

### Acceptance Criteria
- [x] Code analysis complete
- [ ] All 9 TODOs implemented
- [ ] Camera connects and acquires real frames
- [ ] Frame statistics display in GUI with <100ms latency
- [ ] All parameter controls work (exposure, gain, binning)
- [ ] System runs stably for 10+ minutes
- [ ] Operator documentation complete

---

## Task 2: Newport 1830c Power Meter (hw-4)

### Current State
- ✅ **Implementation complete** at `src/instrument/newport_1830c.rs`
- ✅ 0 TODOs, 0 simulation code
- ✅ Full serial communication implementation
- ✅ Parameter control (attenuator, filter)
- ✅ Retry logic and error handling

### Hardware Testing Plan

**Phase 1: Serial Connectivity**
- [ ] SSH to maitai@100.117.5.12
- [ ] Identify serial port: `ls /dev/tty* | grep USB`
- [ ] Test manual communication: `screen /dev/ttyUSB0 9600`
- [ ] Send test command: `D?` (query power)
- [ ] Verify response format (scientific notation)

**Phase 2: Integration Testing**
- [ ] Update `config/default.toml` with correct port
- [ ] Build and run: `cargo run --features instrument_serial`
- [ ] Verify instrument connects
- [ ] Check power readings display in GUI

**Phase 3: Calibration Testing**
- [ ] Dark measurement (detector covered): record baseline noise
- [ ] Test with known light source (MaiTai laser if available)
- [ ] Validate measurement accuracy
- [ ] Test wavelength-dependent correction if needed

**Phase 4: Parameter Testing**
- [ ] Test attenuator on/off (commands: A0, A1)
- [ ] Test filter settings (commands: F1, F2, F3 for Slow/Medium/Fast)
- [ ] Verify readings change as expected
- [ ] Test zero/clear status command (CS)

**Phase 5: Stability Testing**
- [ ] Continuous measurement for 30+ minutes
- [ ] Monitor for drift
- [ ] Check for communication timeouts
- [ ] Verify graceful handling of disconnects

**Phase 6: Documentation**
- [ ] Verify TOML configuration in docs
- [ ] Create operator guide (zeroing procedure, calibration)
- [ ] Document troubleshooting (drift, noise, communication)

### Acceptance Criteria
- [ ] Power meter connects via serial
- [ ] Real-time measurements display in GUI
- [ ] All parameters work (attenuator, filter)
- [ ] Measurements within spec accuracy (<5%)
- [ ] 30+ minute stability verified
- [ ] Operator documentation complete

---

## Task 3: MaiTai Laser (hw-3)

### Current State
- ✅ **Implementation complete** at `src/instrument/maitai.rs`
- ✅ 0 TODOs, 0 simulation code
- ✅ Serial communication for wavelength and shutter
- ✅ Error handling and retry logic

### Hardware Testing Plan

**Phase 1: Serial Connectivity**
- [ ] SSH to maitai@100.117.5.12
- [ ] Identify serial port (likely /dev/ttyUSB1 or similar)
- [ ] Test manual communication with laser
- [ ] Query wavelength and shutter status
- [ ] Document command format

**Phase 2: Safety Testing**
- [ ] Verify shutter closes on disconnect
- [ ] Test emergency stop behavior
- [ ] Confirm beam block indicators
- [ ] Document laser safety interlock status

**Phase 3: Integration Testing**
- [ ] Update `config/default.toml` with correct port
- [ ] Build and run application
- [ ] Verify laser connects and reports wavelength
- [ ] Test shutter control (open/close)

**Phase 4: Tuning Testing**
- [ ] Test wavelength tuning across range (700-1000nm typical)
- [ ] Measure tuning speed and settling time
- [ ] Verify wavelength accuracy (use Newport power meter + filters)
- [ ] Test limits (min/max wavelength)

**Phase 5: Coordinated Measurements**
- [ ] Test with Newport power meter as light detector
- [ ] Wavelength sweep with power monitoring
- [ ] Verify coordinated data acquisition
- [ ] Test trigger/sync if supported

**Phase 6: Stability Testing**
- [ ] Continuous operation for 30+ minutes
- [ ] Monitor wavelength drift
- [ ] Check for communication timeouts
- [ ] Document warmup behavior

**Phase 7: Documentation**
- [ ] Verify TOML configuration
- [ ] Create operator guide (startup, tuning, shutdown)
- [ ] Document laser safety procedures
- [ ] Add troubleshooting section

### Acceptance Criteria
- [ ] Laser connects via serial
- [ ] Wavelength tuning works across full range
- [ ] Shutter control operates reliably
- [ ] Live wavelength displays in GUI
- [ ] Safety interlocks tested and documented
- [ ] Coordinated with power meter validated
- [ ] Operator documentation complete

---

## Task 4: Elliptec Rotators (hw-5)

### Current State
- ✅ **Implementation complete** at `src/instrument/elliptec.rs`
- ✅ 0 TODOs, 0 simulation code
- ✅ Multi-device RS-485 bus support
- ✅ Address-based command routing

### Hardware Testing Plan

**Phase 1: Bus Connectivity**
- [ ] SSH to maitai@100.117.5.12
- [ ] Identify serial port for Elliptec bus distributor
- [ ] Verify baud rate (9600 8N1 per config)
- [ ] Document bus topology (3 rotators, which addresses?)
- [ ] Test individual device queries

**Phase 2: Device Discovery**
- [ ] Query all device addresses (config shows [0, 1] - verify if 3rd exists)
- [ ] Test address-prefixed command protocol
- [ ] Verify no crosstalk between devices
- [ ] Handle missing/offline rotators gracefully

**Phase 3: Integration Testing**
- [ ] Update `config/default.toml` with all 3 device addresses
- [ ] Build and run application
- [ ] Verify all 3 rotators connect
- [ ] Check position displays for each device

**Phase 4: Position Control**
- [ ] Test absolute positioning (0-360 degrees)
- [ ] Test relative moves
- [ ] Measure position accuracy and repeatability
- [ ] Test simultaneous multi-rotator movements

**Phase 5: Homing & Calibration**
- [ ] Implement homing sequence for each rotator
- [ ] Test index position detection
- [ ] Document mechanical zero vs software zero
- [ ] Create optical alignment calibration procedure

**Phase 6: Endurance Testing**
- [ ] Continuous rotation test (100+ rotations per device)
- [ ] Multi-hour stability test
- [ ] Test for position drift
- [ ] Monitor for communication errors

**Phase 7: Coordinated Motion**
- [ ] Test coordinated rotation sequences
- [ ] Verify position synchronization
- [ ] Test triggered rotations (e.g., on MaiTai wavelength change)
- [ ] Document polarization control use cases

**Phase 8: Documentation**
- [ ] Document TOML configuration for 3-device bus
- [ ] Create operator guide (homing, positioning, calibration)
- [ ] Add troubleshooting section (address conflicts, timeouts)
- [ ] Document mechanical limits

### Acceptance Criteria
- [ ] All 3 rotators communicate on shared bus
- [ ] Position control works independently
- [ ] Live positions display in GUI for all devices
- [ ] Homing sequence >95% success rate
- [ ] Position accuracy <1 degree
- [ ] Coordinated movements tested
- [ ] Operator documentation complete

---

## Task 5: ESP300 Motion Controller (hw-6)

### Current State
- ✅ **Implementation complete** at `src/instrument/esp300.rs`
- ✅ 0 TODOs, 0 simulation code
- ✅ 3-axis motion control
- ✅ Hardware flow control support

### Hardware Testing Plan

**Phase 1: Serial Connectivity**
- [ ] SSH to maitai@100.117.5.12
- [ ] Identify serial port
- [ ] Verify baud rate (19200) and hardware flow control (per config)
- [ ] Test basic serial communication
- [ ] Query controller firmware version

**Phase 2: Axis Configuration**
- [ ] Query number of active axes
- [ ] Verify axis configuration matches config
- [ ] Test individual axis queries
- [ ] Document axis naming/numbering

**Phase 3: Integration Testing**
- [ ] Update `config/default.toml` with correct port and axis count
- [ ] Build and run application
- [ ] Verify all axes connect and report positions
- [ ] Check position displays in GUI (5Hz per config)

**Phase 4: Motion Control**
- [ ] Test absolute positioning for each axis
- [ ] Test relative moves
- [ ] Verify motion completion detection
- [ ] Test velocity and acceleration settings
- [ ] Test emergency stop

**Phase 5: Homing & Limits**
- [ ] Implement homing sequence for each axis
- [ ] Test limit switch detection
- [ ] Verify home position repeatability
- [ ] Test soft limits vs hard limits
- [ ] Document safe operating ranges

**Phase 6: Multi-Axis Testing**
- [ ] Test simultaneous multi-axis moves
- [ ] Verify axis independence (no crosstalk)
- [ ] Test coordinated moves (diagonal, circular paths)
- [ ] Test motion queue if supported

**Phase 7: Performance Validation**
- [ ] Position accuracy testing (commanded vs measured)
- [ ] Repeatability testing (10+ moves to same position)
- [ ] Settling time characterization
- [ ] Backlash measurement
- [ ] Long-term drift testing (1+ hour stationary)

**Phase 8: Safety Testing**
- [ ] Test motor enable/disable
- [ ] Verify error state handling (stall, following error)
- [ ] Test recovery from error states
- [ ] Test power loss recovery
- [ ] Document emergency procedures

**Phase 9: Documentation**
- [ ] Document TOML configuration for all axes
- [ ] Create operator guide (homing, positioning, safety)
- [ ] Add troubleshooting section (communication, motion faults)
- [ ] Document mechanical setup
- [ ] Create coordinate system documentation

### Acceptance Criteria
- [ ] All axes communicate and respond
- [ ] Position control works for all axes
- [ ] Live positions display in GUI
- [ ] Homing succeeds consistently
- [ ] Position accuracy <10μm (typical)
- [ ] Multi-axis coordination tested
- [ ] Safety features validated
- [ ] Operator documentation complete

---

## Overall Timeline

### Week 1: PVCAM Camera (Priority 1)
- **Days 1-2**: SDK integration (TODOs 1-6)
- **Day 3**: Command implementation (TODOs 7-9)
- **Days 4-5**: Hardware testing and debugging

### Week 2: Serial Instruments (Priority 2-3)
- **Days 1-2**: Newport 1830c Power Meter
- **Days 3-4**: MaiTai Laser
- **Day 5**: Buffer/catch-up

### Week 3: Motion Systems (Priority 4-5)
- **Days 1-3**: Elliptec Rotators (3 devices)
- **Days 4-5**: ESP300 Motion Controller

### Week 4: Integration & Documentation
- **Days 1-2**: Multi-instrument coordination testing
- **Days 3-4**: Documentation completion
- **Day 5**: Final validation and sign-off

**Total Estimated Duration**: 4 weeks

---

## Dependencies & Prerequisites

### Remote Access
- [ ] Confirm SSH access to maitai@100.117.5.12
- [ ] Verify all hardware is powered on and connected
- [ ] Test remote desktop/GUI access if needed for visualization

### Build Environment
- [ ] PVCAM SDK installed on remote machine
- [ ] Rust toolchain on remote machine (or cross-compile)
- [ ] Feature flags configured correctly
- [ ] Test build on remote system

### Hardware Verification
- [ ] Camera: Photometrics Prime BSI sCMOS detected by system
- [ ] Newport 1830c: Connected via RS-232, photodetector functional
- [ ] MaiTai: Laser powered on, serial connection available
- [ ] Elliptec: All 3 rotators on bus, mechanical freedom of motion
- [ ] ESP300: Controller powered, all axes enabled, limit switches functional

### Safety
- [ ] Laser safety training completed
- [ ] Shutter/interlock procedures documented
- [ ] Emergency stop locations identified
- [ ] Eye protection available if needed

---

## Risk Assessment

### High Risk
- **PVCAM SDK Integration**: Most complex task, potential for unexpected SDK issues
  - *Mitigation*: Allocate extra time, have PVCAM support contact ready
  
### Medium Risk
- **Multi-Device Bus (Elliptec)**: RS-485 bus timing and address conflicts
  - *Mitigation*: Test devices individually first, document bus behavior
  
- **Motion Controller Safety**: ESP300 could damage hardware if misconfigured
  - *Mitigation*: Start with small moves, verify limits, document safe ranges

### Low Risk
- **Newport 1830c**: Simple serial instrument, well-documented protocol
- **MaiTai Laser**: Serial communication is straightforward

---

## Success Metrics

### Code Quality
- [ ] All TODOs resolved (PVCAM only)
- [ ] No compiler warnings
- [ ] Clean RML analysis (`~/.rml/rml/rml` passes)
- [ ] Code follows existing patterns

### Hardware Performance
- [ ] All instruments connect reliably (>95% success rate)
- [ ] Data latency <100ms for all instruments
- [ ] Stable operation for 10+ minutes per instrument
- [ ] Parameter controls work as expected

### Documentation
- [ ] Operator guides for all 5 instruments
- [ ] TOML configuration examples verified
- [ ] Troubleshooting sections complete
- [ ] Safety procedures documented

### User Experience
- [ ] Operator can use instruments without developer assistance
- [ ] GUI displays all instrument data correctly
- [ ] Error messages are actionable
- [ ] Recovery procedures are clear

---

## Post-Finalization

### Code Quality
- [ ] Run `~/.rml/rml/rml` on all changed files
- [ ] Run `cargo fmt` and `cargo clippy`
- [ ] Run `cargo test` to verify no regressions

### Git Workflow
- [ ] Commit changes per instrument with descriptive messages
- [ ] Update bd issues to "closed" status
- [ ] Push to remote repository

### Documentation
- [ ] Update main CLAUDE.md with hardware status
- [ ] Add operator guides to docs/operators/
- [ ] Update README with hardware capabilities
- [ ] Create release notes if applicable

---

## Contact & Support

### Hardware Access
- **Remote Machine**: maitai@100.117.5.12
- **SSH Command**: `ssh maitai@100.117.5.12`

### SDK Support
- **PVCAM SDK**: Contact Teledyne Photometrics support if integration issues arise
- **Newport**: Check Newport 1830-C manual for command reference

### Code Review
- **RML Analysis**: `~/.rml/rml/rml` for AI-powered bug detection
- **Beads Issues**: `bd list --status open` to track progress

---

## Appendix: Quick Reference

### Common Commands

```bash
# Remote access
ssh maitai@100.117.5.12

# Build with hardware support
cargo build --features instrument_serial

# Run with full features
cargo run --features full

# Code analysis
~/.rml/rml/rml src/instrument/pvcam.rs

# Check bd issues
export BEADS_DB=.beads/daq.db
bd list --status open | grep hw-
```

### File Locations

- **PVCAM**: `src/instrument/pvcam.rs`
- **Newport**: `src/instrument/newport_1830c.rs`
- **MaiTai**: `src/instrument/maitai.rs`
- **Elliptec**: `src/instrument/elliptec.rs`
- **ESP300**: `src/instrument/esp300.rs`
- **Config**: `config/default.toml`

### BD Issue IDs

- hw-1: Epic (parent of all hardware tasks)
- hw-2/bd-94: PVCAM Camera (duplicate issues)
- hw-3: MaiTai Laser
- hw-4: Newport 1830c
- hw-5: Elliptec Rotators  
- hw-6: ESP300 Motion Controller
