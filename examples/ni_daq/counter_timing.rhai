// NI DAQ Counter Timing Example
//
// This script demonstrates counter/timer operations:
// - Event counting on external inputs
// - Frequency measurement
// - Period measurement
// - Pulse width timing
// - Rate calculation
//
// Hardware Setup:
//   CTR0 (GPCTR0) ← External pulse source (PFI8/GPCTR0_SOURCE)
//   CTR1 (GPCTR1) ← Secondary timing source (PFI3/GPCTR1_SOURCE)
//
// Usage:
//   cargo run --bin rust-daq-daemon -- run examples/ni_daq/counter_timing.rhai
//
// Requires: NI DAQ with Comedi driver configured

print("╔════════════════════════════════════════════════════╗");
print("║  NI DAQ Counter Timing Example                     ║");
print("║  NI PCI-MIO-16XE-10 / Comedi Driver                ║");
print("╚════════════════════════════════════════════════════╝");
print("");

// =============================================================================
// Configuration
// =============================================================================

let counter_0 = 0;           // Primary counter
let counter_1 = 1;           // Secondary counter
let measurement_time = 1.0;  // Gate time for frequency measurement (seconds)
let num_readings = 10;       // Number of readings to average

print("Counter Configuration:");
print(`  Counter 0: Event counting / frequency`);
print(`  Counter 1: Period / timing`);
print(`  Gate time: ${measurement_time} s`);
print(`  Readings:  ${num_readings}`);
print("");

// =============================================================================
// Display Counter Status
// =============================================================================

print("Counter Status:");
print(`  Counter 0 count: ${counter.read(counter_0)}`);
print(`  Counter 1 count: ${counter.read(counter_1)}`);
print(`  Number of counters: ${counter.channel_count()}`);
print("");

// =============================================================================
// Demo 1: Basic Event Counting
// =============================================================================

print("Demo 1: Basic Event Counting");
print("─────────────────────────────────────────────────────────");

// Reset and arm counter
counter.reset(counter_0);
counter.arm(counter_0);
print("  Counter 0 armed and counting...");

// Count events for a fixed duration
let start_count = counter.read(counter_0);
let start_time = timestamp();

sleep(measurement_time);

let end_count = counter.read(counter_0);
let elapsed = (timestamp() - start_time) / 1000.0;
let events = end_count - start_count;

print(`  Initial count: ${start_count}`);
print(`  Final count:   ${end_count}`);
print(`  Events:        ${events}`);
print(`  Duration:      ${elapsed:.3} s`);

counter.disarm(counter_0);
print("  Counter 0 disarmed");
print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Demo 2: Frequency Measurement
// =============================================================================

print("Demo 2: Frequency Measurement (Rate Calculation)");
print("─────────────────────────────────────────────────────────");

let frequencies = [];

for i in 0..num_readings {
    counter.reset(counter_0);
    counter.arm(counter_0);

    let c1 = counter.read(counter_0);
    let t1 = timestamp();

    sleep(measurement_time);

    let c2 = counter.read(counter_0);
    let t2 = timestamp();

    counter.disarm(counter_0);

    let dt = (t2 - t1) / 1000.0;
    let dc = c2 - c1;
    let freq = dc / dt;
    frequencies.push(freq);

    print(`  Reading ${i+1}: ${dc} counts / ${dt:.3}s = ${freq:.2} Hz`);
}

// Calculate statistics
let sum = 0.0;
let sum_sq = 0.0;
for f in frequencies {
    sum += f;
    sum_sq += f * f;
}

let mean_freq = sum / frequencies.len();
let variance = (sum_sq / frequencies.len()) - (mean_freq * mean_freq);
let std_freq = if variance > 0.0 { sqrt(variance) } else { 0.0 };

print("");
print("  Frequency Statistics:");
print(`    Mean:     ${mean_freq:.3} Hz`);
print(`    Std Dev:  ${std_freq:.3} Hz`);
print(`    Samples:  ${frequencies.len()}`);
print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Demo 3: Edge Detection and Timing
// =============================================================================

print("Demo 3: Edge Detection (Waiting for Pulses)");
print("─────────────────────────────────────────────────────────");

counter.reset(counter_0);
counter.arm(counter_0);

print("  Waiting for rising edges on CTR0 input...");
print("  (Connect a pulse source or press Ctrl+C to skip)");

let edge_times = [];
let last_count = counter.read(counter_0);
let wait_start = timestamp();
let timeout = 5.0;  // 5 second timeout

while edge_times.len() < 5 {
    let current_count = counter.read(counter_0);

    if current_count > last_count {
        // Edge detected
        let edge_time = (timestamp() - wait_start) / 1000.0;
        edge_times.push(edge_time);
        print(`    Edge ${edge_times.len()}: ${edge_time:.6} s (count: ${current_count})`);
        last_count = current_count;
    }

    // Check timeout
    if (timestamp() - wait_start) / 1000.0 > timeout {
        print("    Timeout - no more edges detected");
        break;
    }

    sleep(0.001);
}

counter.disarm(counter_0);

// Calculate inter-edge intervals
if edge_times.len() >= 2 {
    print("");
    print("  Inter-edge intervals:");
    for i in 1..edge_times.len() {
        let interval = edge_times[i] - edge_times[i-1];
        let freq = 1.0 / interval;
        print(`    Edge ${i} to ${i+1}: ${interval * 1000.0:.3} ms (${freq:.2} Hz)`);
    }
}

print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Demo 4: Dual Counter Comparison
// =============================================================================

print("Demo 4: Dual Counter Comparison");
print("─────────────────────────────────────────────────────────");

counter.reset(counter_0);
counter.reset(counter_1);
counter.arm(counter_0);
counter.arm(counter_1);

print("  Both counters armed...");

for i in 0..5 {
    sleep(measurement_time / 5.0);
    let c0 = counter.read(counter_0);
    let c1 = counter.read(counter_1);
    let elapsed = (i + 1) * (measurement_time / 5.0);
    print(`    t=${elapsed:.2}s: CTR0=${c0}, CTR1=${c1}, diff=${c0 - c1}`);
}

counter.disarm(counter_0);
counter.disarm(counter_1);
print("  Both counters disarmed");

print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Demo 5: Counter Reset Verification
// =============================================================================

print("Demo 5: Counter Reset Verification");
print("─────────────────────────────────────────────────────────");

// Read current value
let pre_reset = counter.read(counter_0);
print(`  Before reset: ${pre_reset}`);

// Reset
counter.reset(counter_0);

// Read again
let post_reset = counter.read(counter_0);
print(`  After reset:  ${post_reset}`);

if post_reset == 0 {
    print("  ✓ Reset successful");
} else {
    print("  ⚠ Counter not zeroed (may be normal for some modes)");
}

print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Summary
// =============================================================================

print("Counter operations complete!");
print("");
print("Demos performed:");
print("  1. Basic event counting");
print("  2. Frequency measurement with statistics");
print("  3. Edge detection and timing");
print("  4. Dual counter comparison");
print("  5. Counter reset verification");
print("");
print("For hardware timer modes (PWM, pulse generation),");
print("see the NI-DAQmx or Comedi documentation for your card.");
