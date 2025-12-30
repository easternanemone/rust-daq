// NI DAQ Triggered Acquisition Example
//
// This script demonstrates hardware-triggered data acquisition:
// - Arm counter for external trigger detection
// - Wait for trigger pulse
// - Acquire multi-channel data on trigger
// - Display results with timing information
//
// Hardware Setup:
//   PFI0 (Pin 10) ← External trigger source (TTL pulse)
//   AI0-AI3 ← Signals to acquire
//
// Usage:
//   cargo run --bin rust-daq-daemon -- run examples/ni_daq/triggered_acquisition.rhai
//
// Requires: NI DAQ with Comedi driver configured

print("╔════════════════════════════════════════════════════╗");
print("║  NI DAQ Triggered Acquisition Example              ║");
print("║  NI PCI-MIO-16XE-10 / Comedi Driver                ║");
print("╚════════════════════════════════════════════════════╝");
print("");

// =============================================================================
// Configuration
// =============================================================================

let num_triggers = 10;         // Number of triggers to capture
let trigger_timeout = 5.0;     // Timeout waiting for trigger (seconds)
let ai_channels = [0, 1, 2, 3]; // Channels to acquire
let counter_channel = 0;       // Counter used for trigger detection

print("Acquisition Parameters:");
print(`  Triggers to capture: ${num_triggers}`);
print(`  Timeout:             ${trigger_timeout} s`);
print(`  AI Channels:         ${ai_channels}`);
print("");

// =============================================================================
// Display Hardware Configuration
// =============================================================================

print("Hardware Configuration:");
for ch in ai_channels {
    let range = ai.range(ch);
    print(`  AI${ch}: ${range.min} to ${range.max} V`);
}
print(`  Counter ${counter_channel}: Edge detection mode`);
print("");

// =============================================================================
// Initialize Counter for Trigger Detection
// =============================================================================

print("Initializing trigger detection...");
counter.reset(counter_channel);
counter.arm(counter_channel);
print("  ✓ Counter armed for trigger detection");
print("  Waiting for external trigger on PFI0...");
print("");

// =============================================================================
// Triggered Acquisition Loop
// =============================================================================

print("Starting triggered acquisition...");
print("─────────────────────────────────────────────────────────");
print("  Trig# |  AI0 (V)  |  AI1 (V)  |  AI2 (V)  |  AI3 (V)");
print("─────────────────────────────────────────────────────────");

let trigger_count = 0;
let last_count = counter.read(counter_channel);
let start_time = timestamp();
let data = [];

while trigger_count < num_triggers {
    // Check if we've exceeded timeout
    let elapsed = timestamp() - start_time;
    if elapsed > trigger_timeout * 1000.0 {
        print("");
        print(`  ⚠ Timeout after ${elapsed / 1000.0:.1} seconds`);
        break;
    }

    // Check for trigger (counter increment)
    let current_count = counter.read(counter_channel);
    if current_count > last_count {
        // Trigger detected!
        trigger_count += 1;
        last_count = current_count;

        // Read all channels immediately
        let readings = [];
        for ch in ai_channels {
            readings.push(ai.read(ch));
        }

        // Store data
        data.push(readings);

        // Display results
        let trig_str = if trigger_count < 10 { ` ${trigger_count}` } else { `${trigger_count}` };
        print(`   ${trig_str}   |  ${readings[0]:7.4}  |  ${readings[1]:7.4}  |  ${readings[2]:7.4}  |  ${readings[3]:7.4}`);
    }

    // Small delay to avoid CPU spinning
    sleep(0.001);
}

print("─────────────────────────────────────────────────────────");

// =============================================================================
// Disarm Counter
// =============================================================================

counter.disarm(counter_channel);
print("");
print("  ✓ Counter disarmed");

// =============================================================================
// Calculate Statistics per Channel
// =============================================================================

if data.len() > 0 {
    print("");
    print("Channel Statistics:");
    print("─────────────────────────────────────────────────────────");
    print("  Chan |   Mean    |  Std Dev  |    Min    |    Max");
    print("─────────────────────────────────────────────────────────");

    for ch_idx in 0..ai_channels.len() {
        let sum = 0.0;
        let sum_sq = 0.0;
        let min_v = data[0][ch_idx];
        let max_v = data[0][ch_idx];

        for sample in data {
            let v = sample[ch_idx];
            sum += v;
            sum_sq += v * v;
            if v < min_v { min_v = v; }
            if v > max_v { max_v = v; }
        }

        let n = data.len();
        let mean = sum / n;
        let variance = (sum_sq / n) - (mean * mean);
        let std_dev = if variance > 0.0 { sqrt(variance) } else { 0.0 };

        print(`  AI${ai_channels[ch_idx]}  | ${mean:9.5} | ${std_dev:9.5} | ${min_v:9.5} | ${max_v:9.5}`);
    }

    print("─────────────────────────────────────────────────────────");
}

// =============================================================================
// Summary
// =============================================================================

print("");
print("Acquisition Summary:");
print(`  Triggers captured: ${trigger_count} of ${num_triggers}`);
print(`  Elapsed time:      ${(timestamp() - start_time) / 1000.0:.2} s`);
if trigger_count > 0 {
    let rate = trigger_count / ((timestamp() - start_time) / 1000.0);
    print(`  Effective rate:    ${rate:.2} triggers/s`);
}
print("");
print("Acquisition complete!");
