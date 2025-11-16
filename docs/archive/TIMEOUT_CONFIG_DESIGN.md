# Timeout Configuration Architecture Design
**Task**: bd-51b1  
**Date**: 2025-11-07  
**Status**: Design Phase - Blocks bd-ltd3 Implementation  
**Author**: Claude Code Design Review

## Executive Summary

This document defines the architecture for making timeouts configurable in the rust-daq system. Currently, 23 hardcoded timeout values exist across the codebase, reducing deployment flexibility and making the system difficult to tune for different hardware configurations.

**Key Decisions**:
1. **Phase 1**: Global defaults only (simple, immediate value)
2. **Phase 2**: Per-instrument overrides (future enhancement)
3. **Config location**: `[application.timeouts]` (nested, consistent with existing patterns)
4. **Backward compatibility**: Default values if section missing
5. **Validation**: Min/max bounds with clear error messages

---

## 1. Current State Analysis

### 1.1 Timeout Distribution (23 instances)

| Category | Count | Typical Values | Locations |
|----------|-------|----------------|-----------|
| Serial I/O | 7 | 1s | serial_adapter.rs, scpi_common.rs, esp300.rs |
| Protocol Commands | 2 | 2s | scpi_common.rs, maitai.rs |
| Network Operations | 10 | 5-10s | server_actor.rs |
| Instrument Lifecycle | 6 | 5-6s | instrument_manager_v3.rs, primitives.rs |

### 1.2 Key Files Affected

**Serial Communication** (7 instances):
```rust
// src/adapters/serial_adapter.rs:53
timeout: Duration::from_secs(1)

// src/instrument/scpi_common.rs:55,70
timeout: Duration::from_secs(1)  // read
timeout: Duration::from_secs(2)  // command
```

**Network Operations** (10 instances):
```rust
// src/network/server_actor.rs:218,248,267,292,325,418
timeout(Duration::from_secs(5), rx).await  // Client request handling
timeout(Duration::from_secs(10), ...)      // Cleanup operations
```

**Instrument Management** (6 instances):
```rust
// src/instrument_manager_v3.rs:499,694,890
Duration::from_secs(5)  // Connect timeout
Duration::from_secs(5)  // Measurement receive timeout
Duration::from_secs(6)  // Shutdown timeout
```

### 1.3 Why Hardcoded Timeouts Are Problematic

1. **Hardware Variability**: Different instruments have different response times
2. **Network Conditions**: Variable latency across deployments
3. **Debugging**: Cannot increase timeouts during troubleshooting without recompilation
4. **Testing**: Mock instruments need shorter timeouts than real hardware
5. **Production Tuning**: Cannot optimize without code changes

---

## 2. Design Decisions

### 2.1 Config Structure: `[application.timeouts]`

**Decision**: Use nested structure under `[application]` section

**Rationale**:
- Matches existing pattern in `config/default.toml` (lines 6-17)
- `[application]` already contains system-wide settings (broadcast/command capacity)
- `[application.data_distributor]` sets precedent for nested subsections
- Keeps all application-level settings in one logical namespace

**Rejected Alternative**: `[timeouts]` at root level
- Violates existing organizational pattern
- Creates inconsistency with other system settings
- Future config file would have mix of root and nested sections

**Example Structure**:
```toml
[application]
broadcast_channel_capacity = 1024
command_channel_capacity = 32

[application.timeouts]
# Serial I/O timeouts (milliseconds)
serial_read_timeout_ms = 1000
serial_write_timeout_ms = 1000

# Protocol timeouts (milliseconds)
scpi_command_timeout_ms = 2000

# Network timeouts (milliseconds)
network_client_timeout_ms = 5000
network_cleanup_timeout_ms = 10000

# Instrument lifecycle timeouts (milliseconds)
instrument_connect_timeout_ms = 5000
instrument_shutdown_timeout_ms = 6000
instrument_measurement_timeout_ms = 5000

[application.data_distributor]
subscriber_capacity = 1024
# ... existing settings ...
```

### 2.2 Inheritance Model: Phase 1 vs Phase 2

#### Phase 1: Global Defaults Only (IMPLEMENT FIRST)

**Scope**: All timeouts use values from `[application.timeouts]`

**Rationale**:
- Simple to implement (2-3 hours)
- Solves 80% of use cases immediately
- No complex inheritance logic needed
- Easy to validate and test
- Backward compatible with default values

**Implementation**:
```rust
// src/config.rs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApplicationSettings {
    #[serde(default = "default_broadcast_capacity")]
    pub broadcast_channel_capacity: usize,
    
    #[serde(default = "default_command_capacity")]
    pub command_channel_capacity: usize,
    
    #[serde(default)]
    pub data_distributor: DataDistributorSettings,
    
    // NEW: Phase 1 - Global timeouts
    #[serde(default)]
    pub timeouts: TimeoutSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct TimeoutSettings {
    // Serial I/O
    pub serial_read_timeout_ms: u64,
    pub serial_write_timeout_ms: u64,
    
    // Protocol
    pub scpi_command_timeout_ms: u64,
    
    // Network
    pub network_client_timeout_ms: u64,
    pub network_cleanup_timeout_ms: u64,
    
    // Instrument lifecycle
    pub instrument_connect_timeout_ms: u64,
    pub instrument_shutdown_timeout_ms: u64,
    pub instrument_measurement_timeout_ms: u64,
}

impl Default for TimeoutSettings {
    fn default() -> Self {
        Self {
            // Serial I/O (1s default)
            serial_read_timeout_ms: 1000,
            serial_write_timeout_ms: 1000,
            
            // Protocol (2s default)
            scpi_command_timeout_ms: 2000,
            
            // Network (5-10s defaults)
            network_client_timeout_ms: 5000,
            network_cleanup_timeout_ms: 10000,
            
            // Instrument lifecycle (5-6s defaults)
            instrument_connect_timeout_ms: 5000,
            instrument_shutdown_timeout_ms: 6000,
            instrument_measurement_timeout_ms: 5000,
        }
    }
}
```

**Usage Example**:
```rust
// Before (hardcoded):
let timeout = Duration::from_secs(1);

// After (configurable):
use std::time::Duration;

let timeout = Duration::from_millis(
    settings.application.timeouts.serial_read_timeout_ms
);
```

#### Phase 2: Per-Instrument Overrides (FUTURE)

**Scope**: Allow specific instruments to override global defaults

**When to Implement**: After Phase 1 is stable and if real-world need emerges

**Example TOML**:
```toml
[application.timeouts]
serial_read_timeout_ms = 1000  # Global default

[instruments.slow_spectrometer]
type = "scpi_custom"
# Instrument-specific override
timeouts.serial_read_timeout_ms = 5000  # This instrument is slower
```

**Implementation Strategy**:
```rust
// Phase 2 addition to instrument config validation
impl Settings {
    fn get_timeout_for_instrument(&self, instrument_id: &str, timeout_type: TimeoutType) -> Duration {
        // 1. Check instrument-specific override
        if let Some(override_ms) = self.get_instrument_timeout_override(instrument_id, timeout_type) {
            return Duration::from_millis(override_ms);
        }
        
        // 2. Fall back to global default
        let global_ms = match timeout_type {
            TimeoutType::SerialRead => self.application.timeouts.serial_read_timeout_ms,
            TimeoutType::SerialWrite => self.application.timeouts.serial_write_timeout_ms,
            // ... other types ...
        };
        
        Duration::from_millis(global_ms)
    }
}
```

**Phase 2 Deferred Rationale**:
- Adds complexity to config validation
- Requires inheritance logic in every timeout usage site
- No current evidence that different instruments need different timeouts
- Can be added later without breaking changes

### 2.3 Timeout Categories

Based on analysis of 23 instances, group into 8 distinct timeout types:

| Timeout Type | Default (ms) | Purpose | Affected Code |
|--------------|--------------|---------|---------------|
| `serial_read_timeout_ms` | 1000 | Serial port read operations | serial_adapter.rs, scpi_common.rs |
| `serial_write_timeout_ms` | 1000 | Serial port write operations | serial_adapter.rs |
| `scpi_command_timeout_ms` | 2000 | SCPI command/response cycle | scpi_common.rs, maitai.rs |
| `network_client_timeout_ms` | 5000 | Network client request handling | server_actor.rs |
| `network_cleanup_timeout_ms` | 10000 | Network cleanup operations | server_actor.rs |
| `instrument_connect_timeout_ms` | 5000 | Instrument connection/initialization | instrument_manager_v3.rs |
| `instrument_shutdown_timeout_ms` | 6000 | Graceful instrument shutdown | instrument_manager_v3.rs |
| `instrument_measurement_timeout_ms` | 5000 | Waiting for measurement data | instrument_manager_v3.rs |

**Naming Convention**:
- Use `_ms` suffix (milliseconds) for consistency with `metrics_window_secs`
- Descriptive names: `{subsystem}_{operation}_timeout_ms`
- No abbreviations except standard ones (ms, scpi)

### 2.4 Validation Rules

**Min/Max Bounds**:
```rust
impl TimeoutSettings {
    fn validate(&self) -> Result<()> {
        // Serial I/O: 100ms - 30s (prevent too-short hangs, too-long freezes)
        validate_timeout_range(self.serial_read_timeout_ms, 100, 30_000, "serial_read_timeout_ms")?;
        validate_timeout_range(self.serial_write_timeout_ms, 100, 30_000, "serial_write_timeout_ms")?;
        
        // Protocol: 500ms - 60s (commands need reasonable time)
        validate_timeout_range(self.scpi_command_timeout_ms, 500, 60_000, "scpi_command_timeout_ms")?;
        
        // Network: 1s - 120s (network operations can be slow)
        validate_timeout_range(self.network_client_timeout_ms, 1_000, 120_000, "network_client_timeout_ms")?;
        validate_timeout_range(self.network_cleanup_timeout_ms, 1_000, 120_000, "network_cleanup_timeout_ms")?;
        
        // Instrument lifecycle: 1s - 60s (hardware init can take time)
        validate_timeout_range(self.instrument_connect_timeout_ms, 1_000, 60_000, "instrument_connect_timeout_ms")?;
        validate_timeout_range(self.instrument_shutdown_timeout_ms, 1_000, 60_000, "instrument_shutdown_timeout_ms")?;
        validate_timeout_range(self.instrument_measurement_timeout_ms, 1_000, 60_000, "instrument_measurement_timeout_ms")?;
        
        Ok(())
    }
}

fn validate_timeout_range(value: u64, min: u64, max: u64, name: &str) -> Result<()> {
    if value < min || value > max {
        anyhow::bail!(
            "Timeout '{}' = {}ms is out of valid range ({}ms - {}ms)",
            name, value, min, max
        );
    }
    Ok(())
}
```

**Validation Strategy**:
- Permissive bounds (allow wide range for unusual hardware)
- Fail fast at config load time (not at runtime)
- Clear error messages with actual vs expected values
- Prevent common mistakes (0ms, negative, absurdly large)

---

## 3. Migration Strategy

### 3.1 Backward Compatibility

**Requirement**: Existing deployments without `[application.timeouts]` section MUST work

**Implementation**:
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApplicationSettings {
    // ... existing fields ...
    
    #[serde(default)]  // <-- KEY: Uses Default trait if missing
    pub timeouts: TimeoutSettings,
}
```

**Behavior**:
1. **New deployments**: Include `[application.timeouts]` in config.toml
2. **Existing deployments**: Missing section uses `TimeoutSettings::default()`
3. **Partial configs**: Omitted fields use defaults (serde default behavior)

**Example Partial Config**:
```toml
# User only wants to change serial timeout
[application.timeouts]
serial_read_timeout_ms = 5000
# Other timeouts use defaults from TimeoutSettings::default()
```

### 3.2 Migration Steps for Existing Code

**Step 1**: Add `TimeoutSettings` struct to `src/config.rs` (15 min)

**Step 2**: Update `config/default.toml` with full section (10 min)
```toml
[application.timeouts]
# Serial I/O timeouts
serial_read_timeout_ms = 1000
serial_write_timeout_ms = 1000

# Protocol timeouts
scpi_command_timeout_ms = 2000

# Network timeouts
network_client_timeout_ms = 5000
network_cleanup_timeout_ms = 10000

# Instrument lifecycle timeouts
instrument_connect_timeout_ms = 5000
instrument_shutdown_timeout_ms = 6000
instrument_measurement_timeout_ms = 5000
```

**Step 3**: Replace hardcoded values (1-2 hours)

**Before**:
```rust
// src/adapters/serial_adapter.rs:53
timeout: Duration::from_secs(1),
```

**After**:
```rust
// Pass settings to SerialAdapter constructor
impl SerialAdapter {
    pub fn new(settings: &TimeoutSettings) -> Self {
        Self {
            timeout: Duration::from_millis(settings.serial_read_timeout_ms),
            // ...
        }
    }
}
```

**Step 4**: Add validation to `Settings::validate()` (30 min)

**Step 5**: Update tests (30 min)
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_timeout_validation() {
        let mut settings = Settings::default();
        
        // Too short - should fail
        settings.application.timeouts.serial_read_timeout_ms = 50;
        assert!(settings.validate().is_err());
        
        // Too long - should fail
        settings.application.timeouts.serial_read_timeout_ms = 100_000;
        assert!(settings.validate().is_err());
        
        // Just right - should pass
        settings.application.timeouts.serial_read_timeout_ms = 1000;
        assert!(settings.validate().is_ok());
    }
    
    #[test]
    fn test_backward_compatibility() {
        // Config without [application.timeouts] should use defaults
        let toml_content = r#"
            log_level = "info"
            
            [application]
            broadcast_channel_capacity = 1024
            
            [storage]
            default_path = "./data"
            default_format = "csv"
            
            [instruments]
        "#;
        
        let settings: Settings = toml::from_str(toml_content).unwrap();
        assert_eq!(settings.application.timeouts.serial_read_timeout_ms, 1000);
        assert_eq!(settings.application.timeouts.scpi_command_timeout_ms, 2000);
    }
}
```

### 3.3 Deployment Migration Path

**For users with existing config.toml**:

1. **No action required** - Defaults will be used (same as current hardcoded values)
2. **Optional**: Add `[application.timeouts]` section to customize
3. **Optional**: Test with `cargo run --release` to verify settings load

**For new deployments**:

1. Copy `config/default.toml` as starting point
2. Adjust timeouts in `[application.timeouts]` as needed
3. Run validation: `cargo check` will catch config errors at load time

**Error Handling**:
```rust
// src/main.rs or wherever Settings::new() is called
let settings = Settings::new(Some("default"))
    .context("Failed to load configuration. Check config/default.toml for errors.")?;

// Validation happens inside Settings::new(), so invalid timeouts fail here
```

**Example Error Message**:
```
Error: Failed to load configuration. Check config/default.toml for errors.

Caused by:
    Timeout 'serial_read_timeout_ms' = 50ms is out of valid range (100ms - 30000ms)
```

---

## 4. Implementation Plan

### 4.1 Phase 1 Scope (Immediate)

**Goal**: Global timeout configuration with backward compatibility

**Time Estimate**: 2-3 hours implementation + 1 hour testing

**Tasks**:
1. ✅ Define `TimeoutSettings` struct with defaults
2. ✅ Add `timeouts` field to `ApplicationSettings`
3. ✅ Implement validation in `TimeoutSettings::validate()`
4. ✅ Update `config/default.toml` with full `[application.timeouts]` section
5. ✅ Replace 23 hardcoded `Duration::from_*` calls with config values
6. ✅ Add unit tests for validation and backward compatibility
7. ✅ Update CLAUDE.md with timeout configuration documentation

**Files to Modify** (in order):
1. `src/config.rs` - Add struct definitions and validation
2. `config/default.toml` - Add `[application.timeouts]` section
3. `src/adapters/serial_adapter.rs` - Replace 2 instances
4. `src/instrument/scpi_common.rs` - Replace 3 instances
5. `src/network/server_actor.rs` - Replace 10 instances
6. `src/instrument_manager_v3.rs` - Replace 4 instances
7. `src/experiment/primitives.rs` - Replace 2 instances (if applicable)
8. `tests/timeout_config_test.rs` - New test file

**Acceptance Criteria**:
- ✅ All 23 hardcoded timeouts replaced with config values
- ✅ Config loads with and without `[application.timeouts]` section
- ✅ Validation catches out-of-range values
- ✅ Existing tests pass
- ✅ New tests verify timeout behavior
- ✅ Documentation updated

### 4.2 Phase 2 Scope (Future)

**Goal**: Per-instrument timeout overrides

**When to Implement**: 
- After Phase 1 deployed and stable (1-2 sprints)
- If real-world need emerges (different instruments need different timeouts)
- If user requests this feature

**Time Estimate**: 3-4 hours (more complex inheritance logic)

**Deferred Rationale**:
- No evidence yet that instruments need different timeouts
- Adds significant complexity to config validation
- Can be added later without breaking Phase 1 configs
- Phase 1 solves 80% of use cases

---

## 5. Testing Strategy

### 5.1 Unit Tests

**Test Coverage**:

1. **Validation Tests**:
```rust
#[test]
fn test_timeout_too_short() {
    // Each timeout field tested with value below minimum
}

#[test]
fn test_timeout_too_long() {
    // Each timeout field tested with value above maximum
}

#[test]
fn test_timeout_valid_range() {
    // All timeout fields tested with valid values
}
```

2. **Backward Compatibility Tests**:
```rust
#[test]
fn test_missing_timeout_section_uses_defaults() {
    // Config without [application.timeouts] loads successfully
    // Verify default values are used
}

#[test]
fn test_partial_timeout_section() {
    // Config with only some timeout fields specified
    // Verify specified values used, missing fields use defaults
}
```

3. **Integration Tests**:
```rust
#[test]
fn test_serial_adapter_uses_config_timeout() {
    // Create SerialAdapter with custom timeout settings
    // Verify timeout applied to operations
}

#[test]
fn test_instrument_manager_uses_config_timeout() {
    // Create InstrumentManager with custom timeout settings
    // Verify connect/shutdown timeouts respected
}
```

### 5.2 Manual Testing

**Test Cases**:

1. **Default Config**: `cargo run --release` with default.toml unchanged
   - Expected: All timeouts use defaults (1s, 2s, 5s, etc.)

2. **Custom Config**: Edit `[application.timeouts]`, increase serial_read to 5s
   - Expected: Serial operations now timeout after 5s instead of 1s

3. **Invalid Config**: Set serial_read_timeout_ms = 50 (below minimum)
   - Expected: Config load fails with clear error message

4. **Missing Section**: Remove `[application.timeouts]` entirely
   - Expected: Application runs normally with default timeouts

### 5.3 Test Data

**Create test config files**:

```toml
# tests/configs/timeout_custom.toml
[application.timeouts]
serial_read_timeout_ms = 5000
scpi_command_timeout_ms = 10000
# ... other custom values ...

# tests/configs/timeout_invalid.toml
[application.timeouts]
serial_read_timeout_ms = 50  # Too short - should fail validation

# tests/configs/timeout_missing.toml
# No [application.timeouts] section - should use defaults
[application]
broadcast_channel_capacity = 1024
```

**Use in tests**:
```rust
#[test]
fn test_custom_config_loads() {
    let settings = Settings::new(Some("tests/configs/timeout_custom")).unwrap();
    assert_eq!(settings.application.timeouts.serial_read_timeout_ms, 5000);
}

#[test]
fn test_invalid_config_fails() {
    let result = Settings::new(Some("tests/configs/timeout_invalid"));
    assert!(result.is_err());
}
```

---

## 6. Documentation Requirements

### 6.1 Update CLAUDE.md

**Add Section**:
```markdown
### Timeout Configuration

The system uses configurable timeouts for all I/O and lifecycle operations. Timeouts are defined in `config/default.toml` under `[application.timeouts]`.

**Default Timeouts**:
- Serial I/O: 1000ms read/write
- SCPI commands: 2000ms
- Network operations: 5000ms client, 10000ms cleanup
- Instrument lifecycle: 5000ms connect, 6000ms shutdown, 5000ms measurement

**Customizing Timeouts**:
```toml
[application.timeouts]
serial_read_timeout_ms = 5000  # Increase for slow instruments
scpi_command_timeout_ms = 10000  # Increase for complex commands
```

**Validation Rules**:
- Serial I/O: 100ms - 30s
- Protocol: 500ms - 60s
- Network: 1s - 120s
- Instrument lifecycle: 1s - 60s

Invalid timeouts fail at config load time with clear error messages.
```

### 6.2 Update config/default.toml Comments

**Add Explanatory Comments**:
```toml
[application.timeouts]
# Serial I/O timeouts (milliseconds)
# Applied to serial port read/write operations for all serial instruments
# Typical range: 1000-5000ms (slower instruments need longer timeouts)
serial_read_timeout_ms = 1000
serial_write_timeout_ms = 1000

# Protocol timeouts (milliseconds)
# Applied to SCPI command/response cycles
# Increase for instruments with long processing times
scpi_command_timeout_ms = 2000

# Network timeouts (milliseconds)
# Applied to network client operations and cleanup
network_client_timeout_ms = 5000
network_cleanup_timeout_ms = 10000

# Instrument lifecycle timeouts (milliseconds)
# Applied to connection, shutdown, and measurement operations
# Increase for instruments with slow initialization or shutdown
instrument_connect_timeout_ms = 5000
instrument_shutdown_timeout_ms = 6000
instrument_measurement_timeout_ms = 5000
```

### 6.3 Create docs/TIMEOUT_TUNING.md (Optional)

**Guide for Users**:
```markdown
# Timeout Tuning Guide

## When to Increase Timeouts

1. **Serial I/O**: Instrument doesn't respond within 1s
2. **SCPI Commands**: Commands take >2s to execute
3. **Network**: High latency or slow network
4. **Instrument Init**: Hardware takes >5s to initialize

## Common Scenarios

### Slow Spectrometer
```toml
[application.timeouts]
instrument_measurement_timeout_ms = 30000  # 30s for long integrations
```

### Network Deployment Over VPN
```toml
[application.timeouts]
network_client_timeout_ms = 15000  # 15s for high latency
```

### Debug Mode (Prevent Timeout During Breakpoints)
```toml
[application.timeouts]
serial_read_timeout_ms = 60000  # 60s to inspect during debug
```
```

---

## 7. Design Validation Checklist

Before implementation begins, verify:

- ✅ Config structure follows existing `[application.*]` pattern
- ✅ Backward compatibility ensured via `#[serde(default)]`
- ✅ All 23 timeout instances categorized into 8 types
- ✅ Validation rules defined with min/max bounds
- ✅ Default values match current hardcoded values (no behavior change)
- ✅ Migration path documented for existing deployments
- ✅ Phase 1 scope clearly separated from Phase 2
- ✅ Testing strategy covers validation, backward compat, integration
- ✅ Documentation updates identified (CLAUDE.md, default.toml comments)

---

## 8. Acceptance Criteria

**This design document is complete when**:

- ✅ All key decisions documented (config structure, inheritance, categories)
- ✅ Backward compatibility strategy defined
- ✅ Migration plan covers existing deployments
- ✅ Phase 1 vs Phase 2 scope clearly separated
- ✅ Validation rules specified with min/max bounds
- ✅ Testing strategy defined (unit + integration + manual)
- ✅ Documentation requirements identified

**Ready for Implementation (bd-ltd3) when**:

- ✅ This design reviewed and approved
- ✅ No open questions about config structure
- ✅ Implementation tasks clearly defined
- ✅ Estimated 2-3 hours matches actual scope

---

## 9. Open Questions / Future Considerations

### 9.1 Resolved Questions

**Q**: Should we use `[application.timeouts]` or `[timeouts]`?  
**A**: `[application.timeouts]` - Matches existing pattern

**Q**: Should Phase 1 include per-instrument overrides?  
**A**: No - Adds complexity, can be added in Phase 2 if needed

**Q**: What validation bounds should we use?  
**A**: Permissive (100ms-60s range) to allow unusual hardware

**Q**: How to handle backward compatibility?  
**A**: `#[serde(default)]` on `TimeoutSettings` field

### 9.2 Deferred to Phase 2

- Per-instrument timeout overrides
- Timeout profiles (e.g., "debug", "production")
- Runtime timeout adjustment via GUI

### 9.3 Out of Scope

- Dynamic timeout adjustment based on observed latency
- Timeout auto-tuning via machine learning
- Per-command timeout overrides within SCPI

---

## 10. Summary

**This design establishes**:

1. ✅ **Config Location**: `[application.timeouts]` (nested under application)
2. ✅ **Phase 1 Scope**: Global defaults only (8 timeout types)
3. ✅ **Phase 2 Scope**: Per-instrument overrides (future)
4. ✅ **Backward Compatibility**: Defaults used if section missing
5. ✅ **Validation**: Min/max bounds with clear error messages
6. ✅ **Migration**: No breaking changes for existing deployments
7. ✅ **Testing**: Unit, integration, and manual test strategy
8. ✅ **Documentation**: CLAUDE.md and config.toml updates

**Next Steps**:

1. Review and approve this design document
2. Close bd-51b1 (design task)
3. Begin bd-ltd3 implementation (2-3 hours)
4. Verify all acceptance criteria met

**Design Status**: ✅ COMPLETE - Ready for Implementation

---

**Generated**: 2025-11-07  
**Task**: bd-51b1 (Design Phase)  
**Blocks**: bd-ltd3 (Implementation)  
**Estimated Implementation Time**: 2-3 hours (after design approval)
