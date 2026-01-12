#!/bin/bash
# Beads maintenance check - runs at Claude Code session start
# Checks issue counts and reminds about cleanup if thresholds exceeded
# Output goes to session context automatically

set -e

# Check if .beads directory exists
if [ ! -d ".beads" ]; then
  exit 0  # Not a beads project, silently exit
fi

# Check if bd is available
if ! command -v bd &> /dev/null; then
  exit 0
fi

# Thresholds
THRESHOLD_CLOSED=50
THRESHOLD_TOMBSTONES=50

# Get stats (allow-stale to avoid blocking on sync issues)
STATS=$(bd --allow-stale stats --json 2>/dev/null || echo '{}')

CLOSED=$(echo "$STATS" | jq -r '.summary.closed_issues // 0' 2>/dev/null || echo 0)
TOMBSTONES=$(echo "$STATS" | jq -r '.summary.tombstone_issues // 0' 2>/dev/null || echo 0)
READY=$(echo "$STATS" | jq -r '.summary.ready_issues // 0' 2>/dev/null || echo 0)

NEEDS_CLEANUP=false
MESSAGES=""

if [ "$CLOSED" -gt "$THRESHOLD_CLOSED" ]; then
  NEEDS_CLEANUP=true
  MESSAGES="${MESSAGES}  - $CLOSED closed issues (threshold: $THRESHOLD_CLOSED)\n"
fi

if [ "$TOMBSTONES" -gt "$THRESHOLD_TOMBSTONES" ]; then
  NEEDS_CLEANUP=true
  MESSAGES="${MESSAGES}  - $TOMBSTONES tombstones (threshold: $THRESHOLD_TOMBSTONES)\n"
fi

if [ "$NEEDS_CLEANUP" = true ]; then
  printf "⚠️  Beads maintenance recommended:\n"
  printf "$MESSAGES"
  printf "\n  Run: bd admin cleanup --older-than 30 --force\n"
  printf "  Then: bd admin compact --prune\n"
fi

# Always show ready work count if any
if [ "$READY" -gt 0 ]; then
  printf "✓ Beads ready work items: $READY\n"
fi

exit 0
