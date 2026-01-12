#!/bin/bash
# Generates beads project context for handoff documents
# Outputs markdown-formatted summary of ready work, blocked items, and project state
# Usage: .beads/scripts/beads-context-for-handoff.sh >> whats-next.md

set -e

# Check prerequisites
if [ ! -d ".beads" ]; then
  echo "# Beads Context"
  echo ""
  echo "No .beads directory found - not a beads-tracked project."
  exit 0
fi

if ! command -v bd &> /dev/null; then
  echo "# Beads Context"
  echo ""
  echo "bd command not found - install beads CLI."
  exit 0
fi

# Get stats
STATS=$(bd --allow-stale stats --json 2>/dev/null || echo '{}')

TOTAL=$(echo "$STATS" | jq -r '.summary.total_issues // 0' 2>/dev/null || echo 0)
OPEN=$(echo "$STATS" | jq -r '.summary.open_issues // 0' 2>/dev/null || echo 0)
READY=$(echo "$STATS" | jq -r '.summary.ready_issues // 0' 2>/dev/null || echo 0)
BLOCKED=$(echo "$STATS" | jq -r '.summary.blocked_issues // 0' 2>/dev/null || echo 0)
IN_PROGRESS=$(echo "$STATS" | jq -r '.summary.in_progress_issues // 0' 2>/dev/null || echo 0)
TOMBSTONES=$(echo "$STATS" | jq -r '.summary.tombstone_issues // 0' 2>/dev/null || echo 0)

echo ""
echo "<beads_project_context>"
echo ""
echo "## Beads Project State"
echo ""
echo "| Metric | Count |"
echo "|--------|-------|"
echo "| Open issues | $OPEN |"
echo "| Ready to work | $READY |"
echo "| In progress | $IN_PROGRESS |"
echo "| Blocked | $BLOCKED |"
echo "| Tombstones | $TOMBSTONES |"
echo ""

# Show in-progress items (current work)
if [ "$IN_PROGRESS" -gt 0 ]; then
  echo "## Currently In Progress"
  echo ""
  echo "\`\`\`"
  bd --allow-stale list --status=in_progress 2>/dev/null || echo "Unable to list in-progress items"
  echo "\`\`\`"
  echo ""
fi

# Show ready items (next work)
if [ "$READY" -gt 0 ]; then
  echo "## Ready to Work (No Blockers)"
  echo ""
  echo "Run \`bd ready\` for full details. Top items:"
  echo ""
  echo "\`\`\`"
  bd --allow-stale ready 2>/dev/null | head -15 || echo "Unable to list ready items"
  echo "\`\`\`"
  echo ""
fi

# Show blocked items
if [ "$BLOCKED" -gt 0 ]; then
  echo "## Blocked Items"
  echo ""
  echo "\`\`\`"
  bd --allow-stale blocked 2>/dev/null | head -10 || echo "Unable to list blocked items"
  echo "\`\`\`"
  echo ""
fi

# Maintenance warning if needed
if [ "$TOMBSTONES" -gt 50 ]; then
  echo "## Maintenance Recommended"
  echo ""
  echo "Tombstone count ($TOMBSTONES) exceeds threshold (50)."
  echo ""
  echo "\`\`\`bash"
  echo "bd admin cleanup --older-than 30 --force"
  echo "bd admin compact --prune"
  echo "\`\`\`"
  echo ""
fi

echo "</beads_project_context>"
