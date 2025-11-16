# AGENT.md

## Build/Test Commands
- `cargo check` - Fast compile-time validation (run before PRs)
- `cargo run` - Launch with default features; `cargo run --features full` for all features
- `cargo test --all-features` - Run all tests with optional backends
- `cargo test test_name` - Run specific test by name
- `cargo clippy --all-targets --all-features` - Static analysis linting
- `cargo fmt` - Format code (required before commits)

## Architecture
The Rust DAQ application is structured as a multi-crate workspace, leveraging the V4 architecture principles:
- **Kameo Actors:** Core components (instruments, data processors, storage, GUI) are implemented as independent, supervised Kameo actors for concurrency, fault tolerance, and clear separation of concerns.
- **Apache Arrow:** All in-memory data handling and transfer between actors utilize `apache/arrow-rs` `RecordBatch`es for zero-copy efficiency and interoperability.
- **HDF5 Storage:** Persistent data storage is managed via `hdf5-rust` for high-performance, structured scientific data.
- **Polars:** Data processing and analysis leverage `polars` for efficient DataFrame operations.
- **Modular Design:** The application is composed of several crates (e.g., `daq-core`, `rust-daq-app`) within a workspace, facilitating a plugin-like system.

## Key Dependencies
- **`kameo`**: Asynchronous actor framework for concurrent and fault-tolerant components.
- **`tokio`**: Asynchronous runtime for non-blocking operations.
- **`egui`/`eframe`**: Immediate-mode GUI framework for the user interface.
- **`arrow` / `arrow-array` / `arrow-schema`**: Core Apache Arrow data structures for efficient in-memory data.
- **`polars`**: High-performance DataFrame library for data manipulation and analysis.
- **`hdf5`**: Bindings for HDF5 for structured data storage.
- **`serde` / `config` / `figment`**: For robust configuration management and serialization.
- **`tracing` / `tracing-subscriber`**: Unified logging and diagnostics framework.
- **`async-trait`**: For defining asynchronous traits.
- **`thiserror` / `anyhow`**: For structured and ergonomic error handling.
- **`serialport` / `visa-rs` / `scpi`**: For instrument communication protocols.
- **`ndarray` / `wide`**: For numerical processing and SIMD optimizations.

## Testing
Unit tests beside modules, integration in `tests/`. Mock hardware with feature flags for CI. Use `cargo test --all-features` before pushing.

## Special Notes
- Set `BEADS_DB=.beads/daq.db` when using `bd` command
- Conventional commits (`feat:`, `fix:`) with issue references
- Multi-agent: request separate `git worktree` to avoid conflicts
