# Getting Started Guide: Rust DAQ V4 Architecture

This guide provides instructions for setting up your development environment and getting started with the Rust DAQ application, which is currently undergoing a major refactoring to its V4 architecture.

## Prerequisites

### 1. Rust Toolchain (latest stable)
Ensure you have the latest stable Rust toolchain installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. Recommended Development Tools
```bash
cargo install cargo-watch   # For hot-reloading during development
cargo install cargo-edit    # For managing Cargo.toml dependencies
cargo install cargo-audit   # For checking security vulnerabilities
```

### 3. System Dependencies
Depending on your target hardware and instrument types, you may need additional system libraries (e.g., NI-VISA drivers for VISA instruments, HDF5 libraries for HDF5 storage). Consult specific instrument documentation for details.

## Project Setup

### 1. Clone the Repository
```bash
git clone <repository_url>
cd rust-daq
```

### 2. Project Structure (V4 Focus)
The project is transitioning to a V4 architecture. Key directories and files will include:
```
rust-daq/
├── Cargo.toml                  # Project dependencies and metadata
├── src/                        # Main application source code
│   ├── main.rs                 # Application entry point
│   ├── app/                    # Main application logic (e.g., DaqApp)
│   ├── core/                   # V4 core traits, types, and utilities
│   ├── instruments_v4/         # V4 instrument actor implementations
│   ├── config/                 # Figment-based configuration
│   └── gui/                    # Egui-based user interface
├── crates/                     # Workspace crates (e.g., daq-core for V4)
├── docs/                       # Organized documentation
│   ├── architecture/           # Architectural Decision Records (ADRs)
│   ├── getting_started/        # This guide
│   ├── guides/                 # General guides (data, deployment, etc.)
│   ├── instruments/            # Instrument-specific protocols, notes
│   ├── project_management/     # Project management, agent-related docs
│   └── archive/                # Obsolete documentation
├── archive/                    # Obsolete V1, V2, V3 code (not part of build)
└── tests/                      # Integration and unit tests
```

### 3. Core Dependencies (Cargo.toml - V4 Relevant)
The `Cargo.toml` will be updated to reflect the V4 architecture. Key dependencies will include:
```toml
[dependencies]
# Actor Framework
kameo = "0.1" # Or latest version

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# GUI framework
eframe = { version = "0.25", features = ["persistence"] }
egui = "0.25"
egui_plot = "0.25" # For plotting in egui

# High-performance Data Handling
arrow = { version = "52.0", features = ["full"] } # Apache Arrow for in-memory data
polars = { version = "0.36", features = ["full"] } # For data manipulation

# Hierarchical Data Storage
hdf5 = "0.8" # For HDF5 file format

# Logging and Diagnostics
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Configuration Management
figment = { version = "0.10", features = ["toml", "env"] }

# Command Line Interface
clap = { version = "4.0", features = ["derive"] }

# Error Handling
thiserror = "1.0"
anyhow = "1.0"

# Instrument Control (example)
visa-rs = "0.1" # For VISA instruments
serialport = "4.3" # For serial communication

# Numerical Processing
ndarray = "0.15"
rustfft = "6.0"
```

## V4 Architectural Overview

The project is transitioning to a unified V4 architecture based on **Kameo actors**, **Apache Arrow** for data, and **HDF5** for storage. For a comprehensive understanding of the new architecture, please refer to:
*   [V4 System Architecture](../../ARCHITECTURE.md)
*   [Architectural Flaw Analysis](../architecture/ARCHITECTURAL_FLAW_ANALYSIS.md)
*   [Rust Library Recommendations](../architecture/RUST_LIBRARY_RECOMMENDATIONS.md)
*   [Additional Library Research](../architecture/ADDITIONAL_LIBRARY_RESEARCH.md)

## Development Workflow

### 1. Running the Application
```bash
# Development mode with hot reload
cargo watch -x run

# Release mode
cargo run --release
```

### 2. Testing
```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture
```

### 3. Code Quality
```bash
# Format code
cargo fmt

# Check for issues
cargo clippy

# Audit dependencies
cargo audit
```

## Next Steps (V4 Development)

1.  **Review V4 Plan:** Familiarize yourself with the `beads` issue tracker (main epic `bd-xvpw`) for the detailed refactoring plan.
2.  **Implement Core Traits:** Begin implementing the V4 core traits and types within the new `daq-core` crate.
3.  **Configuration Setup:** Set up the `figment`-based configuration system.
4.  **Logging Setup:** Initialize the `tracing` infrastructure.
5.  **First Vertical Slice:** Implement the first instrument actor (e.g., Newport 1830C) using Kameo, Arrow, and HDF5.

This guide will be updated as the V4 architecture stabilizes.