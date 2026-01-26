//! Adaptive scan alert modal dialog.
//!
//! Shows trigger detection results and allows user to approve or cancel
//! the planned adaptive action.

use egui::{Color32, Id, Modal, RichText};

use crate::graph::adaptive::DetectedPeak;
use crate::graph::nodes::AdaptiveAction;

/// Data for an adaptive trigger alert.
#[derive(Clone, Debug)]
pub struct AdaptiveAlertData {
    /// Unique ID for this alert instance
    pub id: String,
    /// Which trigger condition(s) fired
    pub trigger_description: String,
    /// Detected peak information (if peak detection)
    pub peak: Option<DetectedPeak>,
    /// Action that will be taken
    pub action: AdaptiveAction,
    /// Whether user approval is required
    pub requires_approval: bool,
}

/// Response from the alert dialog.
#[derive(Clone, Debug, PartialEq)]
pub enum AdaptiveAlertResponse {
    /// User approved the action
    Approved,
    /// User cancelled the action
    Cancelled,
    /// Dialog still open (no response yet)
    Pending,
}

/// Show an adaptive trigger alert modal.
///
/// # Arguments
/// * `ctx` - egui context
/// * `data` - Alert data to display
///
/// # Returns
/// User response (Pending while dialog is open)
pub fn show_adaptive_alert(ctx: &egui::Context, data: &AdaptiveAlertData) -> AdaptiveAlertResponse {
    let mut response = AdaptiveAlertResponse::Pending;

    let modal = Modal::new(Id::new(&data.id)).backdrop_color(Color32::from_black_alpha(150));

    let modal_response = modal.show(ctx, |ui| {
        ui.set_min_width(300.0);

        // Header with icon
        ui.horizontal(|ui| {
            ui.label(RichText::new("Trigger Detected!").size(18.0).strong());
        });

        ui.separator();
        ui.add_space(8.0);

        // Trigger description
        ui.label(&data.trigger_description);

        // Peak details if available
        if let Some(ref peak) = data.peak {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Peak height:");
                ui.label(RichText::new(format!("{:.2}", peak.height)).strong());
            });
            if let Some(pos) = peak.position {
                ui.horizontal(|ui| {
                    ui.label("Position:");
                    ui.label(RichText::new(format!("{:.4}", pos)).strong());
                });
            }
        }

        ui.add_space(12.0);

        // Action description
        let action_text = match data.action {
            AdaptiveAction::Zoom2x => "Zoom 2x and rescan region",
            AdaptiveAction::Zoom4x => "Zoom 4x and rescan region",
            AdaptiveAction::MoveToPeak => "Move actuator to peak position",
            AdaptiveAction::AcquireAtPeak => "Acquire data at peak position",
            AdaptiveAction::MarkAndContinue => "Mark location and continue",
        };

        ui.horizontal(|ui| {
            ui.label("Action:");
            ui.label(RichText::new(action_text).italics());
        });

        ui.add_space(16.0);

        // Approval buttons
        if data.requires_approval {
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new("Continue").color(Color32::from_rgb(100, 200, 100)))
                    .clicked()
                {
                    response = AdaptiveAlertResponse::Approved;
                }
                ui.add_space(8.0);
                if ui
                    .button(RichText::new("Cancel").color(Color32::from_rgb(255, 100, 100)))
                    .clicked()
                {
                    response = AdaptiveAlertResponse::Cancelled;
                }
            });
        } else {
            // Auto-proceed countdown (3 seconds)
            ui.label(RichText::new("Proceeding automatically...").weak());
            if ui.button("Cancel").clicked() {
                response = AdaptiveAlertResponse::Cancelled;
            }
        }
    });

    // Handle backdrop click or escape to cancel
    if modal_response.should_close() && response == AdaptiveAlertResponse::Pending {
        response = AdaptiveAlertResponse::Cancelled;
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_data_creation() {
        let data = AdaptiveAlertData {
            id: "test_alert".to_string(),
            trigger_description: "Peak detected above threshold".to_string(),
            peak: Some(DetectedPeak {
                index: 42,
                height: 1234.5,
                position: Some(45.67),
            }),
            action: AdaptiveAction::Zoom2x,
            requires_approval: true,
        };

        assert_eq!(data.id, "test_alert");
        assert!(data.requires_approval);
        assert!(data.peak.is_some());
    }

    #[test]
    fn test_alert_response_equality() {
        assert_eq!(AdaptiveAlertResponse::Pending, AdaptiveAlertResponse::Pending);
        assert_ne!(AdaptiveAlertResponse::Approved, AdaptiveAlertResponse::Cancelled);
    }
}
