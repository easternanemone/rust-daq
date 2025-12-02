# ScriptEngine Documentation

Welcome to the rust-daq ScriptEngine documentation. This directory contains comprehensive guides for writing and understanding Rhai scripts that control hardware in the V5 Headless-First architecture.

## Quick Start

```bash
# Run your first script
cargo run -- run examples/simple_scan.rhai

# Validate script syntax (coming soon)
cargo run -- validate my_experiment.rhai
```

## Documentation Structure

### For Users (Scientists/Engineers)

1. **[SCRIPTING_OVERVIEW.md](./SCRIPTING_OVERVIEW.md)** - START HERE
   - What is the ScriptEngine and why use it?
   - Architecture overview
   - Safety features
   - When to use scripts vs compiled Rust

2. **[RHAI_API_REFERENCE.md](./RHAI_API_REFERENCE.md)** - API Documentation
   - Complete function reference
   - Stage control (`move_abs`, `position`, `wait_settled`)
   - Camera control (`arm`, `trigger`, `resolution`)
   - Built-in Rhai features (loops, conditionals, functions)
   - Error handling guide

3. **[SCRIPTING_EXAMPLES.md](./SCRIPTING_EXAMPLES.md)** - Cookbook
   - Common patterns and recipes
   - Basic examples (scans, acquisitions)
   - Advanced patterns (adaptive scanning, error recovery)
   - Performance tips
   - Troubleshooting guide

### For Developers (Rust Programmers)

4. **[ASYNC_BRIDGE_GUIDE.md](./ASYNC_BRIDGE_GUIDE.md)** - Implementation Deep Dive
   - How synchronous Rhai calls async Rust
   - `block_in_place` pattern explained
   - Performance characteristics
   - Common pitfalls and solutions
   - Testing strategies

## Quick Reference Card

### Stage Control
```rhai
// Motion
stage.move_abs(10.0);       // Move to absolute position
stage.move_rel(5.0);        // Move relative distance
stage.wait_settled();       // Wait for motion to complete

// Query
let pos = stage.position(); // Get current position
```

### Camera Control
```rhai
// Acquisition
camera.arm();               // Prepare for trigger
camera.trigger();           // Capture frame

// Query
let res = camera.resolution();  // [width, height]
```

### Utilities
```rhai
sleep(0.5);                 // Wait 500ms
print("Hello");             // Print to console
```

## Example Script

```rhai
// triggered_acquisition.rhai
print("Starting triggered acquisition...");

camera.arm();

for i in 0..10 {
    let pos = i * 1.0;

    // Move to position
    stage.move_abs(pos);
    stage.wait_settled();

    // Capture frame
    camera.trigger();
    print(`Frame ${i+1} at ${pos}mm`);
}

print("Acquisition complete!");
```

## Common Use Cases

### Scientific Workflows
- **Z-stack acquisition** - Scan microscope focus through sample
- **Spectral sweeps** - Tune wavelength, capture spectrum
- **Calibration routines** - Visit reference positions
- **Time series** - Capture frames at regular intervals

### Engineering Tasks
- **Stage characterization** - Measure positioning accuracy
- **Camera testing** - Validate trigger timing
- **Integration tests** - Verify hardware coordination
- **Automated QA** - Daily instrument checks

## Safety Features

### Operation Limit
All scripts are limited to **10,000 operations** to prevent infinite loops:
```rhai
// This will auto-terminate after 10,000 iterations
while true {
    stage.move_abs(0.0);  // UNSAFE but won't hang
}
// → Error: Safety limit exceeded
```

### Error Propagation
Hardware errors immediately terminate scripts:
```rhai
stage.move_abs(1000.0);  // Out of range
print("Never reached");  // Not executed
// → Error: Position exceeds maximum travel
```

## Performance Guidelines

| Operation Type | Recommended Max Frequency | Notes |
|---------------|--------------------------|-------|
| Stage movements | 10 Hz | Limited by motor acceleration |
| Camera triggers | 60 Hz | Limited by exposure time |
| Position queries | 100 Hz | Minimal overhead |
| Script loops | 10,000 iterations | Safety limit |

**For higher frequencies:** Use compiled Rust instead of scripts.

## Architecture Integration

Scripts run on the **headless server**, not the GUI client:

```
┌──────────────┐              ┌───────────────────┐
│ Remote GUI   │  ← gRPC →    │ Headless Server   │
│              │              │                   │
│ - Display    │              │ - Rhai Scripts    │
│ - Controls   │              │ - Hardware        │
└──────────────┘              │ - Data Storage    │
                               └───────────────────┘
```

Scripts interact with hardware through **V5 Capability Traits**:
- `Movable` - Stages, actuators, rotators
- `Camera` - Triggered acquisition devices
- `Readable` - Power meters, sensors (future)

## Limitations

### Current Restrictions
- **One stage, one camera** - Multi-device support planned
- **No data access** - Use Python client for analysis
- **No custom hardware config** - Pre-configured in Rust
- **Synchronous only** - No async/await in scripts

### When NOT to Use Scripts
- Real-time control (< 1ms latency required)
- High-frequency acquisition (> 1kHz)
- Complex data analysis (use Python)
- Safety-critical operations (use tested Rust)

## Getting Help

### Documentation Resources
1. **This directory** - Comprehensive scripting guides
2. **[V5 Architecture](../architecture/V5_ARCHITECTURE.md)** - Overall system design
3. **[Hardware Inventory](../HARDWARE_INVENTORY.md)** - Supported devices
4. **[Rhai Language Book](https://rhai.rs/book/)** - Rhai language reference

### Example Scripts
- `examples/simple_scan.rhai` - Basic stage scan
- `examples/triggered_acquisition.rhai` - Stage + camera
- `examples/error_demo.rhai` - Error handling
- `examples/scripting_demo.rhai` - All features

### Common Questions

**Q: Can I import external libraries?**
A: No, Rhai doesn't support imports. All functionality is pre-registered.

**Q: How do I debug scripts?**
A: Use `print()` statements. Output appears in server logs.

**Q: Can I access camera data in scripts?**
A: No, frame data goes directly to HDF5 storage. Use Python client for analysis.

**Q: What if my script takes too long?**
A: Scripts run until completion or error. Add `print()` statements for progress.

**Q: Can I control multiple stages?**
A: Not yet. Multi-device support is planned for future release.

## Version History

- **V5.0** (2025-11-18) - Initial ScriptEngine implementation
  - Rhai integration with async bridge
  - Stage and camera control
  - Safety limits and error handling
  - Headless-first architecture

## Contributing

When adding new hardware capabilities to scripts:

1. Implement the capability trait in Rust (e.g., `Readable`)
2. Add handle type in `src/scripting/bindings.rs`
3. Register methods with Rhai engine
4. Add unit tests
5. Update this documentation
6. Add example scripts

See [ASYNC_BRIDGE_GUIDE.md](./ASYNC_BRIDGE_GUIDE.md) for implementation details.

---

**Next Steps:**
- Read [SCRIPTING_OVERVIEW.md](./SCRIPTING_OVERVIEW.md) for concepts
- Check [RHAI_API_REFERENCE.md](./RHAI_API_REFERENCE.md) for functions
- Try examples from [SCRIPTING_EXAMPLES.md](./SCRIPTING_EXAMPLES.md)
