#!/usr/bin/env python3
"""
GUI Screenshot Verification Script for Rust DAQ

This script provides automated GUI verification for agent workflows,
similar to verify_consolidation_toggle.py but adapted for the native egui application.

Since rust-daq is a native application (not web-based), this script:
1. Checks if the application is running
2. Triggers screenshot via keyboard shortcut simulation (F12)
3. Verifies screenshot was created
4. Can be extended to check specific UI elements

Usage:
    python jules-scratch/verification/verify_gui_screenshot.py

Requirements:
    - rust-daq application must be running
    - pyautogui for keyboard simulation (pip install pyautogui)

Integration with bd + Jules workflow:
    1. Agent starts rust-daq application
    2. Agent runs this script to verify UI changes
    3. Screenshot is saved for PR attachment
    4. Agent marks bd issue as complete with visual proof
"""

import os
import sys
import time
import subprocess
from pathlib import Path

def check_application_running():
    """Check if rust-daq application is running"""
    try:
        # Check for rust-daq process
        result = subprocess.run(
            ["pgrep", "-f", "rust_daq"],
            capture_output=True,
            text=True
        )
        return result.returncode == 0
    except Exception as e:
        print(f"Warning: Could not check if application is running: {e}")
        return False

def trigger_screenshot():
    """Trigger screenshot via keyboard shortcut (F12)"""
    try:
        import pyautogui
        print("Triggering screenshot with F12...")
        pyautogui.press('f12')
        time.sleep(1)  # Wait for screenshot to be captured
        return True
    except ImportError:
        print("pyautogui not available - cannot trigger screenshot automatically")
        print("Please install: pip install pyautogui")
        return False
    except Exception as e:
        print(f"Error triggering screenshot: {e}")
        return False

def verify_screenshot_exists(screenshot_dir="screenshots"):
    """Verify that a screenshot was created"""
    screenshot_path = Path(screenshot_dir)

    if not screenshot_path.exists():
        print(f"Screenshot directory does not exist: {screenshot_path}")
        return False

    # Find most recent screenshot
    screenshots = list(screenshot_path.glob("screenshot_*.png"))
    if not screenshots:
        print("No screenshots found")
        return False

    latest = max(screenshots, key=lambda p: p.stat().st_mtime)
    age = time.time() - latest.stat().st_mtime

    if age < 5:  # Screenshot created within last 5 seconds
        print(f"✓ Screenshot verified: {latest}")
        # Copy to verification directory for consistency with Playwright script
        verification_path = Path("jules-scratch/verification/verification.png")
        verification_path.parent.mkdir(parents=True, exist_ok=True)
        import shutil
        shutil.copy(latest, verification_path)
        print(f"✓ Copied to: {verification_path}")
        return True
    else:
        print(f"Latest screenshot is {age:.0f}s old - may not be from this run")
        return False

def main():
    """Main verification workflow"""
    print("=== Rust DAQ GUI Verification ===\n")

    # Check if application is running
    if not check_application_running():
        print("⚠️  rust-daq application does not appear to be running")
        print("Please start the application with: cargo run")
        return 1

    print("✓ Application is running\n")

    # Trigger screenshot
    if not trigger_screenshot():
        print("\n⚠️  Could not trigger screenshot automatically")
        print("Manual alternative: Press F12 in the rust-daq window")
        return 1

    # Verify screenshot was created
    if not verify_screenshot_exists():
        print("\n❌ Screenshot verification failed")
        return 1

    print("\n✓ All verifications passed!")
    return 0

if __name__ == "__main__":
    sys.exit(main())
