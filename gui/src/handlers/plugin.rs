//! Plugin handlers
//!
//! Handles plugin device callbacks for dynamic UI rendering.
//! Part of GUI Phase bd-22si.6.2
//!
//! NOTE: This is an initial implementation. Full functionality requires
//! the PluginService gRPC backend to be wired up.

use crate::state::SharedState;
use crate::ui::{
    MainWindow, PluginDeviceInfo, PluginUiElementInfo, SharedString, UiAdapter, VecModel, Weak,
};
use std::rc::Rc;
use tracing::{info, warn};

/// Register plugin-related callbacks
pub fn register(ui: &MainWindow, adapter: UiAdapter, _state: SharedState) {
    let ui_weak = adapter.weak();
    register_refresh_plugins(ui, ui_weak.clone());
    register_slider_changed(ui, ui_weak.clone());
    register_readout_refresh(ui, ui_weak.clone());
    register_toggle_changed(ui, ui_weak.clone());
    register_action_triggered(ui, ui_weak.clone());
    register_dropdown_changed(ui, ui_weak);

    // Initialize with example plugin devices for development
    initialize_example_plugins(ui);
}

fn initialize_example_plugins(ui: &MainWindow) {
    // Provide example plugin devices for UI development/testing
    // In production, this would query the PluginService gRPC endpoint
    let example_plugins = vec![
        PluginDeviceInfo {
            device_id: SharedString::from("ell14-stage-01"),
            device_name: SharedString::from("Thorlabs ELL14 Rotator"),
            plugin_id: SharedString::from("thorlabs-ell14"),
            connected: true,
            mock_mode: true,
            has_readable: true,
            has_movable: true,
            has_settable: false,
            has_switchable: false,
            has_actionable: true,
            ui_elements: Rc::new(VecModel::from(vec![
                PluginUiElementInfo {
                    element_type: SharedString::from("slider"),
                    label: SharedString::from("Position"),
                    target: SharedString::from("position"),
                    source: SharedString::new(),
                    action: SharedString::new(),
                    current_value: 45.0,
                    current_string: SharedString::new(),
                    unit: SharedString::from("deg"),
                    min_value: 0.0,
                    max_value: 360.0,
                    options: Rc::new(VecModel::default()).into(),
                    is_loading: false,
                    child_count: 0,
                },
                PluginUiElementInfo {
                    element_type: SharedString::from("readout"),
                    label: SharedString::from("Current Position"),
                    target: SharedString::new(),
                    source: SharedString::from("position"),
                    action: SharedString::new(),
                    current_value: 45.0,
                    current_string: SharedString::new(),
                    unit: SharedString::from("deg"),
                    min_value: 0.0,
                    max_value: 360.0,
                    options: Rc::new(VecModel::default()).into(),
                    is_loading: false,
                    child_count: 0,
                },
                PluginUiElementInfo {
                    element_type: SharedString::from("button"),
                    label: SharedString::from("Home"),
                    target: SharedString::new(),
                    source: SharedString::new(),
                    action: SharedString::from("home"),
                    current_value: 0.0,
                    current_string: SharedString::new(),
                    unit: SharedString::new(),
                    min_value: 0.0,
                    max_value: 0.0,
                    options: Rc::new(VecModel::default()).into(),
                    is_loading: false,
                    child_count: 0,
                },
            ]))
            .into(),
        },
        PluginDeviceInfo {
            device_id: SharedString::from("power-meter-01"),
            device_name: SharedString::from("Newport 1830-C Power Meter"),
            plugin_id: SharedString::from("newport-1830c"),
            connected: true,
            mock_mode: true,
            has_readable: true,
            has_movable: false,
            has_settable: true,
            has_switchable: true,
            has_actionable: false,
            ui_elements: Rc::new(VecModel::from(vec![
                PluginUiElementInfo {
                    element_type: SharedString::from("readout"),
                    label: SharedString::from("Power"),
                    target: SharedString::new(),
                    source: SharedString::from("power"),
                    action: SharedString::new(),
                    current_value: 0.000_042,
                    current_string: SharedString::new(),
                    unit: SharedString::from("W"),
                    min_value: 0.0,
                    max_value: 100.0,
                    options: Rc::new(VecModel::default()).into(),
                    is_loading: false,
                    child_count: 0,
                },
                PluginUiElementInfo {
                    element_type: SharedString::from("dropdown"),
                    label: SharedString::from("Wavelength"),
                    target: SharedString::from("wavelength"),
                    source: SharedString::new(),
                    action: SharedString::new(),
                    current_value: 0.0,
                    current_string: SharedString::from("800nm"),
                    unit: SharedString::new(),
                    min_value: 0.0,
                    max_value: 0.0,
                    options: Rc::new(VecModel::from(vec![
                        SharedString::from("532nm"),
                        SharedString::from("633nm"),
                        SharedString::from("800nm"),
                        SharedString::from("1064nm"),
                    ]))
                    .into(),
                    is_loading: false,
                    child_count: 0,
                },
                PluginUiElementInfo {
                    element_type: SharedString::from("toggle"),
                    label: SharedString::from("Auto-Range"),
                    target: SharedString::from("auto_range"),
                    source: SharedString::new(),
                    action: SharedString::new(),
                    current_value: 1.0, // 1.0 = on
                    current_string: SharedString::new(),
                    unit: SharedString::new(),
                    min_value: 0.0,
                    max_value: 1.0,
                    options: Rc::new(VecModel::default()).into(),
                    is_loading: false,
                    child_count: 0,
                },
            ]))
            .into(),
        },
    ];

    let model = Rc::new(VecModel::from(example_plugins));
    ui.set_plugin_devices(model.into());
}

fn register_refresh_plugins(ui: &MainWindow, ui_weak: Weak<MainWindow>) {
    ui.on_refresh_plugins(move || {
        info!("Refreshing plugin devices");

        // TODO: In the future, this would query PluginService::ListPlugins
        // and PluginService::GetPluginInfo for each spawned device
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            // Re-initialize with example data for now
            initialize_example_plugins(&ui);

            ui.invoke_show_toast(
                SharedString::from("info"),
                SharedString::from("Plugins Refreshed"),
                SharedString::from("Loaded example plugin devices (stub)"),
            );
        });
    });
}

fn register_slider_changed(ui: &MainWindow, ui_weak: Weak<MainWindow>) {
    ui.on_plugin_slider_changed(move |device_id, target, value| {
        let device_id = device_id.to_string();
        let target = target.to_string();

        info!(
            "Plugin slider changed: device={}, target={}, value={}",
            device_id, target, value
        );

        // TODO: Call appropriate gRPC method based on capability
        // e.g., HardwareService::MoveAbsolute for movable, or a generic SetValue
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            warn!("Plugin slider is a stub - no actual change performed");

            ui.invoke_show_toast(
                SharedString::from("info"),
                SharedString::from("Slider Changed"),
                SharedString::from(format!(
                    "{}.{} = {:.3} (stub)",
                    device_id, target, value
                )),
            );
        });
    });
}

fn register_readout_refresh(ui: &MainWindow, ui_weak: Weak<MainWindow>) {
    ui.on_plugin_readout_refresh(move |device_id, source| {
        let device_id = device_id.to_string();
        let source = source.to_string();

        info!(
            "Plugin readout refresh: device={}, source={}",
            device_id, source
        );

        // TODO: Call HardwareService::ReadValue or similar
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            warn!("Plugin readout refresh is a stub - no actual read performed");

            ui.invoke_show_toast(
                SharedString::from("info"),
                SharedString::from("Readout Refreshed"),
                SharedString::from(format!("{}.{} (stub)", device_id, source)),
            );
        });
    });
}

fn register_toggle_changed(ui: &MainWindow, ui_weak: Weak<MainWindow>) {
    ui.on_plugin_toggle_changed(move |device_id, target, is_on| {
        let device_id = device_id.to_string();
        let target = target.to_string();

        info!(
            "Plugin toggle changed: device={}, target={}, is_on={}",
            device_id, target, is_on
        );

        // TODO: Call HardwareService::SetSwitch or similar
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            warn!("Plugin toggle is a stub - no actual change performed");

            ui.invoke_show_toast(
                SharedString::from("info"),
                SharedString::from("Toggle Changed"),
                SharedString::from(format!(
                    "{}.{} = {} (stub)",
                    device_id,
                    target,
                    if is_on { "ON" } else { "OFF" }
                )),
            );
        });
    });
}

fn register_action_triggered(ui: &MainWindow, ui_weak: Weak<MainWindow>) {
    ui.on_plugin_action_triggered(move |device_id, action| {
        let device_id = device_id.to_string();
        let action = action.to_string();

        info!(
            "Plugin action triggered: device={}, action={}",
            device_id, action
        );

        // TODO: Call HardwareService::ExecuteAction or similar
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            warn!("Plugin action is a stub - no actual action performed");

            ui.invoke_show_toast(
                SharedString::from("info"),
                SharedString::from("Action Triggered"),
                SharedString::from(format!("{}.{}() (stub)", device_id, action)),
            );
        });
    });
}

fn register_dropdown_changed(ui: &MainWindow, ui_weak: Weak<MainWindow>) {
    ui.on_plugin_dropdown_changed(move |device_id, target, value| {
        let device_id = device_id.to_string();
        let target = target.to_string();
        let value = value.to_string();

        info!(
            "Plugin dropdown changed: device={}, target={}, value={}",
            device_id, target, value
        );

        // TODO: Call appropriate setter method
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            warn!("Plugin dropdown is a stub - no actual change performed");

            ui.invoke_show_toast(
                SharedString::from("info"),
                SharedString::from("Selection Changed"),
                SharedString::from(format!("{}.{} = {} (stub)", device_id, target, value)),
            );
        });
    });
}
