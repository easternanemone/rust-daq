#!/bin/bash
# Close dead architecture issues with proper explanations
# Part of Beads Cleanup Plan - Phase 4
# DESTRUCTIVE: Closes issues (but they remain in database)

set -e
cd "$(git rev-parse --show-toplevel)"

echo "=== Closing Dead Architecture Issues ==="
echo "⚠️  This will close issues. Press Ctrl+C to abort, or Enter to continue..."
read -r

# Backup first
BACKUP_DIR=".beads-backup-$(date +%Y%m%d-%H%M%S)"
echo "Creating backup at $BACKUP_DIR..."
cp -r .beads "$BACKUP_DIR"
echo "✓ Backup created"
echo

# Close V4 Issues
echo "Closing V4 (Kameo) architecture issues..."

echo "  Closing bd-9uko (V4 production deployment)..."
bd close bd-9uko 2>/dev/null || echo "    Already closed or doesn't exist"

echo "  Closing bd-53tr (V4 stability testing)..."
bd close bd-53tr 2>/dev/null || echo "    Already closed or doesn't exist"

echo "  Closing bd-zozl (V4 performance validation)..."
bd close bd-zozl 2>/dev/null || echo "    Already closed or doesn't exist"

echo "  Closing bd-nc7d (V4 Hardware Validation)..."
bd close bd-nc7d 2>/dev/null || echo "    Already closed or doesn't exist"

echo "  Closing bd-vtjc (V4 Production Deployment)..."
bd close bd-vtjc 2>/dev/null || echo "    Already closed or doesn't exist"

echo "  Closing bd-r896 (Kameo vs V3 performance)..."
bd close bd-r896 2>/dev/null || echo "    Already closed or doesn't exist"

echo "  Closing bd-o6c7 (Migrate V4 SCPI Actor)..."
bd close bd-o6c7 2>/dev/null || echo "    Already closed or doesn't exist"

echo "  Closing bd-ca6e (Eliminate Kameo Dependencies)..."
bd close bd-ca6e 2>/dev/null || echo "    Already closed or doesn't exist"

# Add comments explaining closure
echo
echo "Adding closure comments to V4 issues..."
cat <<'EOF' > /tmp/v4_closure_comment.txt
CLOSED: V4 Kameo architecture was abandoned in favor of V5 headless architecture.

V5 uses direct async + capability traits instead of the actor model.
See architectural analysis (2025-11-19) for details.

Related files deleted:
- v4-daq/ directory
- All Kameo dependencies

This issue is preserved for historical reference.
EOF

for issue in bd-9uko bd-53tr bd-zozl bd-nc7d bd-vtjc bd-r896 bd-o6c7 bd-ca6e; do
  echo "  Adding comment to $issue..."
  bd comment add "$issue" "$(cat /tmp/v4_closure_comment.txt)" 2>/dev/null || true
done
rm /tmp/v4_closure_comment.txt

# Close obsolete GUI issues
echo
echo "Closing obsolete GUI issues (V5 is headless)..."

echo "  Closing bd-d647.4 (GUI metrics refresh)..."
bd close bd-d647.4 2>/dev/null || echo "    Already closed or doesn't exist"

cat <<'EOF' > /tmp/gui_closure_comment.txt
DEFERRED: V5 architecture is headless-first.
GUI functionality moved to separate React/Python application.

The GUI should connect to V5's remote API rather than
being embedded in the rust-daq binary.

This issue may be repurposed for the external GUI app.
EOF

bd comment add "bd-d647.4" "$(cat /tmp/gui_closure_comment.txt)" 2>/dev/null || true
rm /tmp/gui_closure_comment.txt

# V1/V2 Deletion Tasks - Mark as completed
echo
echo "Closing V1/V2 deletion tasks (verify manually first!)..."
echo "⚠️  Run ./01-verify-deletions.sh first to confirm files are deleted"
echo

# These should only be closed if verification passed
# Uncomment after running verification script:

# bd close bd-q98c 2>/dev/null  # Delete V2 App Actor
# bd close bd-ogit 2>/dev/null  # Delete V1 Core Traits
# bd close bd-pe1y 2>/dev/null  # Delete V2 Registry
# bd close bd-1dpo 2>/dev/null  # Remove V1/V2 Tests

echo "  → V1/V2 deletion tasks NOT closed yet"
echo "  → Run 01-verify-deletions.sh, then uncomment in this script"

echo
echo "=== Issue Closure Complete ==="
echo "Backup saved at: $BACKUP_DIR"
echo "Run 'bd list --status closed --label arch:v4-dead' to verify"
