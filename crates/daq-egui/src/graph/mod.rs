//! Node graph editor module for experiment design.

pub mod commands;
pub mod nodes;
pub mod validation;
pub mod viewer;

pub use commands::{AddNode, ConnectNodes, DisconnectNodes, GraphTarget, ModifyNode, RemoveNode};
pub use nodes::ExperimentNode;
pub use validation::{input_pin_type, output_pin_type, validate_connection, PinType};
pub use viewer::ExperimentViewer;

// Re-export Snarl for convenience
#[allow(unused_imports)]
pub use egui_snarl::Snarl;
