#!/bin/bash
# Regular maintenance script for beads database
# Run weekly/monthly to keep database healthy

set -e
cd "$(git rev-parse --show-toplevel)"

echo "=== Beads Database Maintenance ==="
echo "Run: $(date)"
echo

# Check for unlabeled issues
echo "Checking for unlabeled issues..."
UNLABELED=$(bd list --all --json | jq -r '.[] | select(.labels | length == 0) | .id' | wc -l)
echo "  Found $UNLABELED issues without labels"

if [ "$UNLABELED" -gt 0 ]; then
  echo "  Top 10 unlabeled issues:"
  bd list --all --json | jq -r '.[] | select(.labels | length == 0) | "\(.id)|\(.status)|\(.title)"' | head -10
  echo
  echo "  → Consider adding labels with component:*, arch:*, priority:*"
fi

# Check for stale in_progress issues
echo
echo "Checking for stale in_progress issues..."
STALE=$(bd list --status in_progress --json | jq -r 'length')
echo "  Found $STALE in_progress issues"

if [ "$STALE" -gt 0 ]; then
  echo "  In-progress issues:"
  bd list --status in_progress --json | jq -r '.[] | "\(.id)|\(.title)"'
  echo
  echo "  → Review if these are actually being worked on"
fi

# Check for blocked issues without dependencies
echo
echo "Checking blocked issues..."
BLOCKED=$(bd list --status blocked --json | jq -r 'length')
echo "  Found $BLOCKED blocked issues"

if [ "$BLOCKED" -gt 0 ]; then
  echo "  Blocked issues:"
  bd list --status blocked --json | jq -r '.[] | "\(.id)|\(.title)"'
  echo
  echo "  → Ensure each has a dependency or blocking reason"
fi

# Check compaction opportunities
echo
echo "Checking compaction opportunities..."
bd compact --stats

# Priority distribution
echo
echo "Priority distribution:"
bd list --all --json | jq -r '.[] | .priority' | sort | uniq -c | sort -rn

# Status distribution
echo
echo "Status distribution:"
bd list --all --json | jq -r '.[] | .status' | sort | uniq -c | sort -rn

# Label coverage
echo
echo "Label coverage:"
TOTAL=$(bd list --all --json | jq -r 'length')
LABELED=$(bd list --all --json | jq -r '.[] | select(.labels | length > 0) | .id' | wc -l)
COVERAGE=$(echo "scale=1; $LABELED * 100 / $TOTAL" | bc)
echo "  $LABELED of $TOTAL issues labeled ($COVERAGE%)"

# Database size
echo
echo "Database size:"
du -h .beads/*.db .beads/*.jsonl 2>/dev/null | sed 's/^/  /'

echo
echo "=== Maintenance Check Complete ==="
echo
echo "Recommended actions:"
echo "1. Label unlabeled issues: ./03-label-v5-active.sh"
echo "2. Review stale in_progress: bd edit <issue-id>"
echo "3. Run compaction monthly: ./05-run-compaction.sh"
echo "4. Target label coverage: 80%+"
