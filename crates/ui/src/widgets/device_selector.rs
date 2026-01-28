//! Device selector widget with fuzzy-match autocomplete.

use egui::Ui;

/// Device selector widget with simple autocomplete.
///
/// Provides device selection with substring matching and dropdown suggestions.
pub struct DeviceSelector {
    text: String,
    candidates: Vec<String>,
    popup_id: egui::Id,
}

impl DeviceSelector {
    /// Create a new device selector with the given list of device IDs.
    pub fn new(device_ids: &[String]) -> Self {
        Self {
            text: String::new(),
            candidates: device_ids.to_vec(),
            popup_id: egui::Id::new("device_selector_popup"),
        }
    }

    /// Show the autocomplete text edit widget.
    ///
    /// Returns `true` if the selection changed.
    pub fn show(&mut self, ui: &mut Ui, hint: &str) -> bool {
        let before = self.text.clone();

        let response = ui.text_edit_singleline(&mut self.text);

        // Show hint text when empty
        if !hint.is_empty() && self.text.is_empty() && !response.has_focus() {
            let rect = response.rect;
            let painter = ui.painter();
            painter.text(
                rect.left_center() + egui::vec2(4.0, 0.0),
                egui::Align2::LEFT_CENTER,
                hint,
                egui::FontId::default(),
                ui.visuals().weak_text_color(),
            );
        }

        // Show dropdown with matching candidates
        if response.has_focus() && !self.text.is_empty() {
            let matches: Vec<_> = self
                .candidates
                .iter()
                .filter(|c| c.to_lowercase().contains(&self.text.to_lowercase()))
                .take(10)
                .cloned()
                .collect();

            if !matches.is_empty() {
                let popup_pos = response.rect.left_bottom();
                egui::Area::new(self.popup_id)
                    .fixed_pos(popup_pos)
                    .order(egui::Order::Foreground)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.set_min_width(response.rect.width());
                            egui::ScrollArea::vertical()
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    for candidate in matches {
                                        if ui.selectable_label(false, &candidate).clicked() {
                                            self.text = candidate;
                                            response.request_focus();
                                        }
                                    }
                                });
                        });
                    });
            }
        }

        self.text != before
    }

    /// Get the currently selected device ID.
    pub fn selected(&self) -> &str {
        &self.text
    }

    /// Set the selected device ID programmatically.
    pub fn set_selected(&mut self, device_id: &str) {
        self.text = device_id.to_string();
    }

    /// Update the list of available device IDs (for registry refresh).
    #[allow(dead_code)]
    pub fn update_candidates(&mut self, device_ids: &[String]) {
        self.candidates = device_ids.to_vec();
    }
}

impl Default for DeviceSelector {
    fn default() -> Self {
        Self::new(&[])
    }
}
