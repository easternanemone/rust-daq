//! Application entry point for the native GUI.
//!
//! This file is the executable entry point for the `rust_daq` application. Its primary
//! responsibilities are:
//!
//! 1.  **Initialization**: It sets up the application's core components, including:
//!     -   **Logging**: Configures a multi-backend logger (`multi_log`) that directs log
//!         messages to both the standard console (via `env_logger`) and a custom in-memory
//!         buffer for display in the GUI (`log_capture::LogCollector`). This provides
//!         both persistent and interactive logging.
//!     -   **Configuration**: Loads the `settings.toml` file into the `Settings` struct.
//!     -   **Instrument Registry V2**: Creates an `InstrumentRegistryV2` and registers all the
//!         available V2 instrument drivers. This acts as a simple "plugin" system where
//!         instrument types are mapped to their constructor functions. Feature flags
//!         are used to conditionally compile and register drivers that have specific
//!         dependencies.
//!     -   **Processor Registry**: Creates a `ProcessorRegistry` for data processing modules.
//!     -   **Actor System**: Creates DaqManagerActor and spawns it in async task.
//!
//! 2.  **GUI Launch**: It configures and runs the native GUI using the `eframe` crate.
//!     It passes the command channel and runtime handle to the `gui::Gui` struct, which then
//!     takes control of the main event loop.
//!
//! 3.  **Shutdown**: After the `eframe` event loop exits (i.e., the user closes the window),
//!     it sends a shutdown command to the actor for graceful termination.

use anyhow::Result;
use eframe::NativeOptions;
use log::{info, LevelFilter};
#[cfg(feature = "instrument_serial")]
use rust_daq::instruments_v2::{
    elliptec::ElliptecV2, esp300::ESP300V2, maitai::MaiTaiV2, newport_1830c::Newport1830CV2,
};
use rust_daq::{
    app_actor::DaqManagerActor,
    config::Settings,
    data::registry::ProcessorRegistry,
    gui::Gui,
    instrument::{v2_adapter::V2InstrumentAdapter, InstrumentRegistry, InstrumentRegistryV2},
    instruments_v2::{
        mock_instrument::MockInstrumentV2, pvcam::PVCAMInstrumentV2, scpi::ScpiInstrumentV2,
    },
    log_capture::{LogBuffer, LogCollector},
    measurement::InstrumentMeasurement,
    messages::DaqCommand,
};
use std::sync::Arc;

use tokio::sync::mpsc;

fn main() -> Result<()> {
    // --- Custom Log Initialization ---
    // Create a shared buffer for log messages
    let log_buffer = LogBuffer::new();

    // Create a logger that collects messages for the GUI
    let gui_logger = LogCollector::new(log_buffer.clone());

    // Get the desired log level from the environment or default to "info"
    let log_level_filter = std::env::var("RUST_LOG").map_or(LevelFilter::Info, |s| {
        s.parse().unwrap_or(LevelFilter::Info)
    });

    // Create a logger that prints to the console
    let console_logger = env_logger::Builder::new()
        .filter_level(log_level_filter)
        .build();

    // Combine the loggers. All log messages will be sent to both the
    // console and our GUI log collector.
    log::set_max_level(log_level_filter);
    multi_log::MultiLogger::init(
        vec![Box::new(console_logger), Box::new(gui_logger)],
        log_level_filter.to_level().unwrap_or(log::Level::Info),
    )
    .map_err(|e| anyhow::anyhow!("Failed to initialize logger: {}", e))?;
    // --- End of Log Initialization ---

    // Load configuration
    let settings = Settings::new(None)?;
    info!("Configuration loaded successfully.");

    // Create V1 and V2 instrument registries. V1 registry is backed by adapters so
    // legacy APIs continue to list the available instruments while Phase 3 proceeds.
    let mut instrument_registry = InstrumentRegistry::<InstrumentMeasurement>::new();
    let mut instrument_registry_v2 = InstrumentRegistryV2::new();

    // Register core V2 instruments (no feature gates)
    instrument_registry_v2.register("mock", |id| Box::pin(MockInstrumentV2::new(id.to_string())));
    instrument_registry.register("mock", |id| {
        Box::new(V2InstrumentAdapter::new(MockInstrumentV2::new(
            id.to_string(),
        )))
    });

    instrument_registry_v2.register("scpi_keithley", |id| {
        Box::pin(ScpiInstrumentV2::new(
            id.to_string(),
            "GPIB0::5::INSTR".to_string(),
        ))
    });
    instrument_registry.register("scpi_keithley", |id| {
        Box::new(V2InstrumentAdapter::new(ScpiInstrumentV2::new(
            id.to_string(),
            "GPIB0::5::INSTR".to_string(),
        )))
    });

    instrument_registry_v2.register("pvcam", |id| {
        Box::pin(PVCAMInstrumentV2::new(id.to_string()))
    });
    instrument_registry.register("pvcam", |id| {
        Box::new(V2InstrumentAdapter::new(PVCAMInstrumentV2::new(
            id.to_string(),
        )))
    });

    // Register serial instruments (feature-gated)
    #[cfg(feature = "instrument_serial")]
    {
        instrument_registry_v2.register("newport_1830c", |id| {
            Box::pin(Newport1830CV2::new(
                id.to_string(),
                "/dev/ttyUSB0".to_string(),
                9600,
            ))
        });
        instrument_registry.register("newport_1830c", |id| {
            Box::new(V2InstrumentAdapter::new(Newport1830CV2::new(
                id.to_string(),
                "/dev/ttyUSB0".to_string(),
                9600,
            )))
        });
        instrument_registry_v2.register("maitai", |id| {
            Box::pin(MaiTaiV2::new(
                id.to_string(),
                "/dev/ttyUSB0".to_string(),
                9600,
            ))
        });
        instrument_registry.register("maitai", |id| {
            Box::new(V2InstrumentAdapter::new(MaiTaiV2::new(
                id.to_string(),
                "/dev/ttyUSB0".to_string(),
                9600,
            )))
        });
        instrument_registry_v2.register("elliptec", |id| Box::pin(ElliptecV2::new(id.to_string())));
        instrument_registry.register("elliptec", |id| {
            Box::new(V2InstrumentAdapter::new(ElliptecV2::new(id.to_string())))
        });
        instrument_registry_v2.register("esp300", |id| {
            Box::pin(ESP300V2::new(
                id.to_string(),
                "/dev/ttyUSB0".to_string(),
                19200,
                3,
            ))
        });
        instrument_registry.register("esp300", |id| {
            Box::new(V2InstrumentAdapter::new(ESP300V2::new(
                id.to_string(),
                "/dev/ttyUSB0".to_string(),
                19200,
                3,
            )))
        });
    }

    let instrument_registry = Arc::new(instrument_registry);
    let instrument_registry_v2 = Arc::new(instrument_registry_v2);

    // Create the processor registry
    let processor_registry = Arc::new(ProcessorRegistry::new());

    // Create the module registry and register modules
    let mut module_registry = rust_daq::modules::ModuleRegistry::new();
    module_registry.register("power_meter", |id| {
        Box::new(rust_daq::modules::power_meter::PowerMeterModule::<
            rust_daq::core::DataPoint,
        >::new(id))
    });
    let module_registry = Arc::new(module_registry);

    // Create Tokio runtime for async operations
    let runtime = Arc::new(
        tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Tokio runtime: {}", e))?,
    );

    // Create the DaqManagerActor with both legacy and V2 registries
    let actor = DaqManagerActor::new(
        settings.clone(),
        instrument_registry.clone(),
        instrument_registry_v2.clone(),
        processor_registry.clone(),
        module_registry.clone(),
        runtime.clone(),
    )?;

    // Create command channel
    let (command_tx, command_rx) = mpsc::channel(settings.application.command_channel_capacity);

    // Clone command_tx for shutdown later
    let shutdown_tx = command_tx.clone();

    // Spawn instruments from config before starting GUI
    let instrument_ids: Vec<String> = settings.instruments.keys().cloned().collect();

    // Spawn the actor task
    let runtime_clone = runtime.clone();
    runtime_clone.spawn(async move {
        actor.run(command_rx).await;
    });

    // Spawn configured instruments asynchronously (non-blocking)
    for id in instrument_ids {
        let cmd_tx = command_tx.clone();
        let id_clone = id.clone();
        runtime.spawn(async move {
            let (cmd, rx) = DaqCommand::spawn_instrument(id_clone.clone());
            if cmd_tx.send(cmd).await.is_ok() {
                if let Ok(result) = rx.await {
                    if let Err(e) = result {
                        log::error!("Failed to spawn instrument '{}': {}", id_clone, e);
                    }
                }
            }
        });
    }

    // Set up and run the GUI
    let options = NativeOptions::default();
    info!("Starting GUI...");

    eframe::run_native(
        "Rust DAQ",
        options,
        Box::new(move |cc| {
            Ok(Box::new(Gui::new(
                cc,
                command_tx,
                runtime,
                settings,
                instrument_registry_v2,
                log_buffer,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Eframe run error: {}", e))?;

    // The GUI event loop has finished.
    // Send shutdown command to actor for graceful shutdown.
    info!("GUI closed. Shutting down actor...");
    let (cmd, rx) = DaqCommand::shutdown();
    if shutdown_tx.blocking_send(cmd).is_ok() {
        let _ = rx.blocking_recv();
    }

    Ok(())
}
