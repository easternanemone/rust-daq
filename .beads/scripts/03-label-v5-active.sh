#!/bin/bash
# Label active V5 architecture issues
# Part of Beads Cleanup Plan - Phase 3
# SAFE: Only adds labels
# Compatible with bash 3.x (macOS default)

set -e
cd "$(git rev-parse --show-toplevel)"

echo "=== Labeling Active V5 Architecture Issues ==="
echo

# V5 Core Architecture - Critical Features
echo "Labeling V5 core architecture features..."

bd label add "bd-yb5e" arch:v5-active 2>/dev/null || true
bd label add "bd-yb5e" priority:critical 2>/dev/null || true
echo "  ✓ bd-yb5e: KF4: Capability-Based Hardware Traits"

bd label add "bd-x9tp" arch:v5-active 2>/dev/null || true
bd label add "bd-x9tp" priority:critical 2>/dev/null || true
echo "  ✓ bd-x9tp: KF1: Headless-First Remote Architecture"

bd label add "bd-hqy6" arch:v5-active 2>/dev/null || true
bd label add "bd-hqy6" priority:critical 2>/dev/null || true
bd label add "bd-hqy6" component:scripting 2>/dev/null || true
echo "  ✓ bd-hqy6: P4.1: Define ScriptEngine Trait"

bd label add "bd-oq51" arch:v5-active 2>/dev/null || true
bd label add "bd-oq51" priority:critical 2>/dev/null || true
echo "  ✓ bd-oq51: HEADLESS-FIRST & SCRIPTABLE ARCHITECTURE"

# V5 Driver Migration
echo
echo "Labeling V5 driver migration tasks..."

bd label add "bd-l7vs" arch:v5-migration 2>/dev/null || true
bd label add "bd-l7vs" component:driver 2>/dev/null || true
echo "  ✓ bd-l7vs: Migrate MaiTai and Newport to V3 Traits"

bd label add "bd-e18h" arch:v5-migration 2>/dev/null || true
bd label add "bd-e18h" component:driver 2>/dev/null || true
echo "  ✓ bd-e18h: Fix PVCAM V3 Camera Trait"

bd label add "bd-95pj" arch:v5-migration 2>/dev/null || true
bd label add "bd-95pj" component:driver 2>/dev/null || true
echo "  ✓ bd-95pj: Migrate ESP300 to V3 MotionController"

bd label add "bd-rxur" arch:v5-migration 2>/dev/null || true
bd label add "bd-rxur" component:driver 2>/dev/null || true
bd label add "bd-rxur" priority:safety 2>/dev/null || true
echo "  ✓ bd-rxur: Migrate to serial2-tokio (SAFETY)"

# V5 Cleanup Tasks
echo
echo "Labeling V5 cleanup tasks..."

bd label add "bd-ifxt" arch:v5-cleanup 2>/dev/null || true
bd label add "bd-ifxt" priority:cleanup 2>/dev/null || true
echo "  ✓ bd-ifxt: Fix V3 Import Consolidation"

bd label add "bd-9cz0" arch:v5-cleanup 2>/dev/null || true
bd label add "bd-9cz0" priority:cleanup 2>/dev/null || true
echo "  ✓ bd-9cz0: Fix Trait Signature Mismatches"

bd label add "bd-op7v" arch:v5-cleanup 2>/dev/null || true
bd label add "bd-op7v" priority:cleanup 2>/dev/null || true
echo "  ✓ bd-op7v: Standardize on core_v3::Measurement Enum"

# V5 Scripting Layer
echo
echo "Labeling scripting layer issues..."

bd label add "bd-dxqi" arch:v5-active 2>/dev/null || true
bd label add "bd-dxqi" component:scripting 2>/dev/null || true
echo "  ✓ bd-dxqi: Expose V3 APIs to Python via PyO3"

# Repurpose Remote API issue for V5
echo
echo "Relabeling remote API for V5..."
bd label add "bd-136" arch:v5-active 2>/dev/null || true
bd label add "bd-136" component:api 2>/dev/null || true
bd label add "bd-136" priority:critical 2>/dev/null || true
echo "  ✓ bd-136: Remote API for V5 architecture"

echo
echo "=== Labeling Complete ==="
echo "Run 'bd list --label arch:v5-active' to see current V5 work"
echo "Run 'bd list --label priority:critical' to see blocking issues"
