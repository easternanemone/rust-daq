// Demo Camera Script for rust-daq
//
// This script demonstrates camera acquisition using mock devices.
// No physical hardware required!
//
// Usage (one-shot mode - no daemon needed):
//   cargo run --bin rust-daq-daemon -- run examples/demo_camera.rhai
//
// This demo uses hardcoded mock devices:
//   - MockCamera (1920x1080, automatically initialized)
//   - MockStage (automatically initialized)
//
// For daemon-based workflows with config files, see v0.6.0 features.

print("╔════════════════════════════════════════╗");
print("║  rust-daq Demo: Camera Acquisition    ║");
print("╚════════════════════════════════════════╝");
print("");

// Camera configuration
let num_frames = 5;

print(`Camera Configuration:`);
print(`  Device: MockCamera (1920x1080)`);
print(`  Frames: ${num_frames}`);
print("");

// Arm camera for triggered acquisition
print("Arming camera for acquisition...");
camera.arm();
print("  ✓ Camera armed and ready");
print("");

// Capture frames
print("Capturing frames...");
print("─────────────────────────────────────────");

for i in 0..num_frames {
    // Trigger frame capture
    camera.trigger();
    
    print(`  [${i+1}/${num_frames}] Frame captured`);
    
    // Small delay between frames (mock camera responds instantly)
    sleep(0.2);
}

print("─────────────────────────────────────────");
print("Acquisition complete!");
print("");
print("Next steps:");
print("  • Use rust-daq-gui for live camera streaming (start daemon first)");
print("  • See crates/daq-examples/ for more advanced workflows");
print("  • Try config/demo.toml for daemon-based device loading");
