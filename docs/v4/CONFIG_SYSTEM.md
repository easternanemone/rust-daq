# V4 Configuration System

**Status**: ✅ Implemented (bd-rir3)
**Location**: `src/config_v4.rs`, `config/config.v4.toml`

## Overview

The V4 configuration system uses the [figment](https://docs.rs/figment/) library to provide:
- Strongly-typed configuration with compile-time safety
- Multiple configuration sources with layered priority
- Environment variable overrides
- Validation at load time
- Foundation for future hot-reloading

## Configuration Sources (Priority Order)

1. **Base Configuration**: `config/config.v4.toml` (lowest priority)
2. **Environment Variables**: `RUST_DAQ_*` prefix (highest priority)

Example environment override:
```bash
RUST_DAQ_APPLICATION_LOG_LEVEL=debug cargo run
RUST_DAQ_STORAGE_OUTPUT_DIR=/tmp/data cargo run
```

## Configuration File Structure

### config.v4.toml

```toml
[application]
name = "Rust DAQ V4"
log_level = "info"  # trace, debug, info, warn, error

[actors]
# Kameo actor system configuration
default_mailbox_capacity = 100
spawn_timeout_ms = 5000
shutdown_timeout_ms = 5000

[storage]
default_backend = "hdf5"  # arrow, hdf5, or both
output_dir = "data_output"
compression_level = 6  # 0-9
auto_flush_interval_secs = 30  # 0 = manual only

[[instruments]]
id = "mock_power_meter"
type = "MockPowerMeter"
enabled = true

[instruments.config]
sampling_rate_hz = 10.0
wavelength_nm = 532.0
```

## Rust API

### Loading Configuration

```rust
use rust_daq::config_v4::V4Config;

// Load from default location (config/config.v4.toml)
let config = V4Config::load()?;

// Load from custom path
let config = V4Config::load_from("path/to/config.toml")?;

// Validate after loading
config.validate()?;
```

### Accessing Configuration

```rust
// Application settings
println!("App: {}", config.application.name);
println!("Log level: {}", config.application.log_level);

// Actor system settings
let mailbox_capacity = config.actors.default_mailbox_capacity;
let spawn_timeout = config.actors.spawn_timeout_ms;

// Storage settings
let backend = &config.storage.default_backend;
let output_dir = &config.storage.output_dir;

// Instruments
for instrument in config.enabled_instruments() {
    println!("ID: {}, Type: {}", instrument.id, instrument.r#type);
    // Access instrument-specific config
    let sampling_rate = instrument.config["sampling_rate_hz"].as_float();
}
```

## Type Definitions

### V4Config

Top-level configuration container.

**Fields:**
- `application: ApplicationConfig` - Application-level settings
- `actors: ActorConfig` - Kameo actor system settings
- `storage: StorageConfig` - Storage backend settings
- `instruments: Vec<InstrumentDefinition>` - Instrument configurations

**Methods:**
- `load() -> Result<Self, figment::Error>` - Load from default location
- `load_from<P: AsRef<Path>>(path: P) -> Result<Self, figment::Error>` - Load from custom path
- `validate(&self) -> Result<(), String>` - Validate configuration
- `enabled_instruments(&self) -> Vec<&InstrumentDefinition>` - Get enabled instruments only

### ApplicationConfig

Application-level configuration.

**Fields:**
- `name: String` - Application name
- `log_level: String` - Logging level (trace, debug, info, warn, error)

### ActorConfig

Kameo actor system configuration.

**Fields:**
- `default_mailbox_capacity: usize` - Default mailbox size for actors (default: 100)
- `spawn_timeout_ms: u64` - Actor spawn timeout in milliseconds (default: 5000)
- `shutdown_timeout_ms: u64` - Actor shutdown timeout in milliseconds (default: 5000)

### StorageConfig

Storage backend configuration.

**Fields:**
- `default_backend: String` - Storage backend: "arrow", "hdf5", or "both"
- `output_dir: PathBuf` - Output directory for data files
- `compression_level: u8` - Compression level 0-9 (default: 6)
- `auto_flush_interval_secs: u64` - Auto-flush interval in seconds (0 = manual only)

### InstrumentDefinition

Individual instrument configuration.

**Fields:**
- `id: String` - Unique instrument identifier
- `r#type: String` - Instrument type (e.g., "MockPowerMeter", "Newport1830C")
- `enabled: bool` - Whether this instrument is enabled (default: true)
- `config: toml::Value` - Instrument-specific configuration (dynamic)

## Validation Rules

The `validate()` method enforces:

1. **Log Level**: Must be one of: trace, debug, info, warn, error
2. **Storage Backend**: Must be one of: arrow, hdf5, both
3. **Compression Level**: Must be 0-9
4. **Unique Instrument IDs**: All instrument IDs must be unique
5. **File Paths**: Storage output directory is created if it doesn't exist

## Usage Example

See `examples/config_v4_demo.rs` for a complete demonstration:

```bash
# Run the example
cargo run --example config_v4_demo

# With environment override
RUST_DAQ_APPLICATION_LOG_LEVEL=debug cargo run --example config_v4_demo
```

## Future Enhancements

### Hot-Reloading (Planned)

The configuration system is designed to support hot-reloading:

```rust
// Future API (not yet implemented)
let config = V4Config::load_with_watching()?;
config.on_change(|new_config| {
    // React to configuration changes
    apply_log_level(new_config.application.log_level);
    // Actors will need to handle reconfiguration messages
});
```

### Configuration Profiles (Planned)

Support for multiple configuration profiles:

```bash
# Development
cargo run -- --config config/dev.v4.toml

# Production
cargo run -- --config config/prod.v4.toml

# Testing
cargo run -- --config config/test.v4.toml
```

## Migration from V1/V2/V3

V4 configuration is a clean break from the old `config` crate-based system:

**Old (V1/V2/V3):**
- Used `config` crate with `config::Settings`
- Configuration in `config/default.toml`
- Mixed V1/V2/V3 instrument definitions

**New (V4):**
- Uses `figment` for layered configuration
- Configuration in `config/config.v4.toml`
- Simplified, unified instrument definitions
- Environment variable support out of the box

## Testing

Unit tests in `src/config_v4.rs`:
- `test_load_config()` - Loads and validates default configuration
- `test_config_validation()` - Tests validation logic
- `test_invalid_log_level()` - Tests error handling for invalid log levels
- `test_duplicate_instrument_ids()` - Tests duplicate ID detection

Run tests:
```bash
cargo test config_v4
```

## Architecture Integration

The V4 configuration system integrates with:
- **Kameo Actors**: Actor system settings control mailbox sizes and timeouts
- **Tracing**: Log level configuration drives tracing subscriber setup
- **Apache Arrow/HDF5**: Storage settings configure data persistence
- **Instrument Actors**: Instrument definitions spawn corresponding actor instances

## Related Issues

- **bd-rir3**: Implement figment-based Configuration System ✅ (this document)
- **bd-fxb7**: Initialize Tracing Infrastructure (next, will use log_level setting)
- **bd-662d**: Create V4 Core Crate (will consume this configuration system)

## References

- [Figment Documentation](https://docs.rs/figment/)
- [TOML Specification](https://toml.io/)
- [Kameo Actor Framework](https://docs.rs/kameo/)
