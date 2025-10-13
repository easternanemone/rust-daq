//! Application entry point.
use anyhow::Result;
use eframe::NativeOptions;
use log::info;
use rust_daq::{
    app::DaqApp,
    config::Settings,
    gui::Gui,
    instrument::{mock::MockInstrument, scpi::ScpiInstrument, InstrumentRegistry},
};
use std::sync::Arc;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Load configuration
    let settings = Arc::new(Settings::new()?);
    info!("Configuration loaded successfully.");

    // Create a registry and register available instruments.
    // This is our static "plugin" system.
    let mut instrument_registry = InstrumentRegistry::new();
    instrument_registry.register("mock", |_id| Box::new(MockInstrument::new()));
    instrument_registry.register("scpi_keithley", |_id| Box::new(ScpiInstrument::new()));

    #[cfg(feature = "instrument_visa")]
    {
        use rust_daq::instrument::visa::VisaInstrument;
        // The unwrap here is not ideal, but the factory function doesn't
        // support returning a Result. If creating a VISA instrument fails,
        // the application will panic, which is acceptable for now.
        instrument_registry.register("visa_instrument", |id| {
            Box::new(VisaInstrument::new(id).unwrap())
        });
    }

    let instrument_registry = Arc::new(instrument_registry);

    // Create the core application state
    let app = DaqApp::new(settings.clone(), instrument_registry)?;
    let app_clone = app.clone();

    // Set up and run the GUI
    let options = NativeOptions::default();
    info!("Starting GUI...");

    eframe::run_native(
        "Rust DAQ",
        options,
        Box::new(move |cc| {
            // The `eframe` crate provides the `egui` context `cc`
            // which we can use to style the GUI.
            // Here we are just passing it to our `Gui` struct.
            Box::new(Gui::new(cc, app_clone))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Eframe run error: {}", e))?;

    // The GUI event loop has finished.
    // We can now gracefully shut down the application.
    info!("GUI closed. Shutting down.");
    app.shutdown();

    Ok(())
}
