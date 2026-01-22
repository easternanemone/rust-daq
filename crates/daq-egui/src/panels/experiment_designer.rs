//! Experiment Designer panel with node graph editor.

use egui_snarl::ui::{get_selected_nodes, SnarlStyle};
use egui_snarl::{NodeId, Snarl};
use undo::Record;

use crate::graph::commands::{AddNodeData, GraphEdit, ModifyNodeData};
use crate::graph::{ExperimentNode, ExperimentViewer};
use crate::widgets::node_palette::{NodePalette, NodeType};
use crate::widgets::PropertyInspector;

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
    /// Undo/redo history
    history: Record<GraphEdit>,
    /// Cache of selected node ID (updated from egui-snarl state)
    selected_node: Option<NodeId>,
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
            history: Record::new(),
            selected_node: None,
        }
    }
}

impl ExperimentDesignerPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Handle keyboard shortcuts FIRST (before any UI that might consume keys)
        self.handle_keyboard(ui);

        // Update selected node from egui-snarl state
        self.update_selected_node(ui);

        // Top toolbar with undo/redo buttons
        ui.horizontal(|ui| {
            ui.label("Experiment Designer");
            ui.separator();

            // Undo button
            let can_undo = self.history.can_undo();
            if ui
                .add_enabled(can_undo, egui::Button::new("Undo"))
                .on_hover_text("Ctrl+Z")
                .clicked()
            {
                self.undo();
            }

            // Redo button
            let can_redo = self.history.can_redo();
            if ui
                .add_enabled(can_redo, egui::Button::new("Redo"))
                .on_hover_text("Ctrl+Y or Ctrl+Shift+Z")
                .clicked()
            {
                self.redo();
            }

            ui.separator();

            // Show undo/redo stack info
            ui.label(format!(
                "History: {}/{}",
                self.history.head(),
                self.history.len()
            ));

            ui.separator();

            // Show drag hint if dragging
            if let Some(node_type) = &self.dragging_node {
                ui.colored_label(node_type.color(), format!("Dragging: {}", node_type.name()));
            }
        });

        ui.separator();

        // Three-panel layout: Palette | Canvas | Inspector
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

        egui::SidePanel::right("property_inspector_panel")
            .resizable(true)
            .default_width(200.0)
            .min_width(150.0)
            .max_width(400.0)
            .show_inside(ui, |ui| {
                self.show_property_inspector(ui);
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

    fn show_property_inspector(&mut self, ui: &mut egui::Ui) {
        ui.heading("Properties");
        ui.separator();

        if let Some(node_id) = self.selected_node {
            if let Some(node) = self.snarl.get_node(node_id) {
                // Clone the node before passing to inspector
                let node_clone = node.clone();

                // Show inspector and check for modifications
                if let Some(modified_node) = PropertyInspector::show(ui, &node_clone) {
                    // Create undo-tracked modification
                    self.history.edit(
                        &mut self.snarl,
                        GraphEdit::ModifyNode(ModifyNodeData {
                            node_id,
                            old_data: node_clone,
                            new_data: modified_node,
                        }),
                    );
                }
            } else {
                // Node was deleted, clear selection
                self.selected_node = None;
                PropertyInspector::show_empty(ui);
            }
        } else {
            PropertyInspector::show_empty(ui);
        }
    }

    fn handle_keyboard(&mut self, ui: &mut egui::Ui) {
        // Undo: Ctrl+Z (Cmd+Z on Mac)
        let undo_pressed = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z));
        let shift_held = ui.input(|i| i.modifiers.shift);

        if undo_pressed {
            if shift_held {
                // Ctrl+Shift+Z = Redo
                self.redo();
            } else {
                // Ctrl+Z = Undo
                self.undo();
            }
        }

        // Redo: Ctrl+Y
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Y)) {
            self.redo();
        }

        // Delete: Delete or Backspace to remove selected node
        if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            if let Some(node_id) = self.selected_node.take() {
                // For now, just remove directly (could use RemoveNode command for undo)
                // We do direct removal because RemoveNode would need the node position
                // which we'd need to look up, making it more complex
                self.snarl.remove_node(node_id);
            }
        }
    }

    fn update_selected_node(&mut self, ui: &egui::Ui) {
        // Get selected nodes from egui-snarl's internal state
        let snarl_id = egui::Id::new("experiment_graph");
        let selected = get_selected_nodes(snarl_id, ui.ctx());

        // Update our cached selection (take first selected node if any)
        self.selected_node = selected.first().copied();
    }

    fn undo(&mut self) {
        self.history.undo(&mut self.snarl);
    }

    fn redo(&mut self) {
        self.history.redo(&mut self.snarl);
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
                                // Add node at reasonable position with undo tracking
                                let node_pos = self.next_node_position();
                                let node = node_type.create_node();
                                self.history.edit(
                                    &mut self.snarl,
                                    GraphEdit::AddNode(AddNodeData {
                                        node,
                                        position: node_pos,
                                        node_id: None,
                                    }),
                                );
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
                        // Create node at drop position with undo tracking
                        let node_pos = self.next_node_position();
                        let node = node_type.create_node();
                        self.history.edit(
                            &mut self.snarl,
                            GraphEdit::AddNode(AddNodeData {
                                node,
                                position: node_pos,
                                node_id: None,
                            }),
                        );
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
