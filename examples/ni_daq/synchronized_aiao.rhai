// NI DAQ Synchronized Analog I/O Example
//
// This script demonstrates coordinated analog input/output operations:
// - Generate output waveform on AO channels
// - Synchronously sample AI channels
// - Phase-locked stimulus-response measurement
// - Multi-channel correlation
//
// Hardware Setup:
//   AO0 → Stimulus output (or loopback to AI0)
//   AO1 → Reference output (optional)
//   AI0 ← Response measurement
//   AI1 ← Reference measurement
//
// Usage:
//   cargo run --bin rust-daq-daemon -- run examples/ni_daq/synchronized_aiao.rhai
//
// Requires: NI DAQ with Comedi driver configured

print("╔════════════════════════════════════════════════════╗");
print("║  NI DAQ Synchronized AI/AO Example                 ║");
print("║  NI PCI-MIO-16XE-10 / Comedi Driver                ║");
print("╚════════════════════════════════════════════════════╝");
print("");

// =============================================================================
// Configuration
// =============================================================================

let ao_stimulus = 0;     // AO channel for stimulus
let ao_reference = 1;    // AO channel for reference
let ai_response = 0;     // AI channel for response
let ai_reference = 1;    // AI channel for reference

let sample_rate = 100.0;   // Samples per second
let waveform_freq = 1.0;   // Waveform frequency (Hz)
let num_cycles = 3;        // Number of complete cycles
let amplitude = 2.0;       // Waveform amplitude (V)
let offset = 2.5;          // DC offset (V)

let samples_per_cycle = (sample_rate / waveform_freq).to_int();
let total_samples = samples_per_cycle * num_cycles;
let sample_period = 1.0 / sample_rate;

print("Synchronization Parameters:");
print(`  Sample Rate:      ${sample_rate} Hz`);
print(`  Waveform Freq:    ${waveform_freq} Hz`);
print(`  Cycles:           ${num_cycles}`);
print(`  Samples/Cycle:    ${samples_per_cycle}`);
print(`  Total Samples:    ${total_samples}`);
print(`  Amplitude:        ${amplitude} V`);
print(`  DC Offset:        ${offset} V`);
print("");

// =============================================================================
// Display Hardware Configuration
// =============================================================================

print("Hardware Configuration:");
let ao0_range = ao.range(ao_stimulus);
let ao1_range = ao.range(ao_reference);
let ai0_range = ai.range(ai_response);
let ai1_range = ai.range(ai_reference);
print(`  AO${ao_stimulus} (stimulus):  ${ao0_range.min} to ${ao0_range.max} V`);
print(`  AO${ao_reference} (reference): ${ao1_range.min} to ${ao1_range.max} V`);
print(`  AI${ai_response} (response):  ${ai0_range.min} to ${ai0_range.max} V`);
print(`  AI${ai_reference} (reference): ${ai1_range.min} to ${ai1_range.max} V`);
print("");

// =============================================================================
// Initialize Outputs
// =============================================================================

print("Initializing outputs...");
ao.write(ao_stimulus, offset);
ao.write(ao_reference, offset);
sleep(0.1);
print("  ✓ Outputs set to DC offset");
print("");

// =============================================================================
// Synchronized Acquisition: Sine Wave Stimulus
// =============================================================================

print("Demo 1: Sine Wave Stimulus-Response");
print("─────────────────────────────────────────────────────────");

let PI = 3.14159265359;
let stimulus_data = [];
let response_data = [];
let time_data = [];

print("  Generating synchronized stimulus and sampling response...");
print("");
print("  Sample |  Time (s)  |  AO (V)   |  AI (V)   |  Diff (mV)");
print("─────────────────────────────────────────────────────────");

for i in 0..total_samples {
    let t = i * sample_period;
    let phase = 2.0 * PI * waveform_freq * t;

    // Generate sine wave stimulus
    let v_out = offset + amplitude * sin(phase);
    ao.write(ao_stimulus, v_out);

    // Small settling time
    sleep(sample_period * 0.5);

    // Sample response
    let v_in = ai.read(ai_response);

    // Store data
    time_data.push(t);
    stimulus_data.push(v_out);
    response_data.push(v_in);

    // Display every 10th sample
    if i % 10 == 0 {
        let diff_mv = (v_out - v_in) * 1000.0;
        print(`   ${i:4}  |   ${t:6.3}   | ${v_out:8.4}  | ${v_in:8.4}  | ${diff_mv:+8.2}`);
    }

    // Maintain sample timing
    sleep(sample_period * 0.5);
}

print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Calculate Transfer Characteristics
// =============================================================================

print("Transfer Analysis:");
print("─────────────────────────────────────────────────────────");

// Calculate correlation coefficient
let sum_xy = 0.0;
let sum_x = 0.0;
let sum_y = 0.0;
let sum_x2 = 0.0;
let sum_y2 = 0.0;
let n = stimulus_data.len();

for i in 0..n {
    let x = stimulus_data[i];
    let y = response_data[i];
    sum_xy += x * y;
    sum_x += x;
    sum_y += y;
    sum_x2 += x * x;
    sum_y2 += y * y;
}

let numerator = (n * sum_xy) - (sum_x * sum_y);
let denominator_sq = ((n * sum_x2) - (sum_x * sum_x)) * ((n * sum_y2) - (sum_y * sum_y));
let denominator = if denominator_sq > 0.0 { sqrt(denominator_sq) } else { 1.0 };
let correlation = numerator / denominator;

// Calculate linear regression (gain)
let mean_x = sum_x / n;
let mean_y = sum_y / n;
let gain = numerator / ((n * sum_x2) - (sum_x * sum_x));
let intercept = mean_y - gain * mean_x;

// Calculate RMS error
let sum_err_sq = 0.0;
for i in 0..n {
    let predicted = gain * stimulus_data[i] + intercept;
    let error = response_data[i] - predicted;
    sum_err_sq += error * error;
}
let rms_error = sqrt(sum_err_sq / n);

print(`  Correlation:     ${correlation:.6}`);
print(`  Gain:            ${gain:.6}`);
print(`  Offset:          ${intercept:.6} V`);
print(`  RMS Error:       ${rms_error * 1000.0:.3} mV`);
print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Demo 2: Step Response
// =============================================================================

print("Demo 2: Step Response Measurement");
print("─────────────────────────────────────────────────────────");

let step_levels = [0.0, 2.5, 5.0, 2.5, 0.0];
let settle_samples = 20;

for level in step_levels {
    print(`  Step to ${level} V:`);
    ao.write(ao_stimulus, level);

    // Sample during settling
    for j in 0..settle_samples {
        sleep(sample_period);
        let v = ai.read(ai_response);
        let error = level - v;

        // Show progress
        if j < 5 || j >= settle_samples - 2 {
            print(`    t=${j * sample_period * 1000.0:5.1} ms: ${v:7.4} V (error: ${error * 1000.0:+6.2} mV)`);
        } else if j == 5 {
            print(`    ...`);
        }
    }
    print("");
}

print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Demo 3: Dual Channel Comparison
// =============================================================================

print("Demo 3: Dual Channel Comparison");
print("─────────────────────────────────────────────────────────");

// Generate triangle wave on both outputs
let triangle_samples = 50;

print("  Generating triangle wave on both AO channels...");
print("");
print("  Sample |  AO0 (V)  |  AO1 (V)  |  AI0 (V)  |  AI1 (V)  | AI Diff");
print("─────────────────────────────────────────────────────────");

for i in 0..triangle_samples {
    // Triangle wave
    let phase = i / (triangle_samples / 2.0);
    let v_triangle = if phase < 1.0 {
        phase * amplitude
    } else {
        (2.0 - phase) * amplitude
    };

    // Write to both outputs (AO0 = triangle, AO1 = inverted)
    ao.write(ao_stimulus, offset + v_triangle);
    ao.write(ao_reference, offset + amplitude - v_triangle);

    sleep(sample_period);

    // Read both inputs
    let ai0 = ai.read(ai_response);
    let ai1 = ai.read(ai_reference);
    let ai_diff = ai0 - ai1;

    if i % 5 == 0 {
        print(`   ${i:4}  | ${(offset + v_triangle):8.4}  | ${(offset + amplitude - v_triangle):8.4}  | ${ai0:8.4}  | ${ai1:8.4}  | ${ai_diff:+7.4}`);
    }
}

print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Cleanup
// =============================================================================

print("Cleaning up...");
ao.write(ao_stimulus, 0.0);
ao.write(ao_reference, 0.0);
print("  ✓ All outputs zeroed");
print("");

// =============================================================================
// Summary
// =============================================================================

print("Synchronized acquisition complete!");
print("");
print("Demos performed:");
print("  1. Sine wave stimulus-response with correlation analysis");
print("  2. Step response measurement");
print("  3. Dual channel comparison");
print("");
print("For higher-speed synchronized acquisition, use hardware");
print("triggering and the streaming acquisition API.");
