#!/bin/bash
# Run database compaction and maintenance
# Part of Beads Cleanup Plan - Phase 5
# DESTRUCTIVE: Compacts closed issues (permanent)

set -e
cd "$(git rev-parse --show-toplevel)"

echo "=== Beads Database Compaction & Maintenance ==="
echo

# Backup first
BACKUP_DIR=".beads-backup-compaction-$(date +%Y%m%d-%H%M%S)"
echo "Creating backup at $BACKUP_DIR..."
cp -r .beads "$BACKUP_DIR"
echo "✓ Backup created"
echo

# Show current stats
echo "Current database statistics:"
bd compact --stats
echo

# Preview compaction
echo "Previewing Tier 1 compaction (30+ days closed)..."
bd compact --dry-run --tier 1
echo

echo "⚠️  Compaction is PERMANENT. Closed issues will be summarized."
echo "Press Ctrl+C to abort, or Enter to continue..."
read -r

# Run Tier 1 compaction
echo
echo "Running Tier 1 compaction..."
bd compact --all --tier 1

echo
echo "Compaction complete. New stats:"
bd compact --stats

# Vacuum SQLite database
echo
echo "Vacuuming SQLite database..."
DB_FILE=$(find .beads -name "*.db" -type f | head -1)
if [ -n "$DB_FILE" ]; then
  echo "  Database: $DB_FILE"
  SIZE_BEFORE=$(du -h "$DB_FILE" | cut -f1)
  echo "  Size before: $SIZE_BEFORE"

  sqlite3 "$DB_FILE" "VACUUM;"

  SIZE_AFTER=$(du -h "$DB_FILE" | cut -f1)
  echo "  Size after: $SIZE_AFTER"
  echo "  ✓ Vacuum complete"
else
  echo "  ⚠ No database file found"
fi

# Integrity check
echo
echo "Running integrity check..."
if [ -n "$DB_FILE" ]; then
  INTEGRITY=$(sqlite3 "$DB_FILE" "PRAGMA integrity_check;")
  if [ "$INTEGRITY" == "ok" ]; then
    echo "  ✓ Database integrity: OK"
  else
    echo "  ✗ Database integrity check failed:"
    echo "$INTEGRITY"
  fi
fi

echo
echo "=== Compaction Complete ==="
echo "Backup saved at: $BACKUP_DIR"
echo
echo "Next steps:"
echo "1. Verify database health: bd list | head -20"
echo "2. Check compacted issues: bd list --all --status closed | grep compacted"
echo "3. If issues found, restore from: $BACKUP_DIR"
