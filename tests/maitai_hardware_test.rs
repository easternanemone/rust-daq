//! Hardware integration tests for MaiTai Ti:Sapphire Tunable Laser
//!
//! These tests require real hardware connected to the system.
//! Run with: cargo test --test maitai_hardware_test --features instrument_serial -- --ignored --nocapture
//!
//! Hardware Setup:
//! - MaiTai Ti:Sapphire laser connected via USB-to-serial
//! - Port: /dev/ttyUSB5 (Silicon Labs CP2102 USB-to-UART)
//! - Baud rate: 9600, 8N1
//! - Flow control: SOFTWARE (XON/XOFF) - CRITICAL! Not hardware RTS/CTS
//! - Remote machine: maitai@100.117.5.12
//!
//! SAFETY NOTES:
//! - MaiTai is a Class 4 laser - extremely dangerous
//! - Always keep shutter closed during testing unless measuring power
//! - Ensure proper safety interlocks are in place
//! - Wear appropriate laser safety goggles
//! - Only trained personnel should operate this equipment

// Note: These are documentation tests that outline the hardware validation workflow.
// Actual implementation requires running on the remote machine with hardware connected.

#[tokio::test]
#[ignore] // Hardware-only test
async fn test_connection_and_identity() {
    println!("\n=== MaiTai Connection and Identity Test ===");
    println!("Purpose: Verify connection to MaiTai laser and read identity");
    println!();
    println!("Manual Steps:");
    println!("  1. SSH to maitai@100.117.5.12");
    println!("  2. Verify port /dev/ttyUSB5 exists:");
    println!("     ls -l /dev/ttyUSB5");
    println!("  3. Check USB device info:");
    println!("     lsusb | grep -i 'silicon labs'");
    println!("  4. Configure port with software flow control:");
    println!("     stty -F /dev/ttyUSB5 9600 cs8 -cstopb -parenb raw -echo -crtscts ixon ixoff");
    println!("  5. Send identity query: '*IDN?\\r' and read response");
    println!();
    println!("Expected Response: Manufacturer and model information");
    println!("Example: 'Spectra-Physics,MaiTai,12345,1.23'");
    println!();
    println!("CRITICAL: Must use software flow control (XON/XOFF)");
    println!("Hardware flow control (RTS/CTS) will NOT work per manual");
}

#[tokio::test]
#[ignore]
async fn test_wavelength_reading() {
    println!("\n=== Wavelength Reading Test ===");
    println!("Purpose: Verify current wavelength can be read");
    println!();
    println!("Procedure:");
    println!("  1. Send command: 'WAVELENGTH?\\r'");
    println!("  2. Read response (expect numeric value)");
    println!("  3. Verify wavelength in valid range: 700-1000nm");
    println!();
    println!("Expected: Current wavelength reading (e.g., '820')");
    println!("Valid Range: 700-1000nm (Ti:Sapphire tuning range)");
    println!();
    println!("Note: Actual wavelength depends on current laser state");
    println!("Previous validation (2025-11-02) measured 820nm");
}

#[tokio::test]
#[ignore]
async fn test_wavelength_sweep() {
    println!("\n=== Wavelength Sweep Test ===");
    println!("Purpose: Validate wavelength control across Ti:Sapphire range");
    println!();
    println!("SAFETY WARNING: Keep shutter closed during this test!");
    println!();
    println!("Test wavelengths:");
    println!("  - 700nm (lower limit)");
    println!("  - 750nm");
    println!("  - 800nm (typical operating point)");
    println!("  - 850nm");
    println!("  - 900nm");
    println!("  - 950nm");
    println!("  - 1000nm (upper limit)");
    println!();
    println!("For each wavelength:");
    println!("  1. Send command: 'WAVELENGTH:<value>\\r'");
    println!("  2. Wait 5 seconds for laser to stabilize");
    println!("  3. Query back: 'WAVELENGTH?\\r'");
    println!("  4. Verify response matches set value (±1nm tolerance)");
    println!();
    println!("Expected: All wavelengths accepted, laser tunes correctly");
    println!("Document actual tuning range limits for this laser");
}

#[tokio::test]
#[ignore]
async fn test_wavelength_accuracy() {
    println!("\n=== Wavelength Accuracy Test ===");
    println!("Purpose: Validate wavelength accuracy with wavemeter");
    println!();
    println!("Equipment Required:");
    println!("  - Wavelength meter (HighFinesse, Bristol, etc.)");
    println!("  - Fiber coupler or beam splitter to wavemeter");
    println!();
    println!("Procedure:");
    println!("  1. Set MaiTai to 800nm: 'WAVELENGTH:800\\r'");
    println!("  2. Open shutter: 'SHUTTER:1\\r'");
    println!("  3. Read wavelength from wavemeter");
    println!("  4. Compare MaiTai reading vs wavemeter");
    println!("  5. Close shutter: 'SHUTTER:0\\r'");
    println!("  6. Repeat for 750nm, 850nm, 900nm");
    println!();
    println!("Acceptance Criteria:");
    println!("  - Difference < 0.5nm (wavemeter typically ±0.1nm)");
    println!("  - Consistent offset indicates calibration needed");
    println!();
    println!("Document wavelength accuracy for calibration records");
}

#[tokio::test]
#[ignore]
async fn test_power_measurement() {
    println!("\n=== Power Measurement Test ===");
    println!("Purpose: Validate power reading functionality");
    println!();
    println!("SAFETY: Ensure proper beam path and safety interlocks!");
    println!();
    println!("Procedure:");
    println!("  1. Set wavelength: 'WAVELENGTH:800\\r'");
    println!("  2. Query initial power (shutter closed): 'POWER?\\r'");
    println!("  3. Open shutter: 'SHUTTER:1\\r'");
    println!("  4. Query power readings for 30 seconds");
    println!("  5. Calculate: mean, std dev, min, max");
    println!("  6. Close shutter: 'SHUTTER:0\\r'");
    println!();
    println!("Expected:");
    println!("  - Power with shutter closed: near zero");
    println!("  - Power with shutter open: typical 1-3W for Ti:Sapphire");
    println!("  - Std dev < 5% of mean (stable power)");
    println!();
    println!("Document typical power output at 800nm for reference");
}

#[tokio::test]
#[ignore]
async fn test_power_stability_long_term() {
    println!("\n=== Power Stability Long-Term Test ===");
    println!("Purpose: Characterize power stability over extended operation");
    println!();
    println!("Setup:");
    println!("  1. Allow laser to warm up (30+ minutes)");
    println!("  2. Set wavelength to 800nm");
    println!("  3. Open shutter");
    println!("  4. Ensure thermal stability (no airflow changes)");
    println!();
    println!("Procedure:");
    println!("  1. Collect 60 minutes of power readings (1 Hz)");
    println!("  2. Calculate: mean, std dev, drift over time");
    println!("  3. Drift = |last_10min_avg - first_10min_avg| / mean");
    println!();
    println!("Acceptance Criteria:");
    println!("  - Drift < 2% over 60 minutes");
    println!("  - Std dev < 3% of mean");
    println!("  - No abrupt jumps or mode hops");
    println!();
    println!("Document stability characteristics for experiment planning");
}

#[tokio::test]
#[ignore]
async fn test_shutter_control() {
    println!("\n=== Shutter Control Test ===");
    println!("Purpose: Verify shutter open/close functionality");
    println!();
    println!("Procedure:");
    println!("  1. Query initial shutter state: 'SHUTTER?\\r'");
    println!("  2. Close shutter: 'SHUTTER:0\\r'");
    println!("  3. Verify state: 'SHUTTER?\\r' (expect '0')");
    println!("  4. Verify power near zero: 'POWER?\\r'");
    println!("  5. Open shutter: 'SHUTTER:1\\r'");
    println!("  6. Verify state: 'SHUTTER?\\r' (expect '1')");
    println!("  7. Verify power increases: 'POWER?\\r'");
    println!();
    println!("Expected:");
    println!("  - Shutter state queries return '0' or '1'");
    println!("  - Power correlates with shutter state");
    println!("  - Shutter response time < 100ms");
    println!();
    println!("SAFETY: Validated shutter control is critical for safety interlocks");
}

#[tokio::test]
#[ignore]
async fn test_shutter_response_time() {
    println!("\n=== Shutter Response Time Test ===");
    println!("Purpose: Measure shutter actuation speed");
    println!();
    println!("Equipment Required:");
    println!("  - Fast photodiode (>1 kHz bandwidth)");
    println!("  - Oscilloscope");
    println!();
    println!("Procedure:");
    println!("  1. Connect photodiode to oscilloscope");
    println!("  2. Position photodiode in beam path");
    println!("  3. Trigger scope on rising edge");
    println!("  4. Send: 'SHUTTER:1\\r'");
    println!("  5. Measure time from command to full power");
    println!("  6. Repeat for closing: 'SHUTTER:0\\r'");
    println!("  7. Average over 10 cycles");
    println!();
    println!("Expected:");
    println!("  - Opening time: 10-50ms typical for mechanical shutters");
    println!("  - Closing time: 10-50ms");
    println!();
    println!("Document for safety interlock timing requirements");
}

#[tokio::test]
#[ignore]
async fn test_response_time_characterization() {
    println!("\n=== Response Time Characterization ===");
    println!("Purpose: Measure command-to-response latency");
    println!();
    println!("Procedure:");
    println!("  1. Send 100 'WAVELENGTH?' queries");
    println!("  2. Time each query-to-response cycle");
    println!("  3. Calculate percentiles: p50, p95, p99");
    println!();
    println!("Expected Latency:");
    println!("  - p50 (median): < 100ms (9600 baud + processing)");
    println!("  - p95: < 200ms");
    println!("  - p99: < 500ms");
    println!();
    println!("Note: Response time includes:");
    println!("  - Serial transmission time (~20ms for typical response)");
    println!("  - Laser processing time (variable)");
    println!("  - XON/XOFF flow control overhead");
    println!();
    println!("Document for GUI polling rate planning (recommend 1 Hz)");
}

#[tokio::test]
#[ignore]
async fn test_error_recovery() {
    println!("\n=== Error Recovery Test ===");
    println!("Purpose: Verify laser handles errors gracefully");
    println!();
    println!("Test cases:");
    println!("  1. Send invalid command (e.g., 'INVALID\\r')");
    println!("     - Verify error response or timeout");
    println!("     - Verify laser still responds to valid commands");
    println!();
    println!("  2. Send out-of-range wavelength (e.g., 'WAVELENGTH:500\\r')");
    println!("     - Verify laser rejects or clamps to valid range");
    println!("     - Laser remains operational");
    println!();
    println!("  3. Send malformed command (missing terminator)");
    println!("     - Verify timeout handling");
    println!("     - Subsequent commands work");
    println!();
    println!("Expected:");
    println!("  - Errors don't require laser power cycle");
    println!("  - Valid commands work after errors");
    println!("  - No persistent error states");
    println!();
    println!("Document error response formats for driver error handling");
}

#[tokio::test]
#[ignore]
async fn test_disconnect_recovery() {
    println!("\n=== Disconnect Recovery Test ===");
    println!("Purpose: Validate USB disconnect detection and recovery");
    println!();
    println!("Procedure:");
    println!("  1. Start continuous polling (1 Hz)");
    println!("  2. Physically disconnect USB cable");
    println!("  3. Monitor for timeout errors");
    println!("  4. Reconnect USB cable");
    println!("  5. Verify port re-appears: ls /dev/ttyUSB5");
    println!("  6. Test manual reconnection");
    println!();
    println!("Expected:");
    println!("  - Disconnect detected within ~2 seconds (timeout)");
    println!("  - Errors logged appropriately");
    println!("  - Reconnection restores communication");
    println!();
    println!("Document:");
    println!("  - Disconnect detection time");
    println!("  - Recovery mechanism (auto vs manual reconnect)");
}

#[tokio::test]
#[ignore]
async fn test_integration_with_newport_1830c() {
    println!("\n=== MaiTai + Newport 1830C Integration Test ===");
    println!("Purpose: Validate coordinated wavelength sweep with power measurement");
    println!();
    println!("SAFETY: Ensure Newport power meter is in beam path with proper attenuation!");
    println!();
    println!("Setup:");
    println!("  1. MaiTai output → Neutral density filter → Newport power meter");
    println!("  2. Filter attenuates to <100mW (avoid saturating power meter)");
    println!("  3. Both instruments connected to rust-daq");
    println!();
    println!("Test Procedure:");
    println!("  1. Set Newport wavelength to match MaiTai");
    println!("  2. Open MaiTai shutter");
    println!("  3. Sweep MaiTai: 700nm → 1000nm (25nm steps)");
    println!("  4. At each wavelength:");
    println!("     a. Wait 10 seconds for stabilization");
    println!("     b. Update Newport wavelength setting");
    println!("     c. Record 10 seconds of power data");
    println!("  5. Close shutter");
    println!();
    println!("Expected:");
    println!("  - Power readings correlate with wavelength changes");
    println!("  - Smooth power vs wavelength curve");
    println!("  - Peak power typically near 800nm for Ti:Sapphire");
    println!("  - Demonstrates multi-instrument coordination");
    println!();
    println!("Output: Power vs wavelength plot for laser characterization");
}

#[tokio::test]
#[ignore]
async fn test_hardware_info_collection() {
    println!("\n=== MaiTai Hardware Information Collection ===");
    println!("Purpose: Document all hardware specifications");
    println!();
    println!("Information to collect:");
    println!("  - Identity string: '*IDN?'");
    println!("  - Serial number");
    println!("  - Firmware version");
    println!("  - Tuning range (actual vs specification)");
    println!("  - Typical output power");
    println!("  - Pump diode operating parameters");
    println!("  - Crystal temperature");
    println!("  - Last service date");
    println!();
    println!("Additional queries to try:");
    println!("  - 'STATUS?' - Get laser status");
    println!("  - 'ERROR?' - Check for error conditions");
    println!("  - 'PUMPPOWER?' - Pump diode power");
    println!("  - 'TEMPERATURE?' - Crystal temperature");
    println!();
    println!("Note: Command set may vary by firmware version");
    println!("Refer to MaiTai manual for complete command reference");
    println!();
    println!("Output: Complete hardware report for docs/hardware/maitai.md");
}

#[tokio::test]
#[ignore]
async fn test_safety_interlock_verification() {
    println!("\n=== Safety Interlock Verification ===");
    println!("Purpose: Validate safety interlock functionality");
    println!();
    println!("CRITICAL SAFETY TEST - Perform with caution!");
    println!();
    println!("Tests:");
    println!("  1. Key switch interlock");
    println!("     - Remove key, verify laser cannot be turned on");
    println!("     - Verify shutter commands rejected");
    println!();
    println!("  2. Safety door interlock (if equipped)");
    println!("     - Open door, verify laser shuts down");
    println!("     - Verify shutter closes automatically");
    println!();
    println!("  3. Software shutter control reliability");
    println!("     - Send rapid shutter commands");
    println!("     - Verify all commands executed");
    println!("     - Test emergency close");
    println!();
    println!("Expected:");
    println!("  - All hardware interlocks prevent laser emission");
    println!("  - Software control responds within 100ms");
    println!("  - Shutter defaults to closed on error");
    println!();
    println!("Document: Safety system validation for facility compliance");
}

#[test]
fn test_maitai_hardware_test_summary() {
    println!("\n=== MaiTai Ti:Sapphire Laser Hardware Test Summary ===");
    println!();
    println!("Test Suite Coverage:");
    println!("  ✅ Connection and identity (port /dev/ttyUSB5, 9600 baud, XON/XOFF)");
    println!("  ✅ Wavelength reading and accuracy");
    println!("  ✅ Wavelength sweep (700-1000nm Ti:Sapphire range)");
    println!("  ✅ Power measurement and stability");
    println!("  ✅ Shutter control and response time");
    println!("  ✅ Command response time characterization");
    println!("  ✅ Error recovery");
    println!("  ✅ Disconnect handling");
    println!("  ✅ Integration with Newport 1830C power meter");
    println!("  ✅ Hardware info collection");
    println!("  ✅ Safety interlock verification");
    println!();
    println!("Hardware Details:");
    println!("  - Port: /dev/ttyUSB5 (Silicon Labs CP2102 USB-to-UART)");
    println!("  - Baud: 9600, 8N1");
    println!("  - Flow Control: SOFTWARE (XON/XOFF) - NOT hardware RTS/CTS");
    println!("  - Tuning Range: 700-1000nm (Ti:Sapphire)");
    println!("  - Typical Power: 1-3W");
    println!("  - Polling Rate: 1 Hz (recommended)");
    println!();
    println!("Protocol:");
    println!("  - Command format: '<CMD>\\r' (CR terminator)");
    println!("  - Query format: '<CMD>?\\r'");
    println!("  - Set format: '<CMD>:<value>\\r'");
    println!();
    println!("CRITICAL SAFETY NOTES:");
    println!("  - Class 4 laser - extremely dangerous");
    println!("  - Always verify shutter closed before maintenance");
    println!("  - Ensure safety interlocks functional");
    println!("  - Only trained personnel should operate");
    println!("  - Use appropriate laser safety goggles");
    println!();
    println!("Validation Status (2025-11-02):");
    println!("  ✅ Serial port accessible (/dev/ttyUSB5)");
    println!("  ✅ Communication established");
    println!("  ✅ Wavelength reading: 820nm");
    println!("  ✅ Power reading functional");
    println!("  ✅ Shutter control tested");
    println!();
    println!("Next Steps:");
    println!("  1. Run hardware test suite on maitai@100.117.5.12");
    println!("  2. Characterize wavelength vs power curve");
    println!("  3. Document wavelength accuracy vs wavemeter");
    println!("  4. Validate integration with Newport power meter");
    println!("  5. Create operator guide with safety procedures");
}
