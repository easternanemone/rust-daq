// power_logger.rhai - Rhai Script Plugin Example
//
// This module demonstrates the script-based plugin interface for rust_daq.
// It implements a power logging module that reads from a power meter device
// and logs readings with timestamps.
//
// KEY CONCEPTS DEMONSTRATED:
// 1. Module lifecycle: stage() -> start() -> pause()/resume() -> stop() -> unstage()
// 2. Role-based device binding: requires a device with "readable" capability
// 3. Parameter configuration: log_interval_ms, log_format, output_path
// 4. Data emission: power readings can be emitted to the storage layer
// 5. Hot-reload: script changes take effect without recompiling the host
//
// USAGE:
//   The plugin loader discovers this script via plugin.toml and creates
//   module instances on demand:
//
//   let registry = PluginRegistry::new();
//   registry.add_search_path("./examples/plugins/");
//   registry.scan();
//   let module = registry.create_module("power_logger").await?;
//   module.configure(params)?;
//   module.stage()?;
//   module.start()?;

// =============================================================================
// Module Type Information
// =============================================================================
//
// Returns metadata describing this module type. This is called by the plugin
// loader to understand what the module does and what it requires.
//
// The returned map includes:
// - type_id: Unique identifier for this module type
// - display_name: Human-readable name for UI display
// - description: What the module does
// - version: Module version (should match plugin.toml)
// - parameters: Configuration parameters the module accepts
// - required_roles: Device roles the module needs (bound at configuration time)
// - optional_roles: Optional device roles
// - event_types: Events this module can emit
// - data_types: Data schemas this module produces

fn module_type_info() {
    #{
        type_id: "power_logger",
        display_name: "Power Logger",
        description: "Logs power readings with timestamps. Demonstrates script-based modules.",
        version: "1.0.0",

        // Parameters that can be configured before staging
        parameters: [
            #{
                param_id: "log_interval_ms",
                display_name: "Log Interval",
                description: "Time between log entries in milliseconds",
                param_type: "integer",
                default_value: "1000",
                min_value: "100",
                max_value: "60000",
                units: "ms",
                required: false
            },
            #{
                param_id: "log_format",
                display_name: "Log Format",
                description: "Format for log output",
                param_type: "enum",
                default_value: "simple",
                enum_values: ["simple", "verbose", "csv"],
                required: false
            },
            #{
                param_id: "output_path",
                display_name: "Output Path",
                description: "File path for CSV output",
                param_type: "string",
                default_value: "/tmp/power_log.csv",
                required: false
            }
        ],

        // Required device roles - must be bound before staging
        required_roles: [
            #{
                role_id: "power_source",
                display_name: "Power Source",
                description: "Device providing power readings",
                required_capability: "readable",
                allows_multiple: false
            }
        ],

        // Optional roles (none for this simple example)
        optional_roles: [],

        // Events this module can emit
        event_types: ["log_started", "log_stopped", "power_reading", "error"],

        // Data types this module produces
        data_types: ["power_log"]
    }
}

// =============================================================================
// Module State
// =============================================================================
//
// Script-level state persists across function calls within a module instance.
// This state is isolated per module instance.

// Configuration storage (set during configure())
let config = #{
    log_interval_ms: 1000,
    log_format: "simple",
    output_path: "/tmp/power_log.csv"
};

// Runtime state
let state = #{
    is_staged: false,
    is_running: false,
    is_paused: false,
    log_count: 0,
    last_reading: 0.0,
    start_time: 0,
    total_energy_mwh: 0.0
};

// Bound devices (set during stage())
let devices = #{
    power_source: ()
};

// =============================================================================
// Configuration Functions
// =============================================================================

/// Configures the module with the given parameters.
/// Called before stage() to set up module behavior.
///
/// # Arguments
/// * params - Map of parameter_id -> value
///
/// # Returns
/// Array of warning messages (empty if no warnings)
fn configure(params) {
    let warnings = [];

    // Validate and apply log_interval_ms
    if "log_interval_ms" in params {
        let interval = parse_int(params["log_interval_ms"]);
        if interval >= 100 && interval <= 60000 {
            config.log_interval_ms = interval;
        } else {
            warnings.push(`log_interval_ms clamped to valid range [100, 60000]`);
            config.log_interval_ms = if interval < 100 { 100 } else { 60000 };
        }
    }

    // Validate and apply log_format
    if "log_format" in params {
        let format = params["log_format"];
        if format == "simple" || format == "verbose" || format == "csv" {
            config.log_format = format;
        } else {
            warnings.push(`Invalid log_format '${format}', using 'simple'`);
            config.log_format = "simple";
        }
    }

    // Apply output_path
    if "output_path" in params {
        config.output_path = params["output_path"];
    }

    warnings
}

/// Returns the current configuration as a map.
fn get_config() {
    #{
        log_interval_ms: `${config.log_interval_ms}`,
        log_format: config.log_format,
        output_path: config.output_path
    }
}

// =============================================================================
// Module Lifecycle Functions
// =============================================================================

/// Stage: Prepare resources and validate device bindings.
///
/// Called after configure() but before start(). This is where the module:
/// - Validates that required devices are bound
/// - Opens file handles
/// - Allocates buffers
/// - Performs any one-time setup
///
/// # Arguments
/// * ctx - Context object containing:
///   - module_id: Unique instance identifier
///   - devices: Map of role_id -> device handle
///
/// # Returns
/// () on success, throws on error
fn stage(ctx) {
    print(`[${ctx.module_id}] Staging power logger...`);

    // Validate required device bindings
    if "devices" in ctx && "power_source" in ctx.devices {
        devices.power_source = ctx.devices["power_source"];
        print(`[${ctx.module_id}] Bound power source device`);
    } else {
        print(`[${ctx.module_id}] Warning: No power source device bound (will use mock data)`);
    }

    // Reset runtime state
    state.log_count = 0;
    state.last_reading = 0.0;
    state.total_energy_mwh = 0.0;
    state.is_staged = true;
    state.is_running = false;
    state.is_paused = false;

    // If using CSV format, write header
    if config.log_format == "csv" {
        print("timestamp,module_id,power_mW,energy_mWh");
    }

    print(`[${ctx.module_id}] Staged successfully`);
}

/// Start: Begin module execution.
///
/// Called after stage(). The module should begin its main operation:
/// - Start polling devices
/// - Begin logging
/// - Emit "started" event
///
/// # Arguments
/// * ctx - Context object (same as stage)
fn start(ctx) {
    if !state.is_staged {
        throw `Cannot start: module not staged`;
    }

    print(`[${ctx.module_id}] Starting power logger with interval ${config.log_interval_ms}ms`);

    state.is_running = true;
    state.is_paused = false;
    state.start_time = timestamp();

    // Emit started event
    emit_event(ctx.module_id, "log_started", #{
        interval_ms: config.log_interval_ms,
        format: config.log_format,
        output_path: config.output_path
    });

    // Log initial reading
    let reading = read_power();
    log_reading(ctx.module_id, reading);

    print(`[${ctx.module_id}] Power logging started`);
}

/// Pause: Temporarily suspend execution.
///
/// The module should stop polling but retain state for resume.
fn pause() {
    if !state.is_running {
        return;
    }

    print("Power logger paused");
    state.is_paused = true;
}

/// Resume: Continue execution after pause.
fn resume() {
    if !state.is_running || !state.is_paused {
        return;
    }

    print("Power logger resumed");
    state.is_paused = false;
}

/// Stop: End module execution.
///
/// The module should:
/// - Stop all polling
/// - Flush any pending data
/// - Emit "stopped" event
/// - Retain state for potential restart
fn stop() {
    if !state.is_running {
        return;
    }

    let duration_ms = timestamp() - state.start_time;

    print(`Power logger stopped. Total logs: ${state.log_count}, Duration: ${duration_ms}ms`);
    print(`Total energy logged: ${state.total_energy_mwh} mWh`);

    state.is_running = false;
    state.is_paused = false;
}

/// Unstage: Release all resources.
///
/// Called after stop(). The module should:
/// - Close file handles
/// - Release device bindings
/// - Free any allocated resources
/// - Reset to pre-staged state
///
/// # Arguments
/// * ctx - Context object
fn unstage(ctx) {
    print(`[${ctx.module_id}] Unstaging power logger...`);

    // Release device bindings
    devices.power_source = ();

    // Reset state
    state.is_staged = false;
    state.is_running = false;
    state.is_paused = false;

    print(`[${ctx.module_id}] Unstaged successfully`);
}

// =============================================================================
// Data Acquisition Functions
// =============================================================================

/// Reads power value from the bound device or generates mock data.
fn read_power() {
    // In a real implementation, this would call the device API:
    // let value = devices.power_source.read("power_mW");

    // For demonstration, generate mock data with some variation
    let base_power = 42.5;  // mW
    let noise = (timestamp() % 100) / 100.0 * 2.0 - 1.0;  // +/- 1 mW
    base_power + noise
}

/// Logs a power reading in the configured format.
fn log_reading(module_id, power_mw) {
    let ts = timestamp();
    state.log_count += 1;
    state.last_reading = power_mw;

    // Calculate energy (simple integration)
    let interval_hours = config.log_interval_ms / 3600000.0;
    state.total_energy_mwh += power_mw * interval_hours;

    // Format and print the log entry
    let entry = format_log_entry(module_id, power_mw, ts);
    print(entry);

    // Emit data for storage layer
    emit_data("power_log", #{
        timestamp: ts,
        module_id: module_id,
        power_mw: power_mw,
        energy_mwh: state.total_energy_mwh,
        log_number: state.log_count
    });
}

/// Formats a log entry based on the configured format.
fn format_log_entry(module_id, power, ts) {
    if config.log_format == "simple" {
        `Power: ${power} mW`
    } else if config.log_format == "verbose" {
        `[${ts}] Module ${module_id}: Power reading #${state.log_count} = ${power} mW (total: ${state.total_energy_mwh} mWh)`
    } else if config.log_format == "csv" {
        `${ts},${module_id},${power},${state.total_energy_mwh}`
    } else {
        `Power: ${power} mW`
    }
}

// =============================================================================
// Event and Data Emission (Stubs)
// =============================================================================
//
// These functions would be provided by the host runtime in a real implementation.
// For now, they're placeholders that demonstrate the intended API.

/// Emits an event to the event bus.
fn emit_event(module_id, event_type, data) {
    // In production: host.emit_event(module_id, event_type, data)
    print(`[EVENT] ${module_id}:${event_type} -> ${data}`);
}

/// Emits data to the storage layer.
fn emit_data(data_type, record) {
    // In production: host.emit_data(data_type, record)
    // The storage layer would write this to Parquet, CSV, etc.
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Returns the current timestamp in milliseconds.
/// Note: This would be provided by the host in production.
fn timestamp() {
    // Placeholder - in production this comes from the host runtime
    0
}

// =============================================================================
// Hot-Reload Support
// =============================================================================
//
// Script plugins support hot-reload:
// 1. Modify this file while the module is running
// 2. The runtime detects the change and reloads the script
// 3. Module state is preserved, but function implementations update
// 4. No recompilation required
//
// This enables rapid iteration during experiment development.
