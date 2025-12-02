//! UI callback handlers
//!
//! This module contains all UI callback handlers organized by functionality.
//! Each handler module exposes a `register_*` function that wires up callbacks.

mod camera;
mod common;
mod connection;
mod data;
mod device;
mod experiment;
mod modules;
mod plugin;
mod preset;
mod scan;
mod toast;

use crate::state::SharedState;
use crate::ui::{ComponentHandle, MainWindow, UiAdapter};

/// Register all UI handlers
///
/// This is the main entry point for wiring up all UI callbacks.
/// Call this once after creating the MainWindow and SharedState.
pub fn register_all(ui: &MainWindow, state: SharedState) {
    let adapter = UiAdapter::new(ui.as_weak());

    connection::register(ui, adapter.clone(), state.clone());
    device::register(ui, adapter.clone(), state.clone());
    camera::register(ui, adapter.clone(), state.clone());
    scan::register(ui, adapter.clone(), state.clone());
    modules::register(ui, adapter.clone(), state.clone());
    preset::register(ui, adapter.clone(), state.clone());
    experiment::register(ui, adapter.clone(), state.clone());
    data::register(ui, adapter.clone(), state.clone());
    plugin::register(ui, adapter.clone(), state.clone());
    toast::register(ui, adapter);
}
