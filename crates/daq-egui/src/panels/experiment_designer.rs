//! Experiment Designer panel with node graph editor.

use egui_snarl::ui::SnarlStyle;
use egui_snarl::Snarl;

use crate::graph::{ExperimentNode, ExperimentViewer};

/// Panel for visual experiment design using a node graph editor.
pub struct ExperimentDesignerPanel {
    snarl: Snarl<ExperimentNode>,
    viewer: ExperimentViewer,
    style: SnarlStyle,
}

impl Default for ExperimentDesignerPanel {
    fn default() -> Self {
        Self {
            snarl: Snarl::new(),
            viewer: ExperimentViewer::new(),
            style: SnarlStyle::default(),
        }
    }
}

impl ExperimentDesignerPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Toolbar placeholder (will add buttons in later plans)
        ui.horizontal(|ui| {
            ui.label("Experiment Designer");
            ui.separator();
            ui.label("(Empty canvas - drag nodes from palette)");
        });

        ui.separator();

        // Graph canvas - takes remaining space
        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            let id = egui::Id::new("experiment_graph");
            self.snarl.show(&mut self.viewer, &self.style, id, ui);
        });
    }
}
