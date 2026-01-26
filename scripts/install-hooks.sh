#!/usr/bin/env bash
# Install pre-commit hooks for rust-daq
#
# Usage:
#   bash scripts/install-hooks.sh [quick]
#
# Arguments:
#   quick - Install quick hooks (formatting only, faster for development)
#
# This script installs pre-commit hooks using the pre-commit framework.
# If pre-commit is not installed, it will attempt to install it via pip.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get repository root
REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

echo "Installing pre-commit hooks for rust-daq..."

# Check if pre-commit is installed
if ! command -v pre-commit &> /dev/null; then
    echo -e "${YELLOW}pre-commit not found. Attempting to install...${NC}"
    
    # Try pip install
    if command -v pip3 &> /dev/null; then
        pip3 install pre-commit
    elif command -v pip &> /dev/null; then
        pip install pre-commit
    else
        echo -e "${RED}Error: pip not found. Please install pre-commit manually:${NC}"
        echo "  pip install pre-commit"
        echo "  or: brew install pre-commit"
        exit 1
    fi
fi

# Verify pre-commit is now available
if ! command -v pre-commit &> /dev/null; then
    echo -e "${RED}Error: Failed to install pre-commit${NC}"
    exit 1
fi

echo -e "${GREEN}✓ pre-commit is installed${NC}"

# Determine which config to use
CONFIG_FILE=".pre-commit-config.yaml"
if [ "$1" = "quick" ]; then
    CONFIG_FILE=".pre-commit-quick.yaml"
    echo "Using quick hooks (formatting only)"
else
    echo "Using full hooks (formatting, linting, testing)"
    echo "Tip: Use 'bash scripts/install-hooks.sh quick' for faster hooks during development"
fi

# Install hooks
if [ "$1" = "quick" ]; then
    pre-commit install --config "$CONFIG_FILE"
else
    pre-commit install
fi

echo -e "${GREEN}✓ Pre-commit hooks installed successfully${NC}"
echo ""
echo "Hooks are now active and will run automatically on 'git commit'."
echo ""
echo "Useful commands:"
echo "  pre-commit run --all-files     # Run hooks on all files"
echo "  git commit --no-verify          # Skip hooks (emergency only)"
echo "  pre-commit uninstall            # Remove hooks"
echo ""
echo "To switch configurations:"
echo "  bash scripts/install-hooks.sh        # Full hooks"
echo "  bash scripts/install-hooks.sh quick  # Quick hooks"
