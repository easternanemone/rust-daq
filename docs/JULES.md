# JULES.md - Jules Agent Configuration and Best Practices

**For ALL Jules AI coding agents** working on rust-daq project.

## Project Overview

High-performance scientific data acquisition (DAQ) system in Rust. You are working on a modular, async-first architecture with Tokio runtime, egui GUI, and trait-based plugin system for instruments.

## Hardware Access via Tailscale

**CRITICAL**: Many tasks require access to physical hardware (cameras, motion controllers, lasers, power meters) connected to the maitai-eos machine.

### Tailscale Setup (Required for Hardware Tasks)

**GitHub Actions Configuration:**
- Tailscale is pre-configured in the Jules VM
- Use environment variables to connect to the hardware machine
- **Machine name**: `maitai-eos`
- **Domain**: Check `TAILSCALE_DOMAIN` environment variable in your workflow
- **Auth**: Use `TAILSCALE_AUTHKEY` (already configured in Jules environment)

**Connection Pattern:**
```yaml
# In .github/workflows/jules-workflow.yml (already configured)
env:
  TAILSCALE_AUTHKEY: ${{ secrets.TAILSCALE_AUTHKEY }}
  TAILSCALE_DOMAIN: <your-tailnet-domain>
  HARDWARE_MACHINE: maitai-eos

# Connect in your test scripts:
ping maitai-eos  # Verify connection
ssh user@maitai-eos "command"  # Run remote commands
```

**Hardware-Dependent Tasks:**
- PVCAM camera integration → requires maitai-eos connection
- ESP300 motion controller → requires serial port access via maitai-eos
- MaiTai laser control → requires maitai-eos
- Newport 1830C power meter → can use mocks OR maitai-eos
- Elliptec rotators → requires maitai-eos for integration tests

**Mock-First Testing:**
When possible, use mock devices for unit tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;

    mock! {
        pub SerialDevice {}
        #[async_trait]
        impl SerialDevice for SerialDevice {
            async fn send_command(&self, cmd: &str) -> Result<String>;
        }
    }
}
```

Only request hardware access for integration tests that MUST verify actual hardware behavior.

## Beads Issue Tracker Integration

**CRITICAL**: This project uses [beads](https://github.com/steveyegge/beads) for issue tracking, NOT GitHub issues or markdown TODOs.

### Basic Beads Workflow

```bash
# ALWAYS use project-local database
export BEADS_DB=.beads/daq.db

# Find ready work (no blockers)
bd ready --json

# Show specific issue
bd show bd-XXX

# Update status
bd update bd-XXX --status in_progress
bd update bd-XXX --status closed --notes "Implementation details"

# Create new issues ONLY when discovering work during implementation
bd create "New issue discovered" -t feature -p 1

# Link discovered work
bd dep add <new-id> <parent-id> --type discovered-from
```

**IMPORTANT**:
- DO NOT close beads issues yourself - only update to `in_progress`
- Orchestrator (Claude Code) will close issues after reviewing your work
- DO commit `.beads/issues.jsonl` with your code changes
- If you discover new work, create issues and link them

### Common Beads Commands

```bash
# Check issue priority and status
bd show bd-197 | grep -E "(priority|status|title)"

# Update with notes about your progress
bd update bd-197 --notes "Added MotionController trait, implemented move_absolute/move_relative/get_position"

# Link related issues
bd dep add bd-197.1 bd-197 --type discovered-from
```

## V3 Architecture Migration Pattern

**THE reference implementation**: `src/instruments_v2/newport_1830c_v3.rs` (1,067 lines by Codex)

### V3 Migration Checklist

**1. Core Trait Implementation** (`src/core_v3.rs`):
```rust
#[async_trait]
pub trait Instrument: Send + Sync {
    async fn connect(&mut self, config: &AdapterConfig) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn handle_command(&mut self, cmd: Command) -> Result<()>;
    async fn data_stream(&self) -> Receiver<Measurement>;
}
```

**2. Capability Trait** (PowerMeter, MotionController, Camera, Spectrum, LaserController):
```rust
#[async_trait]
pub trait PowerMeter: Instrument {
    async fn read_power(&self) -> Result<f64>;
    async fn set_wavelength(&mut self, wavelength_nm: f64) -> Result<()>;
    async fn set_range(&mut self, range: PowerRange) -> Result<()>;
}
```

**3. SerialDevice Abstraction** (for testability):
```rust
pub trait SerialDevice: Send + Sync {
    async fn send_command(&self, command: &str) -> Result<String>;
    async fn write_bytes(&self, data: &[u8]) -> Result<()>;
}

// Production implementation
pub struct RealSerialDevice { /* ... */ }

// Test mock
#[cfg(test)]
use mockall::mock;
mock! {
    pub SerialDevice {}
    #[async_trait]
    impl SerialDevice for SerialDevice {
        async fn send_command(&self, command: &str) -> Result<String>;
    }
}
```

**4. Polling Loop Pattern**:
```rust
async fn spawn_poll_loop(&mut self) -> Result<()> {
    let device = self.device.clone();
    let tx = self.measurement_tx.clone();
    let poll_interval = self.config.poll_interval_ms;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(poll_interval));
        loop {
            interval.tick().await;
            match device.send_command("MEASURE?").await {
                Ok(value) => {
                    let measurement = Measurement::Scalar(DataPoint { /* ... */ });
                    let _ = tx.send(measurement);
                }
                Err(e) => eprintln!("Poll error: {}", e),
            }
        }
    });
    Ok(())
}
```

**5. Unit Tests** (6+ tests minimum):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect_initializes_device() {
        let mut mock = MockSerialDevice::new();
        mock.expect_send_command()
            .with(eq("*IDN?"))
            .returning(|_| Ok("Newport 1830-C".to_string()));

        let mut instrument = Newport1830CV3::new(Box::new(mock));
        let result = instrument.connect(&default_config()).await;
        assert!(result.is_ok());
    }

    // More tests: read_power, set_wavelength, error_handling, disconnect, etc.
}
```

**6. Integration into InstrumentManagerV3** (`src/app_actor.rs`):
```rust
instrument_manager_v3.register_factory("newport_1830c", |config| {
    Box::new(Newport1830CV3::new(config))
});
```

**7. Configuration** (`config/default.toml`):
```toml
[[instruments_v3]]
id = "newport_pm_v3"
type = "newport_1830c"  # Must match registry key
[instruments_v3.params]
device_path = "/dev/ttyUSB0"
poll_interval_ms = 500
wavelength_nm = 1064.0
```

## Jules Hierarchical Delegation Pattern

**NEW CAPABILITY**: Jules sessions can spawn other Jules sessions for hierarchical task breakdown.

### When to Use Hierarchical Delegation

**Use hierarchical delegation when:**
- Your task is an epic (bd-XXX with 10+ subtasks)
- You need to parallelize implementation work
- Different subtasks require specialized skills (serial, PVCAM, ESP300)
- You want to follow the "divide and conquer" pattern

**Pattern**:
```bash
# Parent Jules session (you) creates child sessions
jules new --repo TheFermiSea/rust-daq "Subtask 1: Implement Camera trait"
jules new --repo TheFermiSea/rust-daq "Subtask 2: Implement frame acquisition"
jules new --repo TheFermiSea/rust-daq "Subtask 3: Add unit tests"

# Monitor child sessions
jules remote list --session | grep "Completed"

# Pull completed child session results
jules pull <child-session-id>

# Integrate child session work into parent
# (merge changes, run cargo check, create PR)
```

**Example Hierarchical Breakdown**:
```
bd-32: PVCAM V3 Driver (parent)
├── Child Session 1: Camera trait definition
├── Child Session 2: Frame acquisition loop
├── Child Session 3: Unit tests with MockCamera
└── Child Session 4: Integration into InstrumentManagerV3
```

**IMPORTANT**:
- Each child session should be independently testable
- Child sessions inherit Tailscale access if needed
- Parent session is responsible for integration and PR submission
- Use beads to track child session dependencies

## Common Task Types

### V3 Instrument Migration

**Goal**: Migrate V1 instrument to V3 architecture following Newport pattern

**Steps**:
1. Read reference: `src/instruments_v2/newport_1830c_v3.rs`
2. Define capability trait if needed (Camera, Spectrum, LaserController)
3. Implement `core_v3::Instrument` + capability trait
4. Add `SerialDevice` abstraction (or appropriate communication trait)
5. Write 6+ unit tests with mocks
6. Wire into `InstrumentManagerV3`
7. Enable in `config/default.toml`
8. Run `cargo check` and `cargo test`
9. Update beads: `bd update bd-XXX --status in_progress --notes "Completed V3 implementation"`

**Expected Files**:
- `src/instruments_v2/<instrument>_v3.rs` (new)
- `src/core_v3.rs` (add capability trait if needed)
- `src/app_actor.rs` (register factory)
- `config/default.toml` (enable instrument)
- `.beads/issues.jsonl` (update status)

### Integration Tests

**Goal**: Verify instrument behavior with hardware

**Pattern**:
```rust
#[cfg(test)]
#[cfg(feature = "integration_tests")]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore]  // Requires hardware
    async fn test_real_hardware() {
        // Connect to maitai-eos via Tailscale
        let mut instrument = Instrument::connect("maitai-eos:/dev/ttyUSB0").await.unwrap();
        // Test actual hardware behavior
    }
}
```

**Run with**: `cargo test --features integration_tests -- --ignored`

### Dynamic Configuration (bd-128, bd-130, bd-131)

**Goal**: Hot-reload config, transactions, persistence

**Pattern**:
- Use `notify` crate for file watching
- Implement atomic updates with rollback
- Persist to TOML files
- Test with `tests/dynamic_config_test.rs`

## Error Handling Standards

**Use `DaqError` enum** (not `anyhow!`):
```rust
use crate::error::DaqError;

// CORRECT
return Err(DaqError::SerialTimeout {
    device: "Newport PM".to_string(),
    timeout_ms: 1000
});

// INCORRECT
return Err(anyhow!("Serial timeout"));
```

**Add specific variants** when needed:
```rust
// In src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum DaqError {
    #[error("Hardware error on {device}: {message}")]
    HardwareError { device: String, message: String },
    // ... more variants
}
```

## Testing Strategy

**Test Pyramid**:
1. **Unit Tests** (6+ per instrument): Mock all I/O, test logic
2. **Integration Tests**: Test with real hardware via Tailscale
3. **Performance Tests**: Benchmark with `criterion`

**Coverage Target**: 85% for new code

**Run Tests**:
```bash
cargo test                              # Unit tests only
cargo test --features integration_tests # With hardware
cargo test -- --nocapture               # Show output
```

## Pull Request Submission

**Before submitting PR**:
```bash
# 1. Ensure all tests pass
cargo test

# 2. Format code
cargo fmt

# 3. Check clippy warnings
cargo clippy

# 4. Run ast-grep checks
ast-grep scan --config rust_daq_ast_grep_rules.yml

# 5. Verify beads updated
cat .beads/issues.jsonl | grep "bd-XXX"

# 6. Create PR with comprehensive description
gh pr create --title "feat: implement <feature> (bd-XXX)" --body "
## Summary
<what was implemented>

## Testing
- Unit tests: X passing
- Integration tests: Y passing (if applicable)
- Manual testing: Z verified

## Beads
Addresses bd-XXX: <issue title>

## Reference
Follows pattern from: <reference file>
"
```

**PR Title Format**: `feat: implement <feature> (bd-XXX)`

## ByteRover Memory System

**CRITICAL**: Share knowledge with other agents via ByteRover

**Workflow**:
```bash
# 1. Start session: retrieve context
brv retrieve -q "V3 instrument migration patterns"

# 2. During work: record learnings
brv add -s "Lessons Learned" -c "src/instruments_v2/<file>.rs:123 - Specific learning with file:line reference"

# 3. End session: share with team
brv push -y
```

**Good Memory Example**:
```bash
brv add -s "Common Errors" -c "src/instruments_v2/pvcam_v3.rs:450 - PVCAM exposure timeout = 2x exposure_ms + 1000ms buffer to prevent frame loss"
```

**Standard Sections**:
- Lessons Learned
- Best Practices
- Common Errors
- Architecture
- Testing
- Project Structure and Dependencies

## Common Pitfalls

**AVOID**:
1. ❌ Blocking calls in async functions (`std::thread::sleep`)
2. ❌ Hardcoded timeouts or device paths
3. ❌ Using `.unwrap()` or `.expect()` outside tests
4. ❌ Forgetting to spawn poll loop in `connect()`
5. ❌ Missing SerialDevice abstraction for testability
6. ❌ Closing beads issues yourself (orchestrator does this)
7. ❌ Creating PRs without running `cargo check`

**DO**:
1. ✅ Use tokio::time::sleep for async delays
2. ✅ Load timeouts/paths from `AdapterConfig`
3. ✅ Proper `Result` error handling with `DaqError`
4. ✅ Call `spawn_poll_loop()` after serial init
5. ✅ Inject dependencies via traits for mocking
6. ✅ Update beads to `in_progress`, let orchestrator close
7. ✅ Always verify with `cargo check && cargo test`

## Resources

**Key Files to Read**:
- `src/instruments_v2/newport_1830c_v3.rs` - V3 reference implementation
- `src/core_v3.rs` - Trait definitions
- `src/error.rs` - Error handling patterns
- `docs/AGENTS.md` - Multi-agent coordination
- `docs/BYTEROVER_MULTI_AGENT_SETUP.md` - ByteRover usage
- `docs/BEADS_INSTALLATION.md` - Beads setup and workflow

**Commands Reference**:
- `cargo check` - Fast compile check
- `cargo test` - Run unit tests
- `cargo clippy` - Lint checking
- `bd ready --json` - Find ready work
- `jules remote list --session` - Monitor sessions
- `brv retrieve -q "topic"` - Get context

## Environment Variables (Auto-Configured)

Jules sessions have these variables pre-configured:
- `TAILSCALE_AUTHKEY` - Tailscale authentication
- `TAILSCALE_DOMAIN` - Your tailnet domain
- `BEADS_DB` - Set to `.beads/daq.db` for project-local tracker
- `BD_ACTOR` - Your Jules session ID for audit trail

**Access in your code**:
```bash
echo "Tailnet: $TAILSCALE_DOMAIN"
echo "Hardware machine: maitai-eos"
ping maitai-eos  # Should work if Tailscale connected
```

## Success Metrics

**Target**:
- ✅ All unit tests passing
- ✅ No cargo clippy warnings
- ✅ ast-grep checks passing
- ✅ 85%+ code coverage for new code
- ✅ Integration tests passing (if hardware-dependent)
- ✅ Beads issue updated to `in_progress`
- ✅ ByteRover memory recorded with file:line references
- ✅ PR submitted with comprehensive description

**Current Project Stats**:
- 52 Jules sessions running
- 36 completed (69% success rate)
- 2 major V3 migrations completed (Newport, ESP300)
- Reference implementation: 1,067 lines (Newport V3)

---

**Remember**: You're part of a multi-agent team. Claude Code (orchestrator), Gemini (advisor), and Codex (implementer) are coordinating your work. Focus on your specific task, test thoroughly, update beads/ByteRover, and let the orchestrator handle integration.
