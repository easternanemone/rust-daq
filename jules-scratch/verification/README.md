# GUI Verification Scripts for Agent Workflows

This directory contains verification scripts for automated GUI testing in agent workflows (bd + Jules integration).

## Overview

These scripts provide Playwright-style verification capabilities for the native rust-daq egui application, enabling agents to:

- **Verify GUI changes** are implemented correctly
- **Capture screenshots** for visual proof
- **Automate testing** as part of the agent workflow
- **Provide PR evidence** with visual verification

## Available Scripts

### `verify_gui_screenshot.py`

Main verification script for GUI screenshot capture and validation.

**Usage:**
```bash
# Ensure rust-daq is running
cargo run &

# Run verification
python jules-scratch/verification/verify_gui_screenshot.py
```

**What it does:**
1. Checks if rust-daq application is running
2. Triggers screenshot via F12 keyboard shortcut
3. Verifies screenshot was created in `screenshots/` directory
4. Copies to `jules-scratch/verification/verification.png` for consistency

**Requirements:**
```bash
pip install pyautogui
```

### `verify_consolidation_toggle.py`

Original Playwright-based verification (for reference, requires web interface).

## Integration with Agent Workflows

### bd + Jules Workflow with Screenshots

```bash
# 1. Check ready work
bd ready

# 2. Pick GUI-related issue
bd show daq-42  # "Add power meter display panel"

# 3. Create Jules session with visual verification requirement
# Include in prompt: "Screenshot required showing power meter panel"

# 4. Jules implements feature and runs application
cargo run &

# 5. Trigger screenshot manually (F12) or via script
python jules-scratch/verification/verify_gui_screenshot.py

# 6. Attach screenshot to PR
gh pr create --title "[daq-42] Add power meter display panel" \
  --body "## Visual Verification\n\n![Power Meter Panel](./jules-scratch/verification/verification.png)"

# 7. Mark issue complete
bd done daq-42
```

## Screenshot Locations

- **Auto-generated**: `screenshots/screenshot_YYYYMMDD_HHMMSS.png`
  - Timestamped screenshots from F12 key
  - Not committed to git (in .gitignore)
  - Useful for manual testing

- **Verification**: `jules-scratch/verification/verification.png`
  - Standardized location for agent verification
  - Copied by verification scripts
  - Can be committed for PR evidence

## Keyboard Shortcuts

The rust-daq GUI includes these shortcuts for testing:

- **F12**: Capture screenshot
  - Saves to `screenshots/` with timestamp
  - Creates directory if needed
  - Displays confirmation in logs

## Programmatic API

For integration tests and automated workflows:

```rust
use rust_daq::gui::Gui;

// In your test or automation code
gui.request_screenshot("tests/output/test_screenshot.png");
```

See `src/gui/verification.rs` for the full verification framework API.

## Comparison: Web vs Native Verification

### Web (Playwright)
```python
from playwright.sync_api import sync_playwright

# Connect to browser
browser = playwright.chromium.connect_over_cdp("http://localhost:9222")
page = browser.contexts[0].pages()[0]

# Verify element
expect(page.get_by_text("Power Meter")).to_be_visible()

# Screenshot
page.screenshot(path="verification.png")
```

### Native (rust-daq)
```python
import subprocess
import pyautogui

# Check application is running
subprocess.run(["pgrep", "-f", "rust_daq"], check=True)

# Trigger screenshot
pyautogui.press('f12')

# Verify screenshot exists
assert Path("screenshots/screenshot_*.png").exists()
```

## Troubleshooting

### "Application not running"
```bash
# Start the application in background
cargo run &

# Verify it's running
pgrep -f rust_daq
```

### "pyautogui not found"
```bash
pip install pyautogui

# On Linux, may also need:
sudo apt-get install python3-tk python3-dev
```

### "Screenshot not captured"
1. Ensure GUI window has focus
2. Try manual F12 key press
3. Check `screenshots/` directory was created
4. Check application logs for errors

### "Permission denied on screenshots directory"
```bash
# Create directory with correct permissions
mkdir -p screenshots
chmod 755 screenshots
```

## Best Practices

✅ **DO**:
- Run verification script after GUI changes
- Include screenshots in PR descriptions
- Use descriptive commit messages: "Add power meter panel (screenshot)"
- Verify critical UI elements are visible
- Test keyboard shortcuts work before committing

❌ **DON'T**:
- Commit auto-generated timestamped screenshots
- Skip visual verification for GUI issues
- Rely solely on screenshots (add tests too)
- Use screenshots as primary documentation

## Future Enhancements

Potential improvements to the verification framework:

- [ ] Headless screenshot capture (without X11/Wayland)
- [ ] Pixel-perfect image comparison
- [ ] OCR text verification for UI elements
- [ ] Automated UI element detection
- [ ] Integration with CI/CD pipeline
- [ ] Video recording for complex interactions

## Related Documentation

- [BD_JULES_INTEGRATION.md](../../BD_JULES_INTEGRATION.md) - Full agent workflow
- [CLAUDE.md](../../CLAUDE.md) - Testing infrastructure overview
- [src/gui/verification.rs](../../src/gui/verification.rs) - Rust verification API
