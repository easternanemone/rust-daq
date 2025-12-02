//! rust-daq GUI Application
//!
//! Slint-based GUI for remote control of the rust-daq daemon via gRPC.
//!
//! This module provides the application entry point. The application is
//! organized into:
//!
//! - `ui` - Slint UI types and re-exports
//! - `state` - Application state management
//! - `services` - gRPC client and service abstractions
//! - `handlers` - UI callback handlers

mod handlers;
mod services;
mod state;
mod ui;

use anyhow::Result;
use state::{AppState, SharedState};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use ui::{ComponentHandle, MainWindow, UiAdapter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rust_daq_gui=info".parse().unwrap())
                .add_directive("tonic=warn".parse().unwrap()),
        )
        .init();

    info!("Starting rust-daq GUI");

    // Create the UI
    let ui = MainWindow::new()?;

    // Shared state
    let state: SharedState = Arc::new(Mutex::new(AppState::new()));

    // Create UI adapter and initialize models
    let adapter = UiAdapter::new(ui.as_weak());
    adapter.reset_all_models();

    // Register all UI handlers
    handlers::register_all(&ui, state);

    // Run the UI
    info!("GUI ready, running event loop");
    ui.run()?;

    info!("GUI closed");
    Ok(())
}
