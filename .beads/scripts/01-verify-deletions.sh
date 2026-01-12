#!/bin/bash
# Verify that V1/V2 files have been deleted before closing deletion issues
# Part of Beads Cleanup Plan - Phase 1

set -e
cd "$(git rev-parse --show-toplevel)"

echo "=== Verifying V1/V2/V4 File Deletions ==="
echo

# V2 App Actor files (bd-q98c)
echo "Checking V2 App Actor deletion..."
if [ ! -f "src/app_actor.rs" ]; then
  echo "  ✓ src/app_actor.rs deleted"
  echo "  → Can close bd-q98c"
else
  echo "  ✗ src/app_actor.rs still exists"
  echo "  → bd-q98c should remain open"
fi

# V1 Core Trait files (bd-ogit)
echo
echo "Checking V1 Core Trait deletion..."
if [ ! -f "src/core.rs" ]; then
  echo "  ✓ src/core.rs deleted"
  echo "  → Can close bd-ogit"
else
  echo "  ✗ src/core.rs still exists"
  echo "  → bd-ogit should remain open"
fi

# V2 Registry Layer (bd-pe1y)
echo
echo "Checking V2 Registry deletion..."
FILES_EXIST=0
for file in "src/instrument/registry.rs" "src/instrument/adapter.rs"; do
  if [ -f "$file" ]; then
    echo "  ✗ $file still exists"
    FILES_EXIST=1
  fi
done
if [ $FILES_EXIST -eq 0 ]; then
  echo "  ✓ V2 Registry deleted"
  echo "  → Can close bd-pe1y"
else
  echo "  → bd-pe1y should remain open"
fi

# V4 directory (bd-ca6e)
echo
echo "Checking V4 Kameo directory..."
if [ ! -d "v4-daq" ]; then
  echo "  ✓ v4-daq/ directory deleted"
  echo "  → Can close bd-ca6e (Kameo dependencies eliminated)"
else
  echo "  ✗ v4-daq/ directory still exists"
  echo "  → bd-ca6e should remain open"
fi

# V1/V2 Test files (bd-1dpo)
echo
echo "Checking V1/V2 test files..."
V1V2_TESTS=$(find . -name "*_v[12]_test.rs" -o -name "*_actor_test.rs" 2>/dev/null | wc -l)
if [ "$V1V2_TESTS" -eq 0 ]; then
  echo "  ✓ No V1/V2 test files found"
  echo "  → Can close bd-1dpo"
else
  echo "  ✗ Found $V1V2_TESTS V1/V2 test files"
  echo "  → bd-1dpo should remain open"
fi

# GUI directory (for reference)
echo
echo "Checking GUI directory status..."
if [ -d "src/gui" ]; then
  echo "  ⚠ src/gui/ still exists (should be deleted in V5)"
  echo "  → GUI is obsolete in headless V5 architecture"
else
  echo "  ✓ src/gui/ deleted"
fi

echo
echo "=== Verification Complete ==="
