//! Reusable UI widgets for the DAQ GUI.
//!
//! This module contains parameter editors and other UI components
//! that can be shared across different panels.

pub mod auto_scale_plot;
pub mod device_controls;
pub mod device_selector;
pub mod gauge;
pub mod histogram;
pub mod metadata_editor;
pub mod node_palette;
pub mod offline_notice;
pub mod parameter_editor;
pub mod pp_editor;
pub mod property_inspector;
pub mod roi_selector;
pub mod runtime_parameter_editor;
pub mod smart_stream_editor;
pub mod status_bar;
pub mod toast;
pub mod toggle;

pub use auto_scale_plot::{AutoScalePlot, AxisLockState};
pub use device_controls::{
    DeviceControlWidget, MaiTaiControlPanel, PowerMeterControlPanel, RotatorControlPanel,
    StageControlPanel,
};
pub use device_selector::DeviceSelector;
pub use gauge::*;
pub use histogram::*;
pub use metadata_editor::MetadataEditor;
pub use node_palette::{NodePalette, NodeType};
pub use offline_notice::*;
pub use property_inspector::PropertyInspector;
pub use parameter_editor::*;
pub use runtime_parameter_editor::{EditableParameter, ParameterType, RuntimeParameterEditResult, RuntimeParameterEditor};
pub use pp_editor::*;
pub use roi_selector::*;
pub use smart_stream_editor::*;
pub use status_bar::*;
#[allow(unused_imports)]
pub use toast::*;
