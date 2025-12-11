#![allow(missing_docs)]
use eframe::{egui, App, Frame};
use std::collections::{HashMap, HashSet};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use egui_plot::{Line, Plot, PlotImage, PlotPoint as EguiPlotPoint, PlotPoints};

use crate::gui::{
    create_channels, parameter_widget, spawn_backend, BackendCommand, BackendEvent,
    ConnectionStatus, DeviceInfo, ParameterDescriptor, ParameterEditState, UiChannels,
    WidgetResult,
};

/// Device row for display in the UI.
#[derive(Clone, Default)]
pub struct DeviceRow {
    pub id: String,
    pub name: String,
    pub driver_type: String,
    pub capabilities: Vec<String>,
    pub last_value: Option<f64>,
    pub last_units: String,
    pub last_updated: Option<Instant>,
    pub error: Option<String>,
    /// Device state fields from streaming
    pub state_fields: HashMap<String, String>,
    /// Parameter descriptors
    pub parameters: Vec<ParameterDescriptor>,
}

impl From<DeviceInfo> for DeviceRow {
    fn from(info: DeviceInfo) -> Self {
        Self {
            id: info.id,
            name: info.name,
            driver_type: info.driver_type,
            capabilities: info.capabilities,
            last_value: None,
            last_units: String::new(),
            last_updated: None,
            error: None,
            state_fields: HashMap::new(),
            parameters: Vec::new(),
        }
    }
}

/// Main GUI application state.
pub struct DaqGuiApp {
    /// Unique ID for the app instance
    pub id: String,
    /// Human readable name
    pub name: String,
    /// Connection URL (direct or proxy)
    pub url: String,
    /// Current connection status
    pub connection_status: ConnectionStatus,
    /// Status message for display
    pub status_line: String,
    /// List of devices
    pub devices: Vec<DeviceRow>,
    /// Channels for backend communication
    pub channels: UiChannels,
    /// Last UI update time for starvation detection
    pub last_update: Instant,
    /// Backend thread handle (kept alive)
    #[cfg(not(target_arch = "wasm32"))]
    pub _backend_handle: Option<std::thread::JoinHandle<()>>,
    #[cfg(target_arch = "wasm32")]
    pub _backend_handle: Option<()>,
    /// Whether state streaming is active
    pub is_streaming: bool,

    /// Currently selected device ID (for plot/image highlighting)
    pub selected_device_id: Option<String>,

    /// Set of devices with open control windows
    pub open_devices: HashSet<String>,

    /// Parameter edit states for immediate-mode widgets
    pub param_edit_states: HashMap<String, ParameterEditState>,

    /// Data history for plotting. Map device_id -> Vec of [time, value]
    pub history: HashMap<String, Vec<[f64; 2]>>,
    /// Application start time for relative plotting
    pub start_time: Instant,
    /// Latest image data and texture for each device
    pub images: HashMap<String, (egui::ColorImage, Option<egui::TextureHandle>)>,

    /// Application logs
    pub logs: Vec<(String, String)>, // (timestamp, message)

    /// Style initialization check
    pub style_initialized: bool,

    /// Auto-connect flag
    pub first_frame: bool,
}

impl DaqGuiApp {
    pub fn new() -> Self {
        let (channels, backend_handle) = create_channels();
        #[cfg(not(target_arch = "wasm32"))]
        let backend_thread = spawn_backend(backend_handle);
        #[cfg(target_arch = "wasm32")]
        let backend_thread = {
            log::error!("Debug: About to call spawn_backend from app::new");
            let t = spawn_backend(backend_handle);
            log::error!("Debug: spawn_backend returned in app::new");
            t
        };

        Self::init(channels, Some(backend_thread))
    }

    /// Create a new instance with provided channels (for testing)
    pub fn new_with_channels(channels: UiChannels) -> Self {
        Self::init(channels, None)
    }

    fn init(
        channels: UiChannels,
        #[cfg(not(target_arch = "wasm32"))] backend_thread: Option<std::thread::JoinHandle<()>>,
        #[cfg(target_arch = "wasm32")] backend_thread: Option<()>,
    ) -> Self {
        log::info!("App Init Starting");

        #[cfg(target_arch = "wasm32")]
        let url = "http://127.0.0.1:8080".to_string(); // Use proxy
        #[cfg(not(target_arch = "wasm32"))]
        let url = "http://127.0.0.1:50051".to_string();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Rust DAQ GUI".to_string(),
            connection_status: ConnectionStatus::Disconnected,
            url,
            status_line: "Ready".to_string(),
            devices: Vec::new(),
            channels,
            last_update: Instant::now(),
            _backend_handle: backend_thread,
            is_streaming: false,
            selected_device_id: None,
            open_devices: HashSet::new(),
            param_edit_states: HashMap::new(),
            history: HashMap::new(),
            start_time: Instant::now(),
            images: HashMap::new(),
            logs: Vec::new(),
            style_initialized: false,
            first_frame: true,
        }
    }

    /// Add a log message
    pub fn log(&mut self, msg: String) {
        let timestamp = chrono::Utc::now().format("%H:%M:%S").to_string();
        self.logs.push((timestamp, msg));

        // Keep logs bounded
        if self.logs.len() > 1000 {
            self.logs.remove(0);
        }
    }

    /// Process all pending events from the backend.
    pub fn process_backend_events(&mut self) {
        for event in self.channels.drain_events() {
            match event {
                BackendEvent::DevicesRefreshed { devices } => {
                    // Preserve existing state rows if possible
                    let new_rows: Vec<DeviceRow> = devices
                        .into_iter()
                        .map(|d| {
                            if let Some(existing) = self.devices.iter().find(|old| old.id == d.id) {
                                let mut row = DeviceRow::from(d);
                                row.state_fields = existing.state_fields.clone();
                                row.last_value = existing.last_value;
                                row.last_units = existing.last_units.clone();
                                row.last_updated = existing.last_updated;
                                // Preserve params if we have them so windows don't flicker empty
                                if !existing.parameters.is_empty() {
                                    row.parameters = existing.parameters.clone();
                                }
                                row
                            } else {
                                DeviceRow::from(d)
                            }
                        })
                        .collect();

                    self.devices = new_rows;
                    let msg = format!("Loaded {} devices", self.devices.len());
                    self.status_line = msg.clone();
                    self.log(msg);

                    // Auto-start state streaming after devices are loaded
                    if !self.is_streaming {
                        self.channels
                            .send_command(BackendCommand::StartStateStream {
                                device_ids: vec![], // Subscribe to all devices
                            });
                    }
                }
                BackendEvent::ValueRead {
                    device_id,
                    value,
                    units,
                } => {
                    if let Some(row) = self.devices.iter_mut().find(|d| d.id == device_id) {
                        row.last_value = Some(value);
                        row.last_units = units;
                        row.last_updated = Some(Instant::now());
                        row.error = None;
                    }

                    // Update history
                    let t = Instant::now().duration_since(self.start_time).as_secs_f64();
                    let entry = self.history.entry(device_id).or_default();
                    entry.push([t, value]);
                    // Limit history size
                    if entry.len() > 1000 {
                        entry.remove(0);
                    }
                }
                BackendEvent::DeviceStateUpdated { .. } => {
                    // Legacy: ignore if received (state is now via watch channel)
                }
                BackendEvent::StateStreamStarted => {
                    self.is_streaming = true;
                    let msg = format!("{} (streaming)", self.status_line);
                    self.status_line = msg.clone();
                    self.log("State stream started".to_string());
                }
                BackendEvent::StateStreamStopped => {
                    self.is_streaming = false;
                    self.log("State stream stopped".to_string());
                }
                BackendEvent::ParametersFetched {
                    device_id,
                    parameters,
                } => {
                    let mut log_msg = None;
                    if let Some(row) = self.devices.iter_mut().find(|d| d.id == device_id) {
                        row.parameters = parameters;
                        log_msg = Some(format!(
                            "Fetched {} parameters for {}",
                            row.parameters.len(),
                            row.name
                        ));
                    }
                    if let Some(msg) = log_msg {
                        self.log(msg);
                    }
                }
                BackendEvent::Error { message } => {
                    self.status_line = format!("Error: {}", message);
                    self.log(format!("ERROR: {}", message));
                }
                BackendEvent::ConnectionChanged { status } => {
                    self.connection_status = status.clone();
                    let msg = match &status {
                        ConnectionStatus::Disconnected => "Disconnected".to_string(),
                        ConnectionStatus::Connecting => "Connecting...".to_string(),
                        ConnectionStatus::Connected => "Connected".to_string(),
                        ConnectionStatus::Reconnecting { attempt } => {
                            format!("Reconnecting (attempt {})...", attempt)
                        }
                        ConnectionStatus::Failed { reason } => {
                            format!("Connection failed: {}", reason)
                        }
                    };
                    self.status_line = msg.clone();
                    self.log(format!("Connection status: {}", msg));

                    // Reset streaming state on disconnect
                    if matches!(
                        status,
                        ConnectionStatus::Disconnected | ConnectionStatus::Failed { .. }
                    ) {
                        self.is_streaming = false;
                    }
                }
                BackendEvent::ImageReceived {
                    device_id,
                    size,
                    data,
                } => {
                    let [w, h] = size;
                    if w * h > 0 {
                        // Determine format based on data length
                        let image = if data.len() == w * h {
                            // Grayscale
                            egui::ColorImage::from_gray([w, h], &data)
                        } else if data.len() == w * h * 3 {
                            // RGB
                            egui::ColorImage::from_rgb([w, h], &data)
                        } else if data.len() == w * h * 4 {
                            // RGBA
                            egui::ColorImage::from_rgba_unmultiplied([w, h], &data)
                        } else {
                            tracing::warn!("Invalid image data size for dimensions {}x{}", w, h);
                            return;
                        };

                        // Store image and invalidate texture (set to None)
                        self.images.insert(device_id, (image, None));
                    }
                }
            }
        }
    }

    /// Sync device state from watch channel to device rows.
    /// This pulls latest state from the watch channel (never blocks).
    pub fn sync_device_state(&mut self) {
        let snapshot = self.channels.get_state();
        for row in &mut self.devices {
            if let Some(device_state) = snapshot.devices.get(&row.id) {
                // Update state fields
                row.state_fields = device_state.fields.clone();
                if let Some(updated_at) = device_state.updated_at {
                    row.last_updated = Some(updated_at);
                }

                // Extract position if available for movable devices
                if let Some(pos_str) = row.state_fields.get("position") {
                    if let Ok(pos) = pos_str.parse::<f64>() {
                        row.last_value = Some(pos);
                        row.last_units = "pos".to_string();
                    }
                }
            }
        }
    }

    /// Check for UI starvation (frame time > 50ms).
    fn check_starvation(&mut self) {
        let elapsed = self.last_update.elapsed();
        if elapsed.as_millis() > 50 {
            tracing::warn!("UI starvation detected: {:?} since last update", elapsed);
        }
        self.last_update = Instant::now();
    }

    /// Apply a modern dark theme to the egui context.
    fn apply_style(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        let bg_color = egui::Color32::from_rgb(24, 25, 38);
        let panel_color = egui::Color32::from_rgb(30, 32, 48);
        let widget_bg = egui::Color32::from_rgb(45, 48, 69);
        let accent = egui::Color32::from_rgb(138, 173, 244);
        let text_primary = egui::Color32::from_rgb(202, 211, 245);
        let text_secondary = egui::Color32::from_rgb(165, 173, 203);
        let error_color = egui::Color32::from_rgb(237, 135, 150);

        visuals.window_fill = panel_color;
        visuals.panel_fill = panel_color;
        visuals.faint_bg_color = widget_bg;
        visuals.extreme_bg_color = bg_color;

        visuals.widgets.noninteractive.weak_bg_fill = panel_color;
        visuals.widgets.noninteractive.bg_fill = panel_color;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, bg_color);
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text_primary);
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(6.0);

        visuals.widgets.inactive.weak_bg_fill = widget_bg;
        visuals.widgets.inactive.bg_fill = widget_bg;
        visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_secondary);

        visuals.widgets.hovered.weak_bg_fill = accent.linear_multiply(0.2);
        visuals.widgets.hovered.bg_fill = accent.linear_multiply(0.2);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, text_primary);
        visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);

        visuals.widgets.active.bg_fill = accent.linear_multiply(0.4);
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.widgets.active.rounding = egui::Rounding::same(6.0);

        visuals.selection.bg_fill = accent.linear_multiply(0.3);
        visuals.selection.stroke = egui::Stroke::new(1.0, accent);

        visuals.hyperlink_color = accent;
        visuals.error_fg_color = error_color;
        visuals.warn_fg_color = egui::Color32::from_rgb(245, 169, 127);

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.window_margin = egui::Margin::same(8.0);
        style.spacing.button_padding = egui::vec2(8.0, 5.0);
        style.visuals.resize_corner_size = 12.0;
        ctx.set_style(style);
    }

    /// Main UI rendering logic (independent of eframe::Frame).
    pub fn ui(&mut self, ctx: &egui::Context) {
        Self::apply_style(ctx);
        self.check_starvation();
        self.process_backend_events();
        self.sync_device_state();

        // ---------------------------------------------------------------------
        // Top Panel: Connection Controls
        // ---------------------------------------------------------------------
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("rust-daq");
                ui.separator();

                ui.label("Daemon Address:");
                let addr_response = ui.text_edit_singleline(&mut self.url);

                let is_connected = matches!(self.connection_status, ConnectionStatus::Connected);
                let is_connecting = matches!(
                    self.connection_status,
                    ConnectionStatus::Connecting | ConnectionStatus::Reconnecting { .. }
                );

                if is_connected {
                    if ui.button("Disconnect").clicked() {
                        self.channels.send_command(BackendCommand::Disconnect);
                    }
                    if ui.button("Refresh Devices").clicked() {
                        self.channels.send_command(BackendCommand::RefreshDevices);
                        self.status_line = "Refreshing devices...".to_string();
                    }
                } else if is_connecting {
                    ui.add_enabled(false, egui::Button::new("Connecting..."));
                } else {
                    if ui.button("Connect").clicked()
                        || (addr_response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        let address = self.url.clone();
                        self.channels
                            .send_command(BackendCommand::Connect { address });
                    }
                }

                // Status Indicator
                let status_color = match &self.connection_status {
                    ConnectionStatus::Connected => egui::Color32::GREEN,
                    ConnectionStatus::Connecting | ConnectionStatus::Reconnecting { .. } => {
                        egui::Color32::YELLOW
                    }
                    ConnectionStatus::Disconnected => egui::Color32::GRAY,
                    ConnectionStatus::Failed { .. } => egui::Color32::RED,
                };
                ui.colored_label(status_color, "●");
                ui.label(&self.status_line);
            });
        });

        // ---------------------------------------------------------------------
        // Bottom Panel: Logs
        // ---------------------------------------------------------------------
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .min_height(100.0)
            .show(ctx, |ui| {
                ui.heading("Log");
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for (ts, msg) in &self.logs {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(ts).weak().monospace());
                                ui.label(egui::RichText::new(msg).monospace());
                            });
                        }
                    });
            });

        // ---------------------------------------------------------------------
        // Left Panel: Device List
        // ---------------------------------------------------------------------
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.heading("Devices");
                ui.add_space(4.0);
                self.ui_device_list(ui);

                ui.add_space(20.0);
                let metrics = self.channels.get_metrics();
                ui.separator();
                ui.small("Backend Metrics:");
                ui.small(format!("Frame Time: {:.2}ms", metrics.ui_frame_ms));
                ui.small(format!("Dropped Frames: {}", metrics.frames_dropped));
                ui.small(format!("Streams Restarted: {}", metrics.stream_restarts));
            });

        // ---------------------------------------------------------------------
        // Central Panel: Plots & Images
        // ---------------------------------------------------------------------
        egui::CentralPanel::default().show(ctx, |ui| {
            // If we have a selected device, show its plot
            // If it's a camera, show image
            if let Some(id) = &self.selected_device_id {
                if let Some((image, texture_opt)) = self.images.get_mut(id) {
                    let texture = texture_opt.get_or_insert_with(|| {
                        ui.ctx().load_texture(
                            format!("img_{}", id),
                            image.clone(),
                            egui::TextureOptions::NEAREST,
                        )
                    });
                    let texture_id = texture.id();
                    let size = texture.size_vec2();
                    Plot::new("camera_image_center")
                        .view_aspect(1.0)
                        .data_aspect(1.0)
                        .show(ui, |plot_ui| {
                            plot_ui.image(PlotImage::new(
                                texture_id,
                                EguiPlotPoint::new(0.0, 0.0),
                                size,
                            ))
                        });
                } else if let Some(data) = self.history.get(id) {
                    Plot::new("device_plot_center")
                        .view_aspect(2.0)
                        .show(ui, |plot_ui| {
                            plot_ui.line(Line::new(PlotPoints::from(data.clone())))
                        });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No data available for selected device.")
                    });
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a device to view plots. Open device controls from the list.")
                });
            }
        });

        // ---------------------------------------------------------------------
        // Floating Windows for Device Controls
        // ---------------------------------------------------------------------
        // Clone to avoid borrow checker issues while iterating and modifying app state
        let open_ids: Vec<String> = self.open_devices.iter().cloned().collect();
        let mut closed_ids = Vec::new();

        for device_id in open_ids {
            let mut is_open = true;
            if let Some(device) = self.devices.iter_mut().find(|d| d.id == device_id) {
                // Use the device struct to render connection details
                let name = device.name.clone();

                egui::Window::new(&name)
                    .id(egui::Id::new(&device_id))
                    .open(&mut is_open)
                    .resizable(true)
                    .default_size([300.0, 400.0])
                    .show(ctx, |ui| {
                        Self::ui_device_window_content(
                            ui,
                            device,
                            &mut self.param_edit_states,
                            &mut self.channels,
                        );
                    });
            }

            if !is_open {
                closed_ids.push(device_id);
            }
        }

        for id in closed_ids {
            self.open_devices.remove(&id);
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }

    fn ui_device_list(&mut self, ui: &mut egui::Ui) {
        if self.devices.is_empty() {
            ui.label("No devices found.");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("device_grid_list")
                .striped(true)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.strong("Device");
                    ui.strong("Value");
                    ui.strong("Controls");
                    ui.end_row();

                    // Collect data needed for rendering to avoid fighting borrow checker
                    // We need mutable access to send commands, but iterating self.devices borrows self.
                    // However, we can iterate self.devices for display and use channels which is in self.
                    // Splitting borrows is hard here, so we'll index or use a collected view.

                    let device_ids: Vec<String> =
                        self.devices.iter().map(|d| d.id.clone()).collect();

                    for id in device_ids {
                        if let Some(device) = self.devices.iter_mut().find(|d| d.id == id) {
                            let is_selected = self.selected_device_id.as_ref() == Some(&id);
                            let is_open = self.open_devices.contains(&id);

                            // Name (Click to select for Plot)
                            if ui.selectable_label(is_selected, &device.name).clicked() {
                                // Toggle selection
                                if is_selected {
                                    self.selected_device_id = None;
                                } else {
                                    self.selected_device_id = Some(id.clone());
                                    // Also fetch params when selected, just in case
                                    self.channels.send_command(BackendCommand::FetchParameters {
                                        device_id: id.clone(),
                                    });
                                }
                            }

                            // Value
                            if let Some(v) = device.last_value {
                                ui.label(format!("{:.4} {}", v, device.last_units));
                            } else {
                                ui.label("-");
                            }

                            // Open/Close Control Window
                            if ui.selectable_label(is_open, "Control").clicked() {
                                if is_open {
                                    self.open_devices.remove(&id);
                                } else {
                                    self.open_devices.insert(id.clone());
                                    // Ensure we have parameters
                                    self.channels.send_command(BackendCommand::FetchParameters {
                                        device_id: id.clone(),
                                    });
                                }
                            }

                            ui.end_row();
                        }
                    }
                });
        });
    }

    fn ui_device_window_content(
        ui: &mut egui::Ui,
        device: &mut DeviceRow,
        states: &mut HashMap<String, ParameterEditState>,
        channels: &mut UiChannels,
    ) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&device.driver_type).weak());
            ui.separator();
            ui.label(format!("Caps: {}", device.capabilities.join(", ")));
        });
        ui.separator();

        // 1. Motion Controls (Top Priority)
        if device.capabilities.contains(&"Movable".to_string()) {
            ui.heading("Motion");
            egui::Grid::new(format!("motion_{}", device.id))
                .striped(true)
                .show(ui, |ui| {
                    // Absolute
                    ui.label("Absolute:");
                    let abs_key = format!("{}:abs_move", device.id);
                    let state = states.entry(abs_key).or_default();

                    // Init from current pos if just opened
                    if !state.initialized {
                        if let Some(pos_str) = device.state_fields.get("position") {
                            if let Ok(v) = pos_str.parse::<f64>() {
                                state.float_value = v;
                            }
                        }
                        state.initialized = true;
                    }

                    ui.add(egui::DragValue::new(&mut state.float_value).speed(0.1));
                    if ui.button("Go").clicked() {
                        channels.send_command(BackendCommand::MoveAbsolute {
                            device_id: device.id.clone(),
                            position: state.float_value,
                        });
                    }
                    ui.end_row();

                    // Relative
                    ui.label("Relative:");
                    let rel_key = format!("{}:rel_move", device.id);
                    let rel_state = states.entry(rel_key).or_default();
                    ui.add(egui::DragValue::new(&mut rel_state.float_value).speed(0.1));

                    if ui.button("-").clicked() {
                        channels.send_command(BackendCommand::MoveRelative {
                            device_id: device.id.clone(),
                            distance: -rel_state.float_value,
                        });
                    }
                    if ui.button("+").clicked() {
                        channels.send_command(BackendCommand::MoveRelative {
                            device_id: device.id.clone(),
                            distance: rel_state.float_value,
                        });
                    }
                    ui.end_row();
                });
            ui.separator();
        }

        // 2. Parameters (Tree View)
        ui.horizontal(|ui| {
            ui.heading("Parameters");
            if ui.button("↻").clicked() {
                channels.send_command(BackendCommand::FetchParameters {
                    device_id: device.id.clone(),
                });
            }
        });

        if device.parameters.is_empty() {
            ui.label("No parameters available.");
        } else {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    // Organize parameters into a tree structure based on dot notation
                    // e.g. "laser.diode.current" -> Root -> laser -> diode -> current

                    // Optimization: Pre-sort or cache this structure if it gets too slow (unlikely for <100 params)
                    // For now, straightforward immediate rendering.

                    // We'll separate "root" params (no dots) from grouped params
                    let mut root_params = Vec::new();
                    let mut groups: HashMap<String, Vec<&ParameterDescriptor>> = HashMap::new();

                    for param in &device.parameters {
                        if let Some((group, _)) = param.name.split_once('.') {
                            // Simple 1-level grouping for now to handle "laser.power"
                            // Supporting arbitrary depth text recursion in immediate mode is tricky without a recursive helper
                            groups.entry(group.to_string()).or_default().push(param);
                        } else {
                            root_params.push(param);
                        }
                    }

                    if !root_params.is_empty() {
                        // Render root params
                        egui::Grid::new(format!("grid_root_{}", device.id))
                            .striped(true)
                            .show(ui, |ui| {
                                for param in root_params {
                                    render_param_row(ui, param, states, channels);
                                }
                            });
                    }

                    // Render groups in alphabetical order
                    let mut sorted_groups: Vec<_> = groups.keys().collect();
                    sorted_groups.sort();

                    for group in sorted_groups {
                        if let Some(params) = groups.get(group) {
                            egui::CollapsingHeader::new(group)
                                .default_open(false)
                                .show(ui, |ui| {
                                    egui::Grid::new(format!("grid_{}_{}", device.id, group))
                                        .striped(true)
                                        .show(ui, |ui| {
                                            for param in params {
                                                render_param_row(ui, param, states, channels);
                                            }
                                        });
                                });
                        }
                    }
                });
        }

        // 3. Raw State Fields (Debug/Info)
        if !device.state_fields.is_empty() {
            ui.separator();
            ui.collapsing("State Details", |ui| {
                egui::Grid::new(format!("state_{}", device.id))
                    .striped(true)
                    .show(ui, |ui| {
                        for (k, v) in &device.state_fields {
                            ui.label(k);
                            ui.label(v);
                            ui.end_row();
                        }
                    });
            });
        }
    }
}

// Helper to render a single parameter row
fn render_param_row(
    ui: &mut egui::Ui,
    param: &ParameterDescriptor,
    states: &mut HashMap<String, ParameterEditState>,
    channels: &mut UiChannels,
) {
    ui.label(&param.name).on_hover_text(&param.description);

    let state_key = format!("{}:{}", param.device_id, param.name);
    let state = states.entry(state_key).or_default();

    match parameter_widget(ui, param, state) {
        WidgetResult::Committed(value) => {
            channels.send_command(BackendCommand::SetParameter {
                device_id: param.device_id.clone(),
                name: param.name.clone(),
                value,
            });
            state.reset();
        }
        _ => {}
    }

    if !param.writable {
        ui.label("🔒");
    } else {
        ui.label("");
    }
    ui.end_row();
}

impl App for DaqGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // Auto-connect on first frame for debugging
        if self.first_frame {
            log::info!("Auto-connecting on first frame...");
            self.first_frame = false;
            let address = self.url.clone();
            self.channels
                .send_command(BackendCommand::Connect { address });
        }

        self.ui(ctx);
    }
}
