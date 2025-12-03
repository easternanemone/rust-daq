# Quick Start Guide

## Testing the Tauri GUI with rust-daq Daemon

### Step 1: Start the daemon

From the `rust-daq` root directory:

```bash
# Start daemon with networking enabled and mock hardware
cargo run --features networking,all_hardware -- daemon --port 50051
```

The daemon will:
- Listen on `localhost:50051` for gRPC connections
- Register mock devices (mock_stage, mock_camera, etc.)
- Wait for GUI connection

### Step 2: Install GUI dependencies

From the `gui-tauri` directory:

```bash
cd gui-tauri
npm install
```

### Step 3: Run the GUI

```bash
npm run tauri dev
```

This will:
1. Start Vite dev server on `localhost:1420`
2. Open the Tauri window
3. Show the "Connect to Daemon" screen

### Step 4: Connect to Daemon

1. Click "Connect to Daemon" button
2. Enter address: `localhost:50051` (default)
3. Click "Connect"

You should see:
- List of mock devices in the left panel
- Live status updates for each device
- Ability to click and control devices

### Step 5: Test Device Control

**Mock Stage:**
1. Click "mock_stage" in the device list
2. Enter a position (e.g., `10.5`)
3. Click "Move (Async)"
4. Watch position update in real-time

**Mock Power Meter:**
1. Click "mock_power_meter"
2. Click "Read Value"
3. See random power reading with units

## Troubleshooting

### "Failed to connect" Error

**Problem:** GUI can't reach daemon

**Solution:**
```bash
# Check daemon is running
lsof -i :50051

# Restart daemon
cargo run --features networking,all_hardware -- daemon --port 50051
```

### "No devices found"

**Problem:** Daemon started without hardware

**Solution:**
```bash
# Ensure all_hardware feature is enabled
cargo run --features networking,all_hardware -- daemon --port 50051
```

### Build errors

**Problem:** Missing dependencies

**Solution:**
```bash
# Clean and reinstall
rm -rf node_modules
npm install
```

## Development Tips

### Hot Reload

The Tauri dev mode supports hot reload:
- Edit React components in `src/` → auto-reload
- Edit Rust code in `src-tauri/src/` → needs manual restart

### Debugging

**Frontend:**
- Open DevTools: Right-click → "Inspect Element"
- Check console for React errors

**Backend:**
- Check terminal for Rust panics/errors
- Add `println!()` statements in `src-tauri/src/main.rs`

### Testing with Real Hardware

To test with real devices:

1. Configure hardware in daemon config
2. Start daemon with appropriate features:
   ```bash
   cargo run --features networking,instrument_thorlabs,instrument_newport -- daemon --port 50051
   ```
3. Connect GUI and control real devices

### Next Steps

Once basic functionality is verified:
1. Test with multiple devices simultaneously
2. Test error handling (disconnect daemon, invalid commands)
3. Test parameter controls (if using plugin devices)
4. Build production app: `npm run tauri build`

## Build for Production

```bash
# Create optimized build
npm run tauri build

# Output location:
# macOS: src-tauri/target/release/bundle/macos/rust-daq-gui.app
# Linux: src-tauri/target/release/bundle/appimage/rust-daq-gui.AppImage
# Windows: src-tauri/target/release/bundle/msi/rust-daq-gui.msi
```

## Known Limitations (Phase 1)

- No frame streaming visualization (cameras return metadata only)
- No real-time graphing of parameter changes
- No scan execution UI
- No preset management UI

These will be addressed in Phase 2 (bd-rbqk.2+).
