// Demo Scan Script for rust-daq
//
// ⚠️  NOTE: This demo requires features planned for v0.6.0
// ⚠️  Currently, daemon mode does not register device globals from config
//
// This script demonstrates what will be possible in v0.6.0:
// - Multi-device workflows (stage + power_meter)
// - Config-based device loading in daemon mode
// - gRPC script execution with full hardware access
//
// For a working demo today, see: examples/demo_camera.rhai
//
// PLANNED Usage (v0.6.0):
//   1. Start daemon with demo config:
//      cargo run --bin rust-daq-daemon -- daemon --hardware-config config/demo.toml
//
//   2. Upload and run this script via gRPC
//      (GUI Scripts panel will support this in v0.6.0)

print("╔════════════════════════════════════════╗");
print("║  rust-daq Demo: Automated Scan        ║");
print("║  (Requires v0.6.0 features)            ║");
print("╚════════════════════════════════════════╝");
print("");

// Configuration
let scan_start = 0.0;       // mm
let scan_end = 10.0;        // mm
let num_points = 11;
let step_size = (scan_end - scan_start) / (num_points - 1);

print(`Scan Parameters:`);
print(`  Range: ${scan_start} to ${scan_end} mm`);
print(`  Points: ${num_points}`);
print(`  Step: ${step_size} mm`);
print("");

// Perform scan
print("Starting scan...");
print("─────────────────────────────────────────");

for i in 0..num_points {
    let position = scan_start + (i * step_size);

    // Move stage to position
    // NOTE: Requires device globals registered from config (v0.6.0)
    stage.move_abs(position);

    // Wait for stage to settle
    sleep(0.05);

    // Read power meter
    // NOTE: Requires ReadableHandle binding (v0.6.0)
    let power = power_meter.read();

    print(`[${i+1}/${num_points}] Position: ${position:0.2} mm → Power: ${power:0.3e} W`);

    sleep(0.1);
}

print("─────────────────────────────────────────");
print("Scan complete!");
print("");
print("This feature is coming in v0.6.0!");
print("For now, try examples/demo_camera.rhai for a working demo.");
