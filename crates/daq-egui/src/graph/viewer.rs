//! SnarlViewer implementation for ExperimentNode.

use egui_snarl::ui::SnarlViewer;
use egui_snarl::{InPin, OutPin, Snarl};

use super::nodes::ExperimentNode;

/// Viewer for rendering experiment nodes in the graph editor.
pub struct ExperimentViewer {
    // Will hold validation errors in later plans
}

impl ExperimentViewer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ExperimentViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl SnarlViewer<ExperimentNode> for ExperimentViewer {
    fn title(&mut self, node: &ExperimentNode) -> String {
        node.node_name().to_string()
    }

    fn inputs(&mut self, node: &ExperimentNode) -> usize {
        match node {
            ExperimentNode::Scan { .. } => 0, // Entry point, no inputs
            ExperimentNode::Loop { .. } => 1, // Has body input
            _ => 1,                            // Sequential flow input
        }
    }

    fn outputs(&mut self, node: &ExperimentNode) -> usize {
        match node {
            ExperimentNode::Loop { .. } => 2, // Next + loop body outputs
            _ => 1,                            // Sequential flow output
        }
    }

    fn show_input(
        &mut self,
        _pin: &InPin,
        ui: &mut egui::Ui,
        _snarl: &mut Snarl<ExperimentNode>,
    ) -> egui_snarl::ui::PinInfo {
        ui.label(">"); // Flow input indicator
        egui_snarl::ui::PinInfo::default()
    }

    fn show_output(
        &mut self,
        _pin: &OutPin,
        ui: &mut egui::Ui,
        _snarl: &mut Snarl<ExperimentNode>,
    ) -> egui_snarl::ui::PinInfo {
        ui.label(">"); // Flow output indicator
        egui_snarl::ui::PinInfo::default()
    }

    // show_body will be expanded in Plan 02-03 for property editing
}
