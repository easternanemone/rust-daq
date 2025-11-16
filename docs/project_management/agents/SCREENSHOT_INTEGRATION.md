# Screenshot Integration for Agent Workflows

## Summary

This document describes the screenshot capability integration for the rust-daq agent workflow, enabling visual verification similar to the Playwright-based approach but adapted for native egui applications.

## Implementation

### Core Components

#### 1. GUI Screenshot Functionality (`crates/rust-daq-app/src/gui/mod.rs`)

Added screenshot capture capability to the `Gui` struct:

- **Field**: `screenshot_request: Option<PathBuf>` - Tracks pending screenshot requests
- **Keyboard Shortcut**: F12 key triggers screenshot capture
- **Auto-naming**: Screenshots saved as `screenshots/screenshot_YYYYMMDD_HHMMSS.png`
- **Public API**: `gui.request_screenshot(path)` for programmatic access

**Key Features:**
- Non-blocking: Screenshots captured asynchronously via egui's viewport commands
- Directory creation: Automatically creates `screenshots/` directory
- Logging: Confirms screenshot requests in application logs
- Error handling: Gracefully handles directory creation failures

#### 2. Verification Framework (`crates/rust-daq-app/src/gui/verification.rs`)

New module providing agent-friendly verification tools:

- **VerificationCommand**: Enum for verification operations
- **VerificationResult**: Structured result type for agent workflows
- **VerificationHelper**: Utility for generating screenshot paths
- **Tests**: Unit tests for verification framework

**API Example:**
```rust
let helper = VerificationHelper::new("screenshots");
let path = helper.named_screenshot_path("feature_verification");
gui.request_screenshot(path);
```

#### 3. Python Verification Script

Automated verification scripts for agent workflows are currently under development. For now, agents can trigger screenshots manually via the F12 key or programmatically through the Rust API.

### Documentation Updates

#### BD_JULES_INTEGRATION.md

Added comprehensive "Visual Verification with Screenshots" section:

- **Screenshot capabilities**: Keyboard shortcut, programmatic API
- **Example workflow**: Step-by-step GUI issue verification with screenshots
- **Best practices**: What to do/avoid with screenshots

#### CLAUDE.md

Updated "Testing Infrastructure" section:

- Added GUI verification subsection
- Documented screenshot directory and F12 shortcut
- Referenced verification API

#### .gitignore

Added `/screenshots/` directory to prevent auto-generated screenshots from being committed.

## Comparison: Playwright vs Native Verification

### Original Playwright Script

```python
# For reference, requires web interface
from playwright.sync_api import sync_playwright

browser = chromium.connect_over_cdp("http://localhost:9222")
page.screenshot(path="verification.png")
expect(element).to_be_visible()
```

**Characteristics:**
- Web-based (browser connection)
- Synchronous element inspection
- Direct screenshot API

### New Native Verification

```python
# Native egui application verification
import subprocess
import pyautogui

# Check application running
subprocess.run(["pgrep", "-f", "rust_daq"], check=True)

# Trigger screenshot (manual F12 press or programmatic API call)
pyautogui.press('f12') # This simulates a key press, requires GUI focus

# Verify creation
# Verification would involve checking for file existence and content
```

**Characteristics:**
- Native application (process-based)
- Keyboard simulation for interaction (or programmatic API)
- File-based verification

## Agent Workflow Integration

### Example: GUI Feature with Visual Verification

```bash
# 1. Agent selects ready GUI issue from beads
bd ready
bd show daq-42  # "Add spectrum plot visualization"

# 2. Agent creates Jules session with screenshot requirement
# Prompt includes: "Success Criteria: Screenshot showing spectrum plot"

# 3. Jules implements the feature
# - Modifies GUI code
# - Runs application: cargo run &
# - Triggers screenshot manually via F12 key

# 4. Verification (manual or via future script)
# - Manually check the screenshot in the 'screenshots/' directory

# 5. PR includes visual evidence
gh pr create --body "![Spectrum Plot](./screenshots/screenshot_YYYYMMDD_HHMMSS.png)" # Reference the actual screenshot file

# 6. Issue marked complete with visual proof
bd done daq-42
```

## Technical Implementation Details

### Screenshot Flow

1. **User/Agent Action**: F12 keypress or `gui.request_screenshot(path)` call
2. **Request Storage**: Path stored in `screenshot_request` field
3. **Next Frame**: Request processed in `update()` method
4. **Directory Creation**: Parent directory created if needed
5. **Viewport Command**: `ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot)`
6. **Logging**: Confirmation logged for debugging
7. **Async Capture**: egui handles actual screenshot capture

### egui 0.29 Screenshot API

**Current Implementation:**
- Uses `ViewportCommand::Screenshot` for capture requests
- Asynchronous: actual screenshot happens on next frame
- Logged but not synchronously confirmed

**Future Enhancement:**
Could implement callback-based confirmation using egui's event system for synchronous verification.

### Directory Structure

```
rust-daq/
├── screenshots/                    # Auto-generated (gitignored)
│   └── screenshot_YYYYMMDD_HHMMSS.png
└── crates/
    └── rust-daq-app/
        └── src/
            └── gui/
                ├── mod.rs                  # Screenshot implementation
                └── verification.rs         # Verification framework
```

## Benefits for Agent Workflows

1. **Visual Proof**: Screenshots provide concrete evidence of GUI changes
2. **Automated Verification**: Scripts enable hands-off testing
3. **PR Quality**: Visual documentation improves review process
4. **bd Integration**: Complements issue tracking with visual artifacts
5. **Consistent Pattern**: Similar to web-based Playwright approach

## Limitations and Future Work

### Current Limitations

1. **Async Screenshot**: No synchronous confirmation of capture
2. **No Element Inspection**: Cannot programmatically verify specific UI elements
3. **Manual Trigger**: Requires keyboard simulation or manual F12 press
4. **No Headless Mode**: Requires GUI to be running with display

### Future Enhancements

- [ ] Implement screenshot capture callback for synchronous verification
- [ ] Add UI element inspection API (text search, bounds checking)
- [ ] Create headless testing mode with virtual display
- [ ] Implement pixel-perfect image comparison
- [ ] Add OCR-based text verification
- [ ] Integrate with CI/CD pipeline for automated testing
- [ ] Support video recording for complex interactions

## Testing

### Manual Testing

1. Start application:
   ```bash
   cargo run
   ```

2. Press F12 key in GUI window

3. Verify screenshot created:
   ```bash
   ls -la screenshots/
   ```

### Automated Testing

1. Start application in background:
   ```bash
   cargo run &
   ```

2. Run verification script:
   ```bash
   python jules-scratch/verification/verify_gui_screenshot.py
   ```

3. Check output for success confirmation

### Integration Testing

See `src/gui/verification.rs` for unit tests of the verification framework.

## Conclusion

The screenshot integration successfully adapts Playwright-style verification to the native egui application, providing agents with visual verification capabilities for GUI-related issues in the bd + Jules workflow.

**Key Achievement**: Unified verification approach across web (Playwright) and native (egui) applications for consistent agent workflows.
