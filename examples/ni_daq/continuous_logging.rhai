// NI DAQ Continuous Data Logging Example
//
// This script demonstrates continuous time-series acquisition:
// - Acquire multi-channel analog data at fixed sample rate
// - Display live statistics and rate
// - Accumulate data for duration
// - Save summary statistics at end
//
// Hardware Setup:
//   AI0-AI3 ← Signals to log (or loopback from AO for testing)
//   Optionally: AO0 → AI0 (loopback for testing)
//
// Usage:
//   cargo run --bin rust-daq-daemon -- run examples/ni_daq/continuous_logging.rhai
//
// Requires: NI DAQ with Comedi driver configured

print("╔════════════════════════════════════════════════════╗");
print("║  NI DAQ Continuous Data Logging Example            ║");
print("║  NI PCI-MIO-16XE-10 / Comedi Driver                ║");
print("╚════════════════════════════════════════════════════╝");
print("");

// =============================================================================
// Configuration
// =============================================================================

let sample_rate = 100.0;       // Target sample rate (Hz)
let duration = 5.0;            // Logging duration (seconds)
let ai_channels = [0, 1, 2, 3]; // Channels to log
let display_interval = 1.0;    // Stats display interval (seconds)

let sample_period = 1.0 / sample_rate;
let total_samples = (duration * sample_rate).to_int();

print("Logging Parameters:");
print(`  Sample Rate:   ${sample_rate} Hz`);
print(`  Duration:      ${duration} s`);
print(`  Total Samples: ${total_samples}`);
print(`  AI Channels:   ${ai_channels}`);
print("");

// =============================================================================
// Display Hardware Configuration
// =============================================================================

print("Hardware Configuration:");
for ch in ai_channels {
    let range = ai.range(ch);
    print(`  AI${ch}: ${range.min} to ${range.max} V`);
}
print("");

// =============================================================================
// Initialize Data Structures
// =============================================================================

// Running statistics per channel
let channel_sums = [];
let channel_sums_sq = [];
let channel_mins = [];
let channel_maxs = [];

for ch in ai_channels {
    channel_sums.push(0.0);
    channel_sums_sq.push(0.0);
    channel_mins.push(1e10);
    channel_maxs.push(-1e10);
}

// =============================================================================
// Optional: Generate Test Signal on AO0
// =============================================================================

// Uncomment to output a test sine wave on AO0
// print("Generating test signal on AO0...");
// ao.write(0, 2.5);  // DC offset for testing

// =============================================================================
// Start Continuous Acquisition
// =============================================================================

print("Starting continuous acquisition...");
print("─────────────────────────────────────────────────────────");

let start_time = timestamp();
let last_display_time = start_time;
let samples_acquired = 0;
let display_count = 0;

// Header for periodic stats
print("  Time (s) |  Samples  |  Rate (Hz)  |  AI0 (V)  |  AI1 (V)");
print("─────────────────────────────────────────────────────────");

while samples_acquired < total_samples {
    let sample_start = timestamp();

    // Read all channels
    for ch_idx in 0..ai_channels.len() {
        let v = ai.read(ai_channels[ch_idx]);

        // Update running statistics
        channel_sums[ch_idx] += v;
        channel_sums_sq[ch_idx] += v * v;
        if v < channel_mins[ch_idx] { channel_mins[ch_idx] = v; }
        if v > channel_maxs[ch_idx] { channel_maxs[ch_idx] = v; }
    }

    samples_acquired += 1;

    // Display periodic statistics
    let elapsed = (timestamp() - last_display_time) / 1000.0;
    if elapsed >= display_interval {
        let total_elapsed = (timestamp() - start_time) / 1000.0;
        let current_rate = samples_acquired / total_elapsed;

        // Get current readings for display
        let v0 = ai.read(ai_channels[0]);
        let v1 = if ai_channels.len() > 1 { ai.read(ai_channels[1]) } else { 0.0 };

        print(`  ${total_elapsed:7.2}  |   ${samples_acquired:5}   |    ${current_rate:6.1}   |  ${v0:7.4}  |  ${v1:7.4}`);

        last_display_time = timestamp();
        display_count += 1;
    }

    // Sleep to maintain sample rate (approximate)
    let sample_elapsed = (timestamp() - sample_start) / 1000.0;
    let sleep_time = sample_period - sample_elapsed;
    if sleep_time > 0.001 {
        sleep(sleep_time);
    }
}

print("─────────────────────────────────────────────────────────");

let total_elapsed = (timestamp() - start_time) / 1000.0;
let actual_rate = samples_acquired / total_elapsed;

// =============================================================================
// Calculate Final Statistics
// =============================================================================

print("");
print("Final Statistics:");
print("─────────────────────────────────────────────────────────");
print("  Chan |   Mean    |  Std Dev  |    Min    |    Max    |   Range");
print("─────────────────────────────────────────────────────────");

for ch_idx in 0..ai_channels.len() {
    let n = samples_acquired;
    let mean = channel_sums[ch_idx] / n;
    let variance = (channel_sums_sq[ch_idx] / n) - (mean * mean);
    let std_dev = if variance > 0.0 { sqrt(variance) } else { 0.0 };
    let min_v = channel_mins[ch_idx];
    let max_v = channel_maxs[ch_idx];
    let range_v = max_v - min_v;

    print(`  AI${ai_channels[ch_idx]}  | ${mean:9.5} | ${std_dev:9.5} | ${min_v:9.5} | ${max_v:9.5} | ${range_v:9.5}`);
}

print("─────────────────────────────────────────────────────────");

// =============================================================================
// Summary
// =============================================================================

print("");
print("Acquisition Summary:");
print(`  Total samples:     ${samples_acquired}`);
print(`  Total duration:    ${total_elapsed:.3} s`);
print(`  Target rate:       ${sample_rate} Hz`);
print(`  Actual rate:       ${actual_rate:.2} Hz`);
print(`  Rate accuracy:     ${(actual_rate / sample_rate * 100.0):.1}%`);
print("");

// Note about data storage
print("Note: This example calculates statistics in-memory.");
print("For persistent storage, use the ComediStreamWriter from daq-storage.");
print("");
print("Logging complete!");
