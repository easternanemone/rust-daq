//! DAQ Control Panel - egui desktop application
//!
//! A lightweight GUI for controlling the headless rust-daq daemon via gRPC.

#[cfg(feature = "standalone")]
mod app;
#[cfg(feature = "standalone")]
mod panels;
#[cfg(feature = "standalone")]
mod widgets;

// Re-export client from lib
use daq_egui::client;

#[cfg(feature = "standalone")]
use eframe::egui;

#[cfg(feature = "standalone")]
fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting DAQ Control Panel");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("DAQ Control Panel"),
        ..Default::default()
    };

    eframe::run_native(
        "DAQ Control Panel",
        options,
        Box::new(|cc| Ok(Box::new(app::DaqApp::new(cc)))),
    )
}

#[cfg(not(feature = "standalone"))]
fn main() {
    eprintln!("The daq-gui binary requires the 'standalone' feature (enabled by default).");
    std::process::exit(1);
}
