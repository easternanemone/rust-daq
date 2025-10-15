# AGENT.md

## Build/Test Commands
- `cargo check` - Fast compile-time validation (run before PRs)
- `cargo run` - Launch with default features; `cargo run --features full` for all features
- `cargo test --all-features` - Run all tests with optional backends
- `cargo test test_name` - Run specific test by name
- `cargo clippy --all-targets --all-features` - Static analysis linting
- `cargo fmt` - Format code (required before commits)

## Architecture
Single-crate Rust DAQ application with modules: `core` (traits), `instrument/` (drivers), `data/` (processors), `gui/` (egui). Future: workspace with `rust_daq/` GUI crate, plugin system, PyO3 bindings in `python/`.

## Code Style
- Standard Rust conventions: `snake_case` functions/files, `CamelCase` types, `SCREAMING_SNAKE_CASE` constants
- 4-space indentation, `rustfmt` defaults
- Async-first design with Tokio, trait-based interfaces, Result-based error handling
- Use `thiserror` for custom errors, `anyhow` for generic errors

## Key Dependencies
egui/eframe (GUI), tokio (async), serde/config (serialization), ringbuf (data), optional: hdf5, arrow2, visa-rs, serialport

## Testing
Unit tests beside modules, integration in `tests/`. Mock hardware with feature flags for CI. Use `cargo test --all-features` before pushing.

## Special Notes
- Set `BEADS_DB=.beads/daq.db` when using `bd` command
- Conventional commits (`feat:`, `fix:`) with issue references
- Multi-agent: request separate `git worktree` to avoid conflicts
