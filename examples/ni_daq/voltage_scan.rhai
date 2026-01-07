// NI DAQ Voltage Scan Example
//
// This script demonstrates a basic voltage sweep using the NI PCI-MIO-16XE-10:
// - Sweep DAC output (AO0) from start to end voltage
// - Read analog input (AI0) at each point
// - Display results
//
// Hardware Setup:
//   AO0 → External device (or loopback to AI0 for testing)
//   AI0 ← Signal to measure
//
// Usage:
//   cargo run --bin rust-daq-daemon -- run examples/ni_daq/voltage_scan.rhai
//
// Requires: NI DAQ with Comedi driver configured

print("╔════════════════════════════════════════════════════╗");
print("║  NI DAQ Voltage Scan Example                       ║");
print("║  NI PCI-MIO-16XE-10 / Comedi Driver                ║");
print("╚════════════════════════════════════════════════════╝");
print("");

// =============================================================================
// Scan Configuration
// =============================================================================

let start_voltage = 0.0;    // Starting DAC voltage (V)
let end_voltage = 5.0;      // Ending DAC voltage (V)
let num_points = 21;        // Number of measurement points
let settle_time = 0.05;     // Settling time after each step (seconds)
let ao_channel = 0;         // DAC output channel
let ai_channel = 0;         // ADC input channel

// Calculate step size
let step_size = (end_voltage - start_voltage) / (num_points - 1);

print("Scan Parameters:");
print(`  Start Voltage: ${start_voltage} V`);
print(`  End Voltage:   ${end_voltage} V`);
print(`  Points:        ${num_points}`);
print(`  Step Size:     ${step_size} V`);
print(`  Settle Time:   ${settle_time * 1000} ms`);
print("");

// =============================================================================
// Display Hardware Configuration
// =============================================================================

print("Hardware Configuration:");
let ao_range = ao.range(ao_channel);
let ai_range = ai.range(ai_channel);
print(`  AO Channel ${ao_channel}: ${ao_range.min} to ${ao_range.max} V`);
print(`  AI Channel ${ai_channel}: ${ai_range.min} to ${ai_range.max} V`);
print("");

// =============================================================================
// Initialize - Zero the output
// =============================================================================

print("Initializing...");
ao.write(ao_channel, 0.0);
sleep(0.1);
print("  ✓ DAC output zeroed");
print("");

// =============================================================================
// Perform Voltage Scan
// =============================================================================

print("Starting voltage scan...");
print("─────────────────────────────────────────────────────────");
print("  Point |  AO (V)  |  AI (V)  |  Diff (mV)");
print("─────────────────────────────────────────────────────────");

// Arrays to store results
let voltages_out = [];
let voltages_in = [];

for i in 0..num_points {
    // Calculate and set output voltage
    let v_out = start_voltage + (i * step_size);
    ao.write(ao_channel, v_out);

    // Wait for settling
    sleep(settle_time);

    // Read input voltage
    let v_in = ai.read(ai_channel);

    // Calculate difference (for loopback testing)
    let diff_mv = (v_out - v_in) * 1000.0;

    // Store results
    voltages_out.push(v_out);
    voltages_in.push(v_in);

    // Display progress
    let idx_str = if i < 9 { `  ${i+1}` } else { ` ${i+1}` };
    print(`  ${idx_str}    |  ${v_out}   |  ${v_in}   |  ${diff_mv}`);
}

print("─────────────────────────────────────────────────────────");

// =============================================================================
// Calculate Statistics
// =============================================================================

let sum = 0.0;
let sum_sq = 0.0;
let min_v = voltages_in[0];
let max_v = voltages_in[0];

for v in voltages_in {
    sum += v;
    sum_sq += v * v;
    if v < min_v { min_v = v; }
    if v > max_v { max_v = v; }
}

let mean = sum / num_points;
let variance = (sum_sq / num_points) - (mean * mean);
let std_dev = if variance > 0.0 { sqrt(variance) } else { 0.0 };

print("");
print("Scan Statistics:");
print(`  Mean:      ${mean} V`);
print(`  Std Dev:   ${std_dev} V`);
print(`  Min:       ${min_v} V`);
print(`  Max:       ${max_v} V`);
print(`  Range:     ${max_v - min_v} V`);

// =============================================================================
// Cleanup - Zero the output
// =============================================================================

print("");
print("Cleaning up...");
ao.write(ao_channel, 0.0);
print("  ✓ DAC output zeroed");
print("");
print("Scan complete!");
