#!/bin/bash
# Label all dead architecture issues for archival
# Part of Beads Cleanup Plan - Phase 2
# SAFE: Only adds labels, doesn't close issues

set -e
cd "$(git rev-parse --show-toplevel)"

echo "=== Labeling Dead Architecture Issues ==="
echo

# V4 Kameo Architecture Issues (DEAD)
echo "Labeling V4 (Kameo) architecture issues..."
V4_ISSUES=(
  "bd-9uko"   # V4 production deployment
  "bd-53tr"   # V4 stability testing
  "bd-zozl"   # V4 performance validation
  "bd-nc7d"   # V4 Hardware Validation
  "bd-vtjc"   # V4 Production Deployment
  "bd-r896"   # Kameo vs V3 performance
  "bd-o6c7"   # Migrate V4 SCPI Actor
  "bd-ca6e"   # Eliminate Kameo Dependencies
)

for issue in "${V4_ISSUES[@]}"; do
  echo "  Labeling $issue..."
  bd label add "$issue" arch:v4-dead 2>/dev/null || echo "    (already labeled or doesn't exist)"
  bd label add "$issue" status:wontfix 2>/dev/null || true
done

# V2 Actor Model Issues (DEAD)
echo
echo "Labeling V2 actor model issues..."
V2_ISSUES=(
  "bd-q98c"   # Delete V2 App Actor
  "bd-pe1y"   # Delete V2 Registry
)

for issue in "${V2_ISSUES[@]}"; do
  echo "  Labeling $issue..."
  bd label add "$issue" arch:v2-dead 2>/dev/null || echo "    (already labeled or doesn't exist)"
  bd label add "$issue" status:cleanup 2>/dev/null || true
done

# V1 Core Issues (DEAD)
echo
echo "Labeling V1 core issues..."
V1_ISSUES=(
  "bd-ogit"   # Delete V1 Core Traits
  "bd-1dpo"   # Remove V1/V2 Test Files
)

for issue in "${V1_ISSUES[@]}"; do
  echo "  Labeling $issue..."
  bd label add "$issue" arch:v1-dead 2>/dev/null || echo "    (already labeled or doesn't exist)"
  bd label add "$issue" status:cleanup 2>/dev/null || true
done

# Obsolete GUI Issues (V5 is headless)
echo
echo "Labeling obsolete GUI issues..."
GUI_ISSUES=(
  "bd-d647.4"   # GUI metrics refresh
  "bd-5q2g"     # Bridge V3 to GUI
  "bd-d617"     # Fix blocking GUI calls
  "bd-1471"     # Refactor GUI blocking_send
)

for issue in "${GUI_ISSUES[@]}"; do
  echo "  Labeling $issue..."
  bd label add "$issue" component:gui 2>/dev/null || echo "    (already labeled or doesn't exist)"
  bd label add "$issue" status:obsolete 2>/dev/null || true
  bd label add "$issue" arch:v5-headless 2>/dev/null || true
done

echo
echo "=== Labeling Complete ==="
echo "Run 'bd list --label arch:v4-dead' to verify V4 issues"
echo "Run 'bd list --label status:wontfix' to see all issues marked wontfix"
