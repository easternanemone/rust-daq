#!/bin/bash
# Jules setup script for rust-daq
# This runs automatically when Jules starts a new session

set -e

echo "🦀 Setting up rust-daq development environment..."

# Verify Rust toolchain
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust not found. Please install: https://rustup.rs"
    exit 1
fi

# Check Rust version (need 1.70+ for some features)
RUST_VERSION=$(rustc --version | cut -d' ' -f2 | cut -d'.' -f1-2)
echo "✓ Rust toolchain: $(rustc --version)"

# Install required components if missing
if ! rustup component list | grep -q "rustfmt-.*installed"; then
    echo "Installing rustfmt..."
    rustup component add rustfmt
fi

if ! rustup component list | grep -q "clippy-.*installed"; then
    echo "Installing clippy..."
    rustup component add clippy
fi

# Check for protobuf compiler (required for networking feature)
if ! command -v protoc &> /dev/null; then
    echo "⚠️  protoc not found - networking feature may fail to build"
    echo "   Install: brew install protobuf (macOS) or apt-get install protobuf-compiler (Linux)"
else
    echo "✓ protoc: $(protoc --version)"
fi

# Quick build check (use cached artifacts if available)
echo "Building rust-daq with networking feature..."
if cargo build --lib --features networking 2>&1 | tail -5; then
    echo "✓ Build successful"
else
    echo "❌ Build failed - see output above"
    exit 1
fi

# Run quick sanity test
echo "Running sanity tests..."
if cargo test --lib --features networking --quiet 2>&1 | tail -3; then
    echo "✓ Tests passing"
else
    echo "⚠️  Some tests failed"
fi

echo ""
echo "✅ rust-daq environment ready!"
echo ""
echo "📚 Architecture: V5 Headless-First (see .jules/agents.md)"
echo "🚀 Phase 3 Complete: Network Layer with gRPC + CLI client"
echo "🔜 Phase 4 Next: Arrow batching (PR #104)"
echo ""
