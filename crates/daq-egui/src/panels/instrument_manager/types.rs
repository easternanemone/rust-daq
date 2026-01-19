//! Type definitions for the Instrument Manager Panel.

use daq_proto::daq::DeviceInfo;

/// Device category for grouping in the tree view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)] // All variants defined for completeness
pub enum DeviceCategory {
    Camera,
    Stage,
    Detector,
    Laser,
    PowerMeter,
    Other,
}

impl DeviceCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Camera => "Cameras",
            Self::Stage => "Stages",
            Self::Detector => "Detectors",
            Self::Laser => "Lasers",
            Self::PowerMeter => "Power Meters",
            Self::Other => "Other",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Camera => "📷",
            Self::Stage => "🔄",
            Self::Detector => "📊",
            Self::Laser => "🔴",
            Self::PowerMeter => "⚡",
            Self::Other => "🔧",
        }
    }

    /// Infer category from device capabilities
    pub fn from_device_info(info: &DeviceInfo) -> Self {
        if info.is_frame_producer {
            Self::Camera
        } else if info.is_movable {
            Self::Stage
        } else if info.is_readable {
            // Could be detector or power meter - check driver name
            if info.driver_type.to_lowercase().contains("power") {
                Self::PowerMeter
            } else {
                Self::Detector
            }
        } else {
            Self::Other
        }
    }
}

/// Grouped devices for tree display
#[derive(Clone)]
pub struct DeviceGroup {
    pub category: DeviceCategory,
    pub devices: Vec<DeviceInfo>,
    pub expanded: bool,
}

/// Parameter with current value for display
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub name: String,
    #[allow(dead_code)]
    pub description: String,
    pub dtype: String,
    pub units: String,
    #[allow(dead_code)]
    pub readable: bool,
    pub writable: bool,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub enum_values: Vec<String>,
    pub current_value: Option<String>,
}

/// Request to pop out a device control panel into a dockable window
#[derive(Debug, Clone)]
pub struct PopOutRequest {
    /// Full device info with capability flags
    pub device_info: DeviceInfo,
}
