//! Node palette widget for drag-and-drop node creation.

use egui::{Color32, CornerRadius, Response, Sense, StrokeKind, Ui, Vec2};

use crate::graph::ExperimentNode;

/// A node type that can be dragged from the palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeType {
    Scan,
    Acquire,
    Move,
    Wait,
    Loop,
}

impl NodeType {
    /// Returns all available node types.
    pub fn all() -> &'static [NodeType] {
        &[
            Self::Scan,
            Self::Acquire,
            Self::Move,
            Self::Wait,
            Self::Loop,
        ]
    }

    /// Returns the display name for this node type.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Scan => "Scan",
            Self::Acquire => "Acquire",
            Self::Move => "Move",
            Self::Wait => "Wait",
            Self::Loop => "Loop",
        }
    }

    /// Returns a brief description of this node type.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Scan => "Sweep a parameter across a range",
            Self::Acquire => "Capture data from a detector",
            Self::Move => "Move actuator to a position",
            Self::Wait => "Pause for a duration",
            Self::Loop => "Repeat a sequence N times",
        }
    }

    /// Returns the identifying color for this node type.
    pub fn color(&self) -> Color32 {
        match self {
            Self::Scan => Color32::from_rgb(100, 149, 237), // Cornflower blue
            Self::Acquire => Color32::from_rgb(144, 238, 144), // Light green
            Self::Move => Color32::from_rgb(255, 182, 108), // Light orange
            Self::Wait => Color32::from_rgb(192, 192, 192), // Silver
            Self::Loop => Color32::from_rgb(221, 160, 221), // Plum
        }
    }

    /// Creates a new ExperimentNode instance with default values.
    pub fn create_node(&self) -> ExperimentNode {
        match self {
            Self::Scan => ExperimentNode::default_scan(),
            Self::Acquire => ExperimentNode::default_acquire(),
            Self::Move => ExperimentNode::default_move(),
            Self::Wait => ExperimentNode::default_wait(),
            Self::Loop => ExperimentNode::default_loop(),
        }
    }
}

/// Widget for displaying available node types in a draggable palette.
pub struct NodePalette;

impl NodePalette {
    /// Render the palette. Returns `Some(NodeType)` if a drag started this frame.
    pub fn show(ui: &mut Ui) -> Option<NodeType> {
        let mut dragging = None;

        ui.vertical(|ui| {
            ui.heading("Nodes");
            ui.separator();

            for node_type in NodeType::all() {
                let response = Self::node_button(ui, *node_type);

                // Check if drag started
                if response.drag_started() {
                    dragging = Some(*node_type);
                }
            }
        });

        dragging
    }

    fn node_button(ui: &mut Ui, node_type: NodeType) -> Response {
        let desired_size = Vec2::new(ui.available_width(), 40.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::drag());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let rounding = CornerRadius::same(4);

            // Background
            ui.painter()
                .rect_filled(rect, rounding, node_type.color().gamma_multiply(0.3));
            ui.painter()
                .rect_stroke(rect, rounding, visuals.bg_stroke, StrokeKind::Inside);

            // Color indicator bar on left
            let bar_rect = egui::Rect::from_min_size(rect.min, Vec2::new(4.0, rect.height()));
            ui.painter()
                .rect_filled(bar_rect, rounding, node_type.color());

            // Text
            let text_pos = rect.min + Vec2::new(12.0, 8.0);
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_TOP,
                node_type.name(),
                egui::FontId::proportional(14.0),
                visuals.text_color(),
            );

            // Description (smaller, dimmer)
            let desc_pos = rect.min + Vec2::new(12.0, 24.0);
            ui.painter().text(
                desc_pos,
                egui::Align2::LEFT_TOP,
                node_type.description(),
                egui::FontId::proportional(10.0),
                visuals.text_color().gamma_multiply(0.7),
            );
        }

        // Show tooltip on hover
        response.on_hover_text(format!("{}: {}", node_type.name(), node_type.description()))
    }
}
