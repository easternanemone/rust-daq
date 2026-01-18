//! Device-specific control panel widgets.
//!
//! This module provides specialized control panels for different device types,
//! including lasers, power meters, rotators, and stages.

mod maitai_panel;
mod power_meter_panel;
mod rotator_panel;
mod stage_panel;

pub use maitai_panel::MaiTaiControlPanel;
pub use power_meter_panel::PowerMeterControlPanel;
pub use rotator_panel::RotatorControlPanel;
pub use stage_panel::StageControlPanel;

use egui::Ui;
use tokio::runtime::Runtime;

use crate::client::DaqClient;
use daq_proto::daq::DeviceInfo;

/// Trait for device-specific control panel widgets
#[allow(dead_code)] // May be used for future docking support
pub trait DeviceControlWidget {
    /// Render the control panel UI
    ///
    /// # Arguments
    /// * `ui` - egui UI context
    /// * `device` - Device info from the daemon
    /// * `client` - Optional gRPC client for making requests
    /// * `runtime` - Tokio runtime for async operations
    fn ui(
        &mut self,
        ui: &mut Ui,
        device: &DeviceInfo,
        client: Option<&mut DaqClient>,
        runtime: &Runtime,
    );

    /// Return the device type this widget handles
    fn device_type(&self) -> &'static str;
}

/// Factory function to create the appropriate control panel for a device
#[allow(dead_code)] // May be used for future docking support
pub fn create_control_panel(device: &DeviceInfo) -> Box<dyn DeviceControlWidget> {
    let driver_lower = device.driver_type.to_lowercase();

    // Check specific driver types first
    if driver_lower.contains("maitai") || driver_lower.contains("mai_tai") {
        return Box::new(MaiTaiControlPanel::default());
    }

    if driver_lower.contains("1830") || driver_lower.contains("power_meter") {
        return Box::new(PowerMeterControlPanel::default());
    }

    if driver_lower.contains("ell14") || driver_lower.contains("thorlabs") {
        return Box::new(RotatorControlPanel::default());
    }

    if driver_lower.contains("esp300") || driver_lower.contains("newport") && device.is_movable {
        return Box::new(StageControlPanel::default());
    }

    // Fall back to capabilities
    if device.is_wavelength_tunable && device.is_emission_controllable {
        return Box::new(MaiTaiControlPanel::default());
    }

    if device.is_readable && !device.is_movable && !device.is_frame_producer {
        return Box::new(PowerMeterControlPanel::default());
    }

    if device.is_movable {
        // Check if rotator-like (angle-based) vs stage (position-based)
        // For now, default to stage
        return Box::new(StageControlPanel::default());
    }

    // Generic fallback - use stage panel with limited features
    Box::new(StageControlPanel::default())
}
