//! Demonstration of V4 Configuration System
//!
//! This example shows how to load and use the V4 configuration system.
//!
//! Run with:
//! ```bash
//! cargo run --example config_v4_demo
//! ```

use rust_daq::config_v4::V4Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Rust DAQ V4 Configuration Demo ===\n");

    // Load configuration from config.v4.toml
    println!("Loading configuration from config/config.v4.toml...");
    let config = V4Config::load()?;

    println!("✓ Configuration loaded successfully!\n");

    // Validate configuration
    println!("Validating configuration...");
    config.validate()?;
    println!("✓ Configuration is valid!\n");

    // Display configuration details
    println!("=== Application Settings ===");
    println!("Name: {}", config.application.name);
    println!("Log Level: {}", config.application.log_level);

    println!("\n=== Actor System Settings ===");
    println!("Mailbox Capacity: {}", config.actors.default_mailbox_capacity);
    println!("Spawn Timeout: {}ms", config.actors.spawn_timeout_ms);
    println!("Shutdown Timeout: {}ms", config.actors.shutdown_timeout_ms);

    println!("\n=== Storage Settings ===");
    println!("Backend: {}", config.storage.default_backend);
    println!("Output Directory: {}", config.storage.output_dir.display());
    println!("Compression Level: {}", config.storage.compression_level);
    println!("Auto-flush Interval: {}s", config.storage.auto_flush_interval_secs);

    println!("\n=== Instruments ===");
    let enabled_instruments = config.enabled_instruments();
    println!("Total: {}, Enabled: {}", config.instruments.len(), enabled_instruments.len());

    for instrument in enabled_instruments {
        println!("\n  ID: {}", instrument.id);
        println!("  Type: {}", instrument.r#type);
        println!("  Enabled: {}", instrument.enabled);
        println!("  Config: {:#?}", instrument.config);
    }

    println!("\n=== Environment Override Demo ===");
    println!("You can override configuration with environment variables:");
    println!("  RUST_DAQ_APPLICATION_LOG_LEVEL=debug cargo run --example config_v4_demo");
    println!("  RUST_DAQ_STORAGE_OUTPUT_DIR=/tmp/data cargo run --example config_v4_demo");

    Ok(())
}
