//! Experiment Designer panel with node graph editor.

use egui_snarl::ui::SnarlStyle;
use egui_snarl::Snarl;

use crate::graph::{ExperimentNode, ExperimentViewer};
use crate::widgets::node_palette::{NodePalette, NodeType};

/// Panel for visual experiment design using a node graph editor.
pub struct ExperimentDesignerPanel {
    snarl: Snarl<ExperimentNode>,
    viewer: ExperimentViewer,
    style: SnarlStyle,
    /// Track if a node is being dragged from palette
    dragging_node: Option<NodeType>,
    /// Position to add context menu node
    context_menu_pos: Option<egui::Pos2>,
    /// Counter for generating unique node positions to avoid overlap
    node_count: usize,
}

impl Default for ExperimentDesignerPanel {
    fn default() -> Self {
        Self {
            snarl: Snarl::new(),
            viewer: ExperimentViewer::new(),
            style: SnarlStyle::default(),
            dragging_node: None,
            context_menu_pos: None,
            node_count: 0,
        }
    }
}

impl ExperimentDesignerPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Top toolbar
        ui.horizontal(|ui| {
            ui.label("Experiment Designer");
            ui.separator();

            // Show drag hint if dragging
            if let Some(node_type) = &self.dragging_node {
                ui.colored_label(node_type.color(), format!("Dragging: {}", node_type.name()));
            }
        });

        ui.separator();

        // Split into palette (left) and canvas (right)
        egui::SidePanel::left("node_palette_panel")
            .resizable(true)
            .default_width(180.0)
            .min_width(150.0)
            .max_width(300.0)
            .show_inside(ui, |ui| {
                // Check if drag started from palette
                if let Some(node_type) = NodePalette::show(ui) {
                    self.dragging_node = Some(node_type);
                }
            });

        // Main canvas area
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Handle context menu for adding nodes
            self.handle_context_menu(ui);

            // Handle drop onto canvas
            self.handle_canvas_drop(ui);

            // Graph canvas - takes remaining space
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let id = egui::Id::new("experiment_graph");
                self.snarl.show(&mut self.viewer, &self.style, id, ui);
            });
        });
    }

    fn handle_context_menu(&mut self, ui: &mut egui::Ui) {
        // Check for right-click to open context menu
        let response = ui.interact(
            ui.available_rect_before_wrap(),
            egui::Id::new("canvas_context"),
            egui::Sense::click(),
        );

        if response.secondary_clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.context_menu_pos = Some(pos);
            }
        }

        // Show context menu popup
        if let Some(pos) = self.context_menu_pos {
            let popup_id = egui::Id::new("add_node_menu");

            egui::Area::new(popup_id)
                .fixed_pos(pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(120.0);
                        ui.label("Add Node");
                        ui.separator();

                        let mut close_menu = false;

                        for node_type in NodeType::all() {
                            if ui.button(node_type.name()).clicked() {
                                // Add node at reasonable position
                                let node_pos = self.next_node_position();
                                let node = node_type.create_node();
                                self.snarl.insert_node(node_pos, node);
                                close_menu = true;
                            }
                        }

                        // Close menu on click outside or after adding node
                        if close_menu
                            || ui.input(|i| i.pointer.any_click())
                                && !ui.rect_contains_pointer(ui.min_rect())
                        {
                            self.context_menu_pos = None;
                        }
                    });
                });

            // Close menu when clicking elsewhere
            if ui.input(|i| {
                i.pointer.any_click() && i.pointer.hover_pos().is_some_and(|p| p != pos)
            }) {
                self.context_menu_pos = None;
            }
        }
    }

    fn handle_canvas_drop(&mut self, ui: &mut egui::Ui) {
        // Check if we're dragging and the mouse was released over the canvas
        if let Some(node_type) = self.dragging_node {
            let response = ui.interact(
                ui.available_rect_before_wrap(),
                egui::Id::new("canvas_drop_zone"),
                egui::Sense::hover(),
            );

            if response.hovered() {
                // Show drop indicator
                ui.painter().rect_stroke(
                    response.rect,
                    egui::CornerRadius::same(4),
                    egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE),
                    egui::StrokeKind::Inside,
                );
            }

            // Check if drag ended (mouse released)
            if !ui.input(|i| i.pointer.any_down()) {
                if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                    if response.rect.contains(pos) {
                        // Create node at drop position
                        // Note: We use a simple grid-based position since exact
                        // screen-to-graph conversion is complex. The user can
                        // reposition the node after dropping.
                        let node_pos = self.next_node_position();
                        let node = node_type.create_node();
                        self.snarl.insert_node(node_pos, node);
                    }
                }
                self.dragging_node = None;
            }
        }
    }

    /// Generate a position for the next node to avoid overlapping.
    fn next_node_position(&mut self) -> egui::Pos2 {
        let x = 50.0 + (self.node_count % 5) as f32 * 180.0;
        let y = 50.0 + (self.node_count / 5) as f32 * 120.0;
        self.node_count += 1;
        egui::pos2(x, y)
    }
}
