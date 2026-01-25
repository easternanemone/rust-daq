//! ELL14 Rotator control panel.
//!
//! Provides:
//! - Position display with degree formatting
//! - Jog buttons: -90, -10, -1, +1, +10, +90
//! - Home button
//! - Direct position input

use std::sync::atomic::{AtomicU64, Ordering};

use egui::Ui;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::client::DaqClient;
use crate::widgets::device_controls::DeviceControlWidget;
use daq_proto::daq::DeviceInfo;

/// Global counter for unique panel instance IDs (for diagnostic logging)
static PANEL_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Rotator state cached from the daemon
#[derive(Debug, Clone, Default)]
struct RotatorState {
    position_deg: Option<f64>,
    moving: bool,
}

/// Async action results
enum ActionResult {
    FetchState(Result<RotatorState, String>),
    Move(Result<(), String>),
    Home(Result<(), String>),
}

/// ELL14 Rotator control panel
pub struct RotatorControlPanel {
    /// Unique instance ID for diagnostic logging (identifies duplicate panels)
    panel_instance_id: u64,
    state: RotatorState,
    position_input: String,
    action_tx: mpsc::Sender<ActionResult>,
    action_rx: mpsc::Receiver<ActionResult>,
    /// User-initiated actions in flight (move, home) - disables controls
    actions_in_flight: usize,
    /// Status refresh request in flight - does NOT disable controls
    refresh_in_flight: bool,
    error: Option<String>,
    status: Option<String>,
    device_id: Option<String>,
    initial_fetch_done: bool,
    /// Auto-refresh enabled
    auto_refresh: bool,
    /// Last refresh time for auto-refresh
    last_refresh: Option<std::time::Instant>,
    /// Last command time for debouncing rapid clicks
    last_command_time: Option<std::time::Instant>,
}

impl Default for RotatorControlPanel {
    fn default() -> Self {
        let (action_tx, action_rx) = mpsc::channel(16);
        let panel_instance_id = PANEL_INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed);
        tracing::debug!(panel_instance_id, "RotatorControlPanel instance created");
        Self {
            panel_instance_id,
            state: RotatorState::default(),
            position_input: "0.0".to_string(),
            action_tx,
            action_rx,
            actions_in_flight: 0,
            refresh_in_flight: false,
            error: None,
            status: None,
            device_id: None,
            initial_fetch_done: false,
            auto_refresh: true,
            last_refresh: None,
            last_command_time: None,
        }
    }
}

impl RotatorControlPanel {
    /// Auto-refresh interval
    const REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);
    
    /// Minimum time between commands to prevent duplicate clicks
    const COMMAND_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(250);

    /// Check if enough time has passed since the last command to allow a new one
    fn can_send_command(&self) -> bool {
        self.last_command_time
            .map(|t| t.elapsed() >= Self::COMMAND_DEBOUNCE)
            .unwrap_or(true)
    }

    fn poll_results(&mut self) {
        while let Ok(result) = self.action_rx.try_recv() {
            match result {
                ActionResult::FetchState(result) => {
                    // Status refresh complete - does not affect user action count
                    self.refresh_in_flight = false;
                    match result {
                        Ok(state) => {
                            self.state = state;
                            if let Some(pos) = self.state.position_deg {
                                self.position_input = format!("{:.2}", pos);
                            }
                            self.error = None;
                        }
                        Err(e) => {
                            self.error = Some(format!("Failed to fetch state: {}", e));
                        }
                    }
                }
                ActionResult::Move(result) => {
                    // User action complete
                    self.actions_in_flight = self.actions_in_flight.saturating_sub(1);
                    match result {
                        Ok(()) => {
                            self.status = Some("Move completed".to_string());
                            self.state.moving = false;
                            self.error = None;
                        }
                        Err(e) => {
                            self.error = Some(format!("Move failed: {}", e));
                            self.state.moving = false;
                        }
                    }
                }
                ActionResult::Home(result) => {
                    // User action complete
                    self.actions_in_flight = self.actions_in_flight.saturating_sub(1);
                    match result {
                        Ok(()) => {
                            self.status = Some("Home completed".to_string());
                            self.state.moving = false;
                            self.error = None;
                        }
                        Err(e) => {
                            self.error = Some(format!("Home failed: {}", e));
                            self.state.moving = false;
                        }
                    }
                }
            }
        }
    }

    fn fetch_state(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime, device_id: &str) {
        let Some(client) = client else {
            return;
        };

        // Track refresh separately - doesn't disable controls
        self.refresh_in_flight = true;
        let mut client = client.clone();
        let tx = self.action_tx.clone();
        let device_id = device_id.to_string();

        runtime.spawn(async move {
            let result = client.get_device_state(&device_id).await;
            let state_result = result
                .map(|proto| RotatorState {
                    position_deg: proto.position,
                    moving: false, // TODO: Get from proto when available
                })
                .map_err(|e| e.to_string());
            let _ = tx.send(ActionResult::FetchState(state_result)).await;
        });

        self.last_refresh = Some(std::time::Instant::now());
    }

    fn move_absolute(
        &mut self,
        client: Option<&mut DaqClient>,
        runtime: &Runtime,
        device_id: &str,
        position: f64,
    ) {
        let Some(client) = client else {
            self.error = Some("Not connected".to_string());
            return;
        };

        // Debounce rapid clicks
        if !self.can_send_command() {
            tracing::debug!(
                panel_instance_id = self.panel_instance_id,
                device_id,
                position,
                elapsed_ms = self.last_command_time.map(|t| t.elapsed().as_millis() as u64).unwrap_or(0),
                "Move absolute command debounced - too soon after last command"
            );
            return;
        }

        self.state.moving = true;
        self.actions_in_flight += 1;
        self.last_command_time = Some(std::time::Instant::now());
        tracing::info!(
            panel_instance_id = self.panel_instance_id,
            device_id,
            position,
            actions_in_flight = self.actions_in_flight,
            "Move absolute command sent"
        );
        let mut client = client.clone();
        let tx = self.action_tx.clone();
        let device_id = device_id.to_string();

        runtime.spawn(async move {
            let result = client
                .move_absolute(&device_id, position)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string());
            let _ = tx.send(ActionResult::Move(result)).await;
        });
    }

    fn move_relative(
        &mut self,
        client: Option<&mut DaqClient>,
        runtime: &Runtime,
        device_id: &str,
        delta: f64,
    ) {
        let Some(client) = client else {
            self.error = Some("Not connected".to_string());
            return;
        };

        // Debounce rapid clicks
        if !self.can_send_command() {
            tracing::debug!(
                panel_instance_id = self.panel_instance_id,
                device_id,
                delta,
                elapsed_ms = self.last_command_time.map(|t| t.elapsed().as_millis() as u64).unwrap_or(0),
                "Jog command debounced - too soon after last command"
            );
            return;
        }

        self.state.moving = true;
        self.actions_in_flight += 1;
        self.last_command_time = Some(std::time::Instant::now());
        tracing::info!(
            panel_instance_id = self.panel_instance_id,
            device_id,
            delta,
            actions_in_flight = self.actions_in_flight,
            "Jog command sent"
        );
        let mut client = client.clone();
        let tx = self.action_tx.clone();
        let device_id = device_id.to_string();

        runtime.spawn(async move {
            let result = client
                .move_relative(&device_id, delta)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string());
            let _ = tx.send(ActionResult::Move(result)).await;
        });
    }

    fn home(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime, device_id: &str) {
        let Some(client) = client else {
            self.error = Some("Not connected".to_string());
            return;
        };

        // Debounce rapid clicks
        if !self.can_send_command() {
            tracing::debug!(
                panel_instance_id = self.panel_instance_id,
                device_id,
                elapsed_ms = self.last_command_time.map(|t| t.elapsed().as_millis() as u64).unwrap_or(0),
                "Home command debounced - too soon after last command"
            );
            return;
        }

        self.state.moving = true;
        self.actions_in_flight += 1;
        self.last_command_time = Some(std::time::Instant::now());
        tracing::info!(
            panel_instance_id = self.panel_instance_id,
            device_id,
            actions_in_flight = self.actions_in_flight,
            "Home command sent"
        );
        let mut client = client.clone();
        let tx = self.action_tx.clone();
        let device_id = device_id.to_string();

        runtime.spawn(async move {
            // Home by moving to 0 position
            let result = client
                .move_absolute(&device_id, 0.0)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string());
            let _ = tx.send(ActionResult::Home(result)).await;
        });
    }
}

impl DeviceControlWidget for RotatorControlPanel {
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

        // Initial fetch
        if !self.initial_fetch_done && client.is_some() {
            self.initial_fetch_done = true;
            self.fetch_state(client.as_deref_mut(), runtime, &device_id);
        }

        // Auto-refresh logic - only refresh if no refresh already in flight
        let should_refresh = self.auto_refresh
            && !self.refresh_in_flight
            && self
                .last_refresh
                .map(|t| t.elapsed() >= Self::REFRESH_INTERVAL)
                .unwrap_or(true);

        if should_refresh && client.is_some() {
            self.fetch_state(client.as_deref_mut(), runtime, &device_id);
        }

        // Header
        ui.horizontal(|ui| {
            ui.heading("🔄 Rotator");
            if self.state.moving || self.actions_in_flight > 0 {
                ui.spinner();
                ui.label("Moving...");
            } else if self.refresh_in_flight {
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

        // Current position display (large)
        ui.vertical_centered(|ui| {
            if let Some(pos) = self.state.position_deg {
                ui.label(
                    egui::RichText::new(format!("{:.2}°", pos))
                        .monospace()
                        .size(32.0),
                );
            } else {
                ui.label(egui::RichText::new("---°").monospace().size(32.0));
            }
        });

        ui.add_space(8.0);

        // Jog buttons row - only disable during user actions (move/home), NOT during status refresh
        let is_busy = self.state.moving || self.actions_in_flight > 0;

        ui.horizontal(|ui| {
            ui.label("Jog:");

            if ui
                .add_enabled(!is_busy, egui::Button::new("-90°"))
                .clicked()
            {
                self.move_relative(client.as_deref_mut(), runtime, &device_id, -90.0);
            }
            if ui
                .add_enabled(!is_busy, egui::Button::new("-10°"))
                .clicked()
            {
                self.move_relative(client.as_deref_mut(), runtime, &device_id, -10.0);
            }
            if ui.add_enabled(!is_busy, egui::Button::new("-1°")).clicked() {
                self.move_relative(client.as_deref_mut(), runtime, &device_id, -1.0);
            }
            if ui.add_enabled(!is_busy, egui::Button::new("+1°")).clicked() {
                self.move_relative(client.as_deref_mut(), runtime, &device_id, 1.0);
            }
            if ui
                .add_enabled(!is_busy, egui::Button::new("+10°"))
                .clicked()
            {
                self.move_relative(client.as_deref_mut(), runtime, &device_id, 10.0);
            }
            if ui
                .add_enabled(!is_busy, egui::Button::new("+90°"))
                .clicked()
            {
                self.move_relative(client.as_deref_mut(), runtime, &device_id, 90.0);
            }
        });

        ui.add_space(4.0);

        // Direct position input
        ui.horizontal(|ui| {
            ui.label("Move to:");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.position_input)
                    .desired_width(60.0)
                    .hint_text("deg"),
            );
            ui.label("°");

            if ui.add_enabled(!is_busy, egui::Button::new("Go")).clicked() {
                if let Ok(pos) = self.position_input.parse::<f64>() {
                    self.move_absolute(client.as_deref_mut(), runtime, &device_id, pos);
                } else {
                    self.error = Some("Invalid position value".to_string());
                }
            }

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !is_busy {
                if let Ok(pos) = self.position_input.parse::<f64>() {
                    self.move_absolute(client.as_deref_mut(), runtime, &device_id, pos);
                }
            }
        });

        ui.add_space(8.0);
        ui.separator();

        // Quick position buttons
        ui.horizontal(|ui| {
            ui.label("Quick positions:");
            for angle in [0.0, 45.0, 90.0, 180.0, 270.0] {
                if ui
                    .add_enabled(!is_busy, egui::Button::new(format!("{}°", angle)))
                    .clicked()
                {
                    self.move_absolute(client.as_deref_mut(), runtime, &device_id, angle);
                }
            }
        });

        ui.add_space(8.0);

        // Action buttons
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!is_busy, egui::Button::new("🏠 Home"))
                .clicked()
            {
                self.home(client.as_deref_mut(), runtime, &device_id);
            }

            ui.checkbox(&mut self.auto_refresh, "Auto-refresh");

            if ui.button("🔄 Refresh").clicked() {
                self.fetch_state(client, runtime, &device_id);
            }
        });

        // Request repaint for auto-refresh or while busy
        if self.auto_refresh || self.actions_in_flight > 0 || self.refresh_in_flight {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn device_type(&self) -> &'static str {
        "Rotator"
    }
}
