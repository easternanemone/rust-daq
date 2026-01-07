// NI DAQ Digital I/O Sequencer Example
//
// This script demonstrates digital output pattern generation:
// - Configure DIO pins as outputs
// - Generate timing patterns for external devices
// - Binary counter sequence
// - Custom bit patterns
// - Handshaking with external equipment
//
// Hardware Setup:
//   DIO0-DIO7 → External device control lines
//   DIO8 ← External device ready signal (optional)
//
// Usage:
//   cargo run --bin rust-daq-daemon -- run examples/ni_daq/dio_sequencer.rhai
//
// Requires: NI DAQ with Comedi driver configured

print("╔════════════════════════════════════════════════════╗");
print("║  NI DAQ Digital I/O Sequencer Example              ║");
print("║  NI PCI-MIO-16XE-10 / Comedi Driver                ║");
print("╚════════════════════════════════════════════════════╝");
print("");

// =============================================================================
// Configuration
// =============================================================================

let output_pins = [0, 1, 2, 3, 4, 5, 6, 7];  // 8-bit output port
let input_pin = 8;                            // Ready signal input
let step_delay = 0.1;                         // Delay between patterns (seconds)

print("Sequencer Configuration:");
print(`  Output Pins: DIO${output_pins[0]}-DIO${output_pins[7]}`);
print(`  Input Pin:   DIO${input_pin} (ready signal)`);
print(`  Step Delay:  ${step_delay * 1000} ms`);
print("");

// =============================================================================
// Configure Pin Directions
// =============================================================================

print("Configuring pin directions...");

// Set outputs
for pin in output_pins {
    dio.set_output(pin);
}
print(`  ✓ DIO0-DIO7 configured as outputs`);

// Set input
dio.set_input(input_pin);
print(`  ✓ DIO8 configured as input`);
print("");

// =============================================================================
// Initialize - All outputs LOW
// =============================================================================

print("Initializing outputs to LOW...");
dio.write_port(0x00);
print("  ✓ All outputs LOW");
print("");

// =============================================================================
// Pattern 1: Binary Counter (8-bit)
// =============================================================================

print("Pattern 1: Binary Counter (0-15)");
print("─────────────────────────────────────────────────────────");

for i in 0..16 {
    dio.write_port(i);

    // Display binary representation
    let bin_str = "";
    for bit in 0..8 {
        let b = 7 - bit;
        bin_str += if (i & (1 << b)) != 0 { "1" } else { "0" };
    }

    print(`  Count ${i}: [${bin_str}]`);
    sleep(step_delay);
}

print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Pattern 2: Walking Ones
// =============================================================================

print("Pattern 2: Walking Ones");
print("─────────────────────────────────────────────────────────");

for i in 0..8 {
    let pattern = 1 << i;
    dio.write_port(pattern);

    let bin_str = "";
    for bit in 0..8 {
        let b = 7 - bit;
        bin_str += if (pattern & (1 << b)) != 0 { "█" } else { "░" };
    }

    print(`  Bit ${i}: [${bin_str}]`);
    sleep(step_delay);
}

print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Pattern 3: Walking Zeros
// =============================================================================

print("Pattern 3: Walking Zeros");
print("─────────────────────────────────────────────────────────");

for i in 0..8 {
    let pattern = 0xFF ^ (1 << i);
    dio.write_port(pattern);

    let bin_str = "";
    for bit in 0..8 {
        let b = 7 - bit;
        bin_str += if (pattern & (1 << b)) != 0 { "█" } else { "░" };
    }

    print(`  Bit ${i}: [${bin_str}]`);
    sleep(step_delay);
}

print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Pattern 4: Knight Rider (Bouncing Light)
// =============================================================================

print("Pattern 4: Knight Rider");
print("─────────────────────────────────────────────────────────");

// Forward sweep
for i in 0..8 {
    let pattern = 1 << i;
    dio.write_port(pattern);

    let bin_str = "";
    for bit in 0..8 {
        let b = 7 - bit;
        bin_str += if (pattern & (1 << b)) != 0 { "█" } else { "░" };
    }

    print(`  [${bin_str}]`);
    sleep(step_delay / 2.0);
}

// Backward sweep
for i in 0..8 {
    let pattern = 1 << (7 - i);
    dio.write_port(pattern);

    let bin_str = "";
    for bit in 0..8 {
        let b = 7 - bit;
        bin_str += if (pattern & (1 << b)) != 0 { "█" } else { "░" };
    }

    print(`  [${bin_str}]`);
    sleep(step_delay / 2.0);
}

print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Pattern 5: Custom Sequence
// =============================================================================

print("Pattern 5: Custom Control Sequence");
print("─────────────────────────────────────────────────────────");

// Define custom patterns (e.g., for stepper motor or device control)
let custom_patterns = [
    0b00000001,  // Step 1
    0b00000011,  // Transition
    0b00000010,  // Step 2
    0b00000110,  // Transition
    0b00000100,  // Step 3
    0b00001100,  // Transition
    0b00001000,  // Step 4
    0b00001001,  // Transition back
];

let pattern_names = [
    "Step 1      ",
    "Transition 1",
    "Step 2      ",
    "Transition 2",
    "Step 3      ",
    "Transition 3",
    "Step 4      ",
    "Transition 4",
];

for idx in 0..custom_patterns.len() {
    let pattern = custom_patterns[idx];
    dio.write_port(pattern);

    let bin_str = "";
    for bit in 0..8 {
        let b = 7 - bit;
        bin_str += if (pattern & (1 << b)) != 0 { "█" } else { "░" };
    }

    print(`  ${pattern_names[idx]}: [${bin_str}] (0x${pattern:02X})`);
    sleep(step_delay);
}

print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Input Monitoring Demo
// =============================================================================

print("Pattern 6: Input Monitoring");
print("─────────────────────────────────────────────────────────");
print("  Monitoring DIO8 for external ready signal...");

for i in 0..10 {
    let ready = dio.read(input_pin);
    let ready_str = if ready { "HIGH ✓" } else { "LOW  ○" };
    print(`  Sample ${i+1}: DIO8 = ${ready_str}`);
    sleep(step_delay);
}

print("─────────────────────────────────────────────────────────");
print("");

// =============================================================================
// Cleanup - All outputs LOW
// =============================================================================

print("Cleaning up...");
dio.write_port(0x00);
print("  ✓ All outputs set to LOW");
print("");

// =============================================================================
// Summary
// =============================================================================

print("Sequencer complete!");
print("");
print("Patterns demonstrated:");
print("  1. Binary counter (0-15)");
print("  2. Walking ones");
print("  3. Walking zeros");
print("  4. Knight rider (bouncing)");
print("  5. Custom control sequence");
print("  6. Input monitoring");
