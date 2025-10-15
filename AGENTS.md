# Repository Guidelines

## Project Structure & Module Organization
The workspace root `Cargo.toml` coordinates the `rust_daq` GUI application and companion crates in `plugins/` (instrument drivers, PID controller, PVCAM bindings). Core application code lives in `rust_daq/src`, with UI panels, processing pipelines, and persistence modules separated into subdirectories. Shared configuration samples reside in `config/`, runtime logs in `logs/`, and workspace-wide integration tests in `tests/`. Build artifacts stay in `target/`; avoid committing anything from that directory.

## Build, Test, and Development Commands
- `cargo check --workspace` — Fast compile-time validation for all crates before opening a PR.
- `cargo run -p rust_daq` — Launches the desktop application with default features; append `--features full` to exercise optional storage and instrument stacks.
- `cargo fmt --all` — Formats every crate using `rustfmt`; required prior to commits.
- `cargo clippy --workspace --all-targets --all-features` — Static analysis configured to catch common Rust pitfalls across binaries, libs, and tests.

## Coding Style & Naming Conventions
We rely on standard Rust 4-space indentation and `rustfmt` defaults. Use `snake_case` for functions and files, `CamelCase` for types, and SCREAMING_SNAKE_CASE for constants. Keep modules cohesive—group instrument drivers under `plugins/` and shared abstractions under `rust_daq/src/lib`. Document intent with concise comments when logic spans multiple async tasks or channels.

## Testing Guidelines
Unit tests live beside their modules; integration coverage belongs in `tests/`, e.g., `tests/fft_processor_integration_test.rs`. Run `cargo test --workspace --all-features` before pushing to ensure driver crates compile with optional backends. When adding hardware integrations, provide mocked pathways or feature flags so tests run in CI without devices attached.

## Commit & Pull Request Guidelines
Follow the Conventional Commits pattern already in history (`feat:`, `fix:`, etc.), referencing issue IDs where relevant. Each PR should summarise the change scope, note impacted subsystems (UI, pipeline, plugin), and include screenshots or logs when UI or acquisition behavior changes. Link configuration updates to the matching sample in `config/` so reviewers can reproduce the scenario.
