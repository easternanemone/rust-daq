//! Analog Output control panel for DAC devices (e.g., EOM voltage control).
//!
//! Provides:
//! - Voltage slider with -10V to +10V range
//! - Numeric input for precise voltage entry
//! - Quick buttons: 0V, Min, Max
//! - Status display

use egui::Ui;
use serde_json::json;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::client::DaqClient;
use crate::widgets::device_controls::DeviceControlWidget;
use daq_proto::daq::DeviceInfo;

/// Async action results
enum ActionResult {
    WriteVoltage(Result<f64, String>),
}

/// Analog Output control panel for DAC devices
pub struct AnalogOutputControlPanel {
    /// Current voltage setpoint
    voltage: f64,
    /// Voltage input as string for text editing
    voltage_input: String,
    /// Action channel sender
    action_tx: mpsc::Sender<ActionResult>,
    /// Action channel receiver
    action_rx: mpsc::Receiver<ActionResult>,
    /// Number of actions in flight
    actions_in_flight: usize,
    /// Error message
    error: Option<String>,
    /// Status message
    status: Option<String>,
    /// Device ID (cached)
    device_id: Option<String>,
    /// Min voltage (from metadata or default)
    min_voltage: f64,
    /// Max voltage (from metadata or default)
    max_voltage: f64,
}

impl Default for AnalogOutputControlPanel {
    fn default() -> Self {
        let (action_tx, action_rx) = mpsc::channel(16);
        Self {
            voltage: 0.0,
            voltage_input: "0.0".to_string(),
            action_tx,
            action_rx,
            actions_in_flight: 0,
            error: None,
            status: None,
            device_id: None,
            min_voltage: -10.0,
            max_voltage: 10.0,
        }
    }
}

impl AnalogOutputControlPanel {
    fn poll_results(&mut self) {
        while let Ok(result) = self.action_rx.try_recv() {
            self.actions_in_flight = self.actions_in_flight.saturating_sub(1);

            match result {
                ActionResult::WriteVoltage(result) => match result {
                    Ok(voltage) => {
                        self.voltage = voltage;
                        self.voltage_input = format!("{:.3}", voltage);
                        self.status = Some(format!("Set to {:.3} V", voltage));
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(format!("Write failed: {}", e));
                    }
                },
            }
        }
    }

    fn write_voltage(
        &mut self,
        client: Option<&mut DaqClient>,
        runtime: &Runtime,
        device_id: &str,
        voltage: f64,
    ) {
        let Some(client) = client else {
            self.error = Some("Not connected".to_string());
            return;
        };

        self.actions_in_flight += 1;
        let mut client = client.clone();
        let tx = self.action_tx.clone();
        let device_id = device_id.to_string();
        let voltage = voltage.clamp(self.min_voltage, self.max_voltage);

        runtime.spawn(async move {
            // Use write_voltage command with JSON args
            let args = json!({ "voltage": voltage }).to_string();
            let result = client
                .execute_device_command(&device_id, "write_voltage", &args)
                .await
                .map(|_| voltage)
                .map_err(|e| e.to_string());
            let _ = tx.send(ActionResult::WriteVoltage(result)).await;
        });
    }
}

impl DeviceControlWidget for AnalogOutputControlPanel {
    fn ui(
        &mut self,
        ui: &mut Ui,
        device: &DeviceInfo,
        mut client: Option<&mut DaqClient>,
        runtime: &Runtime,
    ) {
        self.poll_results();

        let device_id = device.id.clone();
        self.device_id = Some(device_id.clone());

        // Header
        ui.horizontal(|ui| {
            ui.heading("⚡ Analog Output");
            if self.actions_in_flight > 0 {
                ui.spinner();
            }
        });

        if let Some(ref err) = self.error {
            ui.colored_label(egui::Color32::RED, err);
        }
        if let Some(ref status) = self.status {
            ui.colored_label(egui::Color32::GREEN, status);
        }

        ui.separator();

        // Current voltage display (large)
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(format!("{:.3} V", self.voltage))
                    .monospace()
                    .size(32.0),
            );
        });

        ui.add_space(8.0);

        // Voltage slider
        let is_busy = self.actions_in_flight > 0;

        ui.horizontal(|ui| {
            ui.label("Voltage:");
            let mut voltage = self.voltage;
            let slider = egui::Slider::new(&mut voltage, self.min_voltage..=self.max_voltage)
                .suffix(" V")
                .clamping(egui::SliderClamping::Always);

            if ui.add_enabled(!is_busy, slider).changed() {
                self.voltage = voltage;
                self.voltage_input = format!("{:.3}", voltage);
                self.write_voltage(client.as_deref_mut(), runtime, &device_id, voltage);
            }
        });

        // Numeric input
        ui.horizontal(|ui| {
            ui.label("Set:");
            let response = ui.add_enabled(
                !is_busy,
                egui::TextEdit::singleline(&mut self.voltage_input)
                    .desired_width(80.0)
                    .hint_text("V"),
            );
            ui.label("V");

            if ui.add_enabled(!is_busy, egui::Button::new("Apply")).clicked() {
                if let Ok(v) = self.voltage_input.parse::<f64>() {
                    let v = v.clamp(self.min_voltage, self.max_voltage);
                    self.write_voltage(client.as_deref_mut(), runtime, &device_id, v);
                } else {
                    self.error = Some("Invalid voltage value".to_string());
                }
            }

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !is_busy {
                if let Ok(v) = self.voltage_input.parse::<f64>() {
                    let v = v.clamp(self.min_voltage, self.max_voltage);
                    self.write_voltage(client.as_deref_mut(), runtime, &device_id, v);
                }
            }
        });

        ui.add_space(8.0);
        ui.separator();

        // Quick buttons
        ui.horizontal(|ui| {
            ui.label("Quick set:");

            if ui.add_enabled(!is_busy, egui::Button::new("0 V")).clicked() {
                self.write_voltage(client.as_deref_mut(), runtime, &device_id, 0.0);
            }

            if ui
                .add_enabled(!is_busy, egui::Button::new(format!("{:.0} V", self.min_voltage)))
                .clicked()
            {
                self.write_voltage(client.as_deref_mut(), runtime, &device_id, self.min_voltage);
            }

            if ui
                .add_enabled(!is_busy, egui::Button::new(format!("{:.0} V", self.max_voltage)))
                .clicked()
            {
                self.write_voltage(client.as_deref_mut(), runtime, &device_id, self.max_voltage);
            }
        });

        // Request repaint while busy
        if self.actions_in_flight > 0 {
            ui.ctx().request_repaint();
        }
    }

    fn device_type(&self) -> &'static str {
        "Analog Output"
    }
}
