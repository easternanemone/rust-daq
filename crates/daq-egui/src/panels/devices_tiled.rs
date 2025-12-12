//! Tiled devices panel - each device in its own rearrangeable pane.

use eframe::egui;
use egui_tiles::{Tiles, Tree, TileId, Container, Linear, LinearDir};
use std::collections::HashMap;
use tokio::runtime::Runtime;

use crate::client::DaqClient;

/// A single device pane
#[derive(Clone)]
pub struct DevicePane {
    pub device_id: String,
    pub device_name: String,
    pub driver_type: String,
    pub is_movable: bool,
    pub is_readable: bool,
    pub is_triggerable: bool,
    pub is_frame_producer: bool,
    // Cached state
    pub position: Option<f64>,
    pub last_reading: Option<f64>,
    pub online: bool,
    // UI state
    pub move_target: f64,
}

impl DevicePane {
    pub fn from_device_info(info: &daq_proto::daq::DeviceInfo) -> Self {
        Self {
            device_id: info.id.clone(),
            device_name: info.name.clone(),
            driver_type: info.driver_type.clone(),
            is_movable: info.is_movable,
            is_readable: info.is_readable,
            is_triggerable: info.is_triggerable,
            is_frame_producer: info.is_frame_producer,
            position: None,
            last_reading: None,
            online: false,
            move_target: 0.0,
        }
    }

    pub fn update_state(&mut self, state: &daq_proto::daq::DeviceStateResponse) {
        self.online = state.online;
        self.position = state.position;
        self.last_reading = state.last_reading;
    }
}

/// Pending action for a device
pub enum DeviceAction {
    MoveAbsolute { device_id: String, value: f64 },
    MoveRelative { device_id: String, value: f64 },
    ReadValue { device_id: String },
}

/// Behavior for the tile tree
pub struct DevicePaneBehavior<'a> {
    pub pending_actions: &'a mut Vec<DeviceAction>,
}

impl<'a> egui_tiles::Behavior<DevicePane> for DevicePaneBehavior<'a> {
    fn tab_title_for_pane(&mut self, pane: &DevicePane) -> egui::WidgetText {
        format!("{}", pane.device_name).into()
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: TileId,
        pane: &mut DevicePane,
    ) -> egui_tiles::UiResponse {
        ui.vertical(|ui| {
            // Status indicator
            let status_color = if pane.online {
                egui::Color32::GREEN
            } else {
                egui::Color32::RED
            };
            ui.horizontal(|ui| {
                ui.colored_label(status_color, "●");
                ui.label(if pane.online { "Online" } else { "Offline" });
                ui.label(format!("({})", pane.driver_type));
            });

            ui.separator();

            // Current state
            if let Some(pos) = pane.position {
                ui.horizontal(|ui| {
                    ui.label("Position:");
                    ui.strong(format!("{:.4}", pos));
                });
            }
            if let Some(reading) = pane.last_reading {
                ui.horizontal(|ui| {
                    ui.label("Reading:");
                    ui.strong(format!("{:.4}", reading));
                });
            }

            ui.add_space(8.0);

            // Controls for movable devices
            if pane.is_movable {
                ui.group(|ui| {
                    ui.label("Motion Control");
                    ui.horizontal(|ui| {
                        ui.label("Target:");
                        ui.add(egui::DragValue::new(&mut pane.move_target).speed(0.1));
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Go Abs").clicked() {
                            self.pending_actions.push(DeviceAction::MoveAbsolute {
                                device_id: pane.device_id.clone(),
                                value: pane.move_target,
                            });
                        }
                        if ui.button("Go Rel").clicked() {
                            self.pending_actions.push(DeviceAction::MoveRelative {
                                device_id: pane.device_id.clone(),
                                value: pane.move_target,
                            });
                        }
                    });

                    // Jog buttons
                    ui.horizontal(|ui| {
                        for delta in [-10.0, -1.0, -0.1, 0.1, 1.0, 10.0] {
                            let label = if delta > 0.0 {
                                format!("+{}", delta)
                            } else {
                                format!("{}", delta)
                            };
                            if ui.small_button(label).clicked() {
                                self.pending_actions.push(DeviceAction::MoveRelative {
                                    device_id: pane.device_id.clone(),
                                    value: delta,
                                });
                            }
                        }
                    });
                });
            }

            // Controls for readable devices
            if pane.is_readable {
                ui.add_space(4.0);
                if ui.button("📖 Read").clicked() {
                    self.pending_actions.push(DeviceAction::ReadValue {
                        device_id: pane.device_id.clone(),
                    });
                }
            }
        });

        egui_tiles::UiResponse::None
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
}

/// Tiled devices panel state
pub struct DevicesTiledPanel {
    /// Tile tree for device panes
    tree: Option<Tree<DevicePane>>,
    /// Map of device_id to tile_id for updates
    device_tiles: HashMap<String, TileId>,
    /// Last refresh timestamp
    last_refresh: Option<std::time::Instant>,
    /// Error message
    error: Option<String>,
    /// Status message
    status: Option<String>,
    /// Pending actions
    pending_actions: Vec<DeviceAction>,
}

impl Default for DevicesTiledPanel {
    fn default() -> Self {
        Self {
            tree: None,
            device_tiles: HashMap::new(),
            last_refresh: None,
            error: None,
            status: None,
            pending_actions: Vec::new(),
        }
    }
}

impl DevicesTiledPanel {
    /// Render the tiled devices panel
    pub fn ui(&mut self, ui: &mut egui::Ui, client: Option<&mut DaqClient>, runtime: &Runtime) {
        // Top toolbar
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh Devices").clicked() {
                self.refresh_devices(client, runtime);
            }

            if let Some(last) = self.last_refresh {
                let elapsed = last.elapsed();
                ui.label(format!("Updated {}s ago", elapsed.as_secs()));
            }
        });

        // Show error/status messages
        if let Some(err) = &self.error {
            ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
        }
        if let Some(status) = &self.status {
            ui.colored_label(egui::Color32::GREEN, status);
        }

        ui.separator();

        // Render tile tree
        if let Some(tree) = &mut self.tree {
            let mut behavior = DevicePaneBehavior {
                pending_actions: &mut self.pending_actions,
            };
            tree.ui(&mut behavior, ui);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No devices loaded. Click 'Refresh Devices' to load.");
            });
        }

        // Process pending actions
        let actions: Vec<_> = self.pending_actions.drain(..).collect();
        for action in actions {
            self.execute_action(action, client, runtime);
        }
    }

    /// Refresh devices from daemon
    fn refresh_devices(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime) {
        self.error = None;
        self.status = None;

        let Some(client) = client else {
            self.error = Some("Not connected to daemon".to_string());
            return;
        };

        let mut client = client.clone();
        match runtime.block_on(async {
            let devices = client.list_devices().await?;
            let mut panes = Vec::new();

            for info in devices {
                let mut pane = DevicePane::from_device_info(&info);
                if let Ok(state) = client.get_device_state(&info.id).await {
                    pane.update_state(&state);
                }
                panes.push(pane);
            }

            Ok::<_, anyhow::Error>(panes)
        }) {
            Ok(panes) => {
                self.status = Some(format!("Loaded {} devices", panes.len()));
                self.last_refresh = Some(std::time::Instant::now());
                self.build_tree(panes);
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }

    /// Build the tile tree from device panes
    fn build_tree(&mut self, panes: Vec<DevicePane>) {
        let mut tiles = Tiles::default();
        self.device_tiles.clear();

        let tile_ids: Vec<TileId> = panes
            .into_iter()
            .map(|pane| {
                let device_id = pane.device_id.clone();
                let tile_id = tiles.insert_pane(pane);
                self.device_tiles.insert(device_id, tile_id);
                tile_id
            })
            .collect();

        if tile_ids.is_empty() {
            self.tree = None;
            return;
        }

        // Create a horizontal layout with all devices
        let root = tiles.insert_container(Container::Linear(Linear {
            children: tile_ids,
            dir: LinearDir::Horizontal,
            ..Default::default()
        }));

        self.tree = Some(Tree::new("devices_tree", root, tiles));
    }

    /// Execute a device action
    fn execute_action(
        &mut self,
        action: DeviceAction,
        client: Option<&mut DaqClient>,
        runtime: &Runtime,
    ) {
        self.error = None;
        self.status = None;

        let Some(client) = client else {
            self.error = Some("Not connected to daemon".to_string());
            return;
        };

        let mut client = client.clone();

        match action {
            DeviceAction::MoveAbsolute { device_id, value } => {
                match runtime.block_on(client.move_absolute(&device_id, value)) {
                    Ok(response) => {
                        if response.success {
                            self.status = Some(format!("Moved {} to {:.4}", device_id, response.final_position));
                            self.update_device_position(&device_id, response.final_position);
                        } else {
                            self.error = Some(response.error_message);
                        }
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
            DeviceAction::MoveRelative { device_id, value } => {
                match runtime.block_on(client.move_relative(&device_id, value)) {
                    Ok(response) => {
                        if response.success {
                            self.status = Some(format!("Moved {} to {:.4}", device_id, response.final_position));
                            self.update_device_position(&device_id, response.final_position);
                        } else {
                            self.error = Some(response.error_message);
                        }
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
            DeviceAction::ReadValue { device_id } => {
                match runtime.block_on(client.read_value(&device_id)) {
                    Ok(response) => {
                        if response.success {
                            self.status = Some(format!("{}: {:.4} {}", device_id, response.value, response.units));
                            self.update_device_reading(&device_id, response.value);
                        } else {
                            self.error = Some(response.error_message);
                        }
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
        }
    }

    /// Update a device's position in the tree
    fn update_device_position(&mut self, device_id: &str, position: f64) {
        if let Some(tree) = &mut self.tree {
            if let Some(&tile_id) = self.device_tiles.get(device_id) {
                if let Some(pane) = tree.tiles.get_pane_mut(tile_id) {
                    pane.position = Some(position);
                }
            }
        }
    }

    /// Update a device's reading in the tree
    fn update_device_reading(&mut self, device_id: &str, reading: f64) {
        if let Some(tree) = &mut self.tree {
            if let Some(&tile_id) = self.device_tiles.get(device_id) {
                if let Some(pane) = tree.tiles.get_pane_mut(tile_id) {
                    pane.last_reading = Some(reading);
                }
            }
        }
    }
}
