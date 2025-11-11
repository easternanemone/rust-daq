# JULES.md - Jules Agent Context

**General context for Jules AI coding agents** working on the rust-daq project.

## Project Goals

Build a high-performance scientific data acquisition (DAQ) system in Rust as a modular alternative to Python frameworks like PyMoDAQ. Core architecture: async-first with Tokio runtime, egui GUI, trait-based plugin system for instruments and data processors.

## Available Tools & Resources

### Hardware Access via Tailscale

Some tasks require physical hardware (cameras, motion controllers, lasers, power meters) connected to the **maitai-eos** machine.

**Environment Variables** (pre-configured in Jules VM):
- `TAILSCALE_AUTHKEY` - Authentication for Tailscale
- `TAILSCALE_DOMAIN` - Your tailnet domain
- `HARDWARE_MACHINE` - Set to `maitai-eos`

**Usage**:
```bash
# Verify connection
ping maitai-eos

# Run commands on hardware machine
ssh user@maitai-eos "command"
```

### Issue Tracking with Beads

This project uses **[beads](https://github.com/steveyegge/beads)** for issue tracking (NOT GitHub issues or markdown TODOs).

**Environment Variables**:
- `BEADS_DB` - Already set to `.beads/daq.db` (project-local tracker)
- `BD_ACTOR` - Your Jules session ID (for audit trail)

**Key Commands**:
```bash
# Find ready work
bd ready --json

# Show issue details
bd show bd-XXX

# Update status as you work
bd update bd-XXX --status in_progress
bd update bd-XXX --notes "Progress details"

# Create new issues when discovering work
bd create "Issue title" -t feature -p 1
bd dep add <new-id> <parent-id> --type discovered-from
```

**Important**:
- Update issues to `in_progress` when starting work
- Add `--notes` to document your progress
- Commit `.beads/issues.jsonl` with your code changes
- The orchestrator (Claude Code) will close issues after reviewing your work

### ByteRover Memory System

Share knowledge with other agents via ByteRover:

```bash
# Retrieve context at start
brv retrieve -q "topic you're working on"

# Record learnings (be specific with file:line references)
brv add -s "Lessons Learned" -c "src/file.rs:123 - Specific learning"
brv add -s "Common Errors" -c "src/file.rs:456 - Error to avoid"

# Share with team when done
brv push -y
```

**Standard Sections**: Lessons Learned, Best Practices, Common Errors, Architecture, Testing

## Hierarchical Task Delegation

Jules sessions can spawn other Jules sessions for complex tasks:

```bash
# Create child sessions for subtasks
jules new --repo TheFermiSea/rust-daq "Subtask description"

# Monitor child sessions
jules remote list --session | grep "Completed"

# Pull completed work
jules pull <child-session-id>
```

**Use when**: Your task is complex with multiple independent subtasks that can be parallelized.

## Project Documentation

**Read these files** to understand the codebase:

### Architecture & Patterns
- `docs/rust-daq-app-architecture.md` - Overall architecture
- `src/core.rs` - Core traits (Instrument, DataProcessor, StorageWriter)
- `src/core_v3.rs` - V3 instrument architecture with capability traits
- `src/instruments_v2/newport_1830c_v3.rs` - **Reference V3 implementation** (1,067 lines)

### Development Guides
- `CLAUDE.md` - Development conventions and commands
- `AGENTS.md` - Multi-agent coordination patterns
- `docs/BEADS_INSTALLATION.md` - Beads workflow details
- `docs/BYTEROVER_MULTI_AGENT_SETUP.md` - ByteRover usage

### Domain-Specific
- `docs/rust-daq-instrument-guide.md` - Instrument control patterns
- `docs/rust-daq-data-guide.md` - Data management
- `docs/rust-daq-gui-guide.md` - GUI development

## Common Commands

### Development
```bash
cargo check              # Fast compile check
cargo test               # Run unit tests
cargo test -- --nocapture # Show test output
cargo fmt                # Format code
cargo clippy             # Lint checking
```

### Testing Strategy
- Unit tests: Mock all I/O, test logic
- Integration tests: Test with real hardware if needed
- Target: 85% coverage for new code

### Before Submitting PR
```bash
cargo test
cargo fmt
cargo clippy
ast-grep scan --config rust_daq_ast_grep_rules.yml
git status  # Verify .beads/issues.jsonl committed
```

## Environment Notes

- **Async runtime**: Tokio (use `tokio::time::sleep`, not `std::thread::sleep`)
- **Error handling**: Use `DaqError` enum (defined in `src/error.rs`), not `anyhow!`
- **Configuration**: Load from TOML via `config/default.toml`
- **Testing**: Use `mockall` crate for mocking traits

## Making Decisions

You have full autonomy to:
- Choose implementation approaches based on documentation
- Decide when to use mocks vs hardware testing
- Break complex tasks into child Jules sessions
- Discover and create new beads issues
- Ask questions when requirements are unclear

**Key principle**: Read the documentation thoroughly, understand the existing patterns (especially the Newport V3 reference), and make informed engineering decisions. The orchestrator trusts your judgment.

## Success Criteria

- Code compiles (`cargo check` passes)
- Tests pass (`cargo test` succeeds)
- Clippy warnings addressed
- Beads issue updated with progress
- ByteRover updated with learnings (file:line specificity)
- PR description explains what/why/how

---

**Remember**: You're an autonomous engineer. Use the tools and documentation to make good decisions. The orchestrator will help with integration and coordination, but the implementation approach is yours to determine.
