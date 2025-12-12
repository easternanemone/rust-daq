//! Main application state and UI logic.

use eframe::egui;

use crate::client::DaqClient;
use crate::panels::{ConnectionPanel, DevicesPanel, ScriptsPanel, ScansPanel, StoragePanel, ModulesPanel};

/// Connection state to the DAQ daemon
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// Main application state
pub struct DaqApp {
    /// gRPC client (wrapped in Option for lazy initialization)
    client: Option<DaqClient>,
    
    /// Current connection state
    connection_state: ConnectionState,
    
    /// Target daemon address
    daemon_address: String,
    
    /// Active panel/tab
    active_panel: Panel,
    
    /// Panel states
    connection_panel: ConnectionPanel,
    devices_panel: DevicesPanel,
    scripts_panel: ScriptsPanel,
    scans_panel: ScansPanel,
    storage_panel: StoragePanel,
    modules_panel: ModulesPanel,
    
    /// Tokio runtime for async operations
    runtime: tokio::runtime::Runtime,
    #[cfg(all(feature = "rerun_viewer", feature = "instrument_photometrics", feature = "pvcam_hardware"))]
    pvcam_streaming: bool,
    #[cfg(all(feature = "rerun_viewer", feature = "instrument_photometrics", feature = "pvcam_hardware"))]
    pvcam_task: Option<tokio::task::JoinHandle<()>>,
}

/// Available panels in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Devices,
    Scripts,
    Scans,
    Storage,
    Modules,
}

impl DaqApp {
    /// Create a new application instance
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure egui style
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        cc.egui_ctx.set_style(style);

        // Create tokio runtime for gRPC calls
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        Self {
            client: None,
            connection_state: ConnectionState::Disconnected,
            daemon_address: "http://127.0.0.1:50051".to_string(),
            active_panel: Panel::Devices,
            connection_panel: ConnectionPanel::default(),
            devices_panel: DevicesPanel::default(),
            scripts_panel: ScriptsPanel::default(),
            scans_panel: ScansPanel::default(),
            storage_panel: StoragePanel::default(),
            modules_panel: ModulesPanel::default(),
            runtime,
            #[cfg(all(feature = "rerun_viewer", feature = "instrument_photometrics", feature = "pvcam_hardware"))]
            pvcam_streaming: false,
            #[cfg(all(feature = "rerun_viewer", feature = "instrument_photometrics", feature = "pvcam_hardware"))]
            pvcam_task: None,
        }
    }

    /// Attempt to connect to the daemon
    fn connect(&mut self) {
        self.connection_state = ConnectionState::Connecting;
        let address = self.daemon_address.clone();
        
        match self.runtime.block_on(DaqClient::connect(&address)) {
            Ok(client) => {
                self.client = Some(client);
                self.connection_state = ConnectionState::Connected;
                tracing::info!("Connected to daemon at {}", address);
            }
            Err(e) => {
                self.connection_state = ConnectionState::Error(e.to_string());
                tracing::error!("Failed to connect: {}", e);
            }
        }
    }

    /// Disconnect from the daemon
    fn disconnect(&mut self) {
        self.client = None;
        self.connection_state = ConnectionState::Disconnected;
        tracing::info!("Disconnected from daemon");
    }

    /// Render the top menu bar
    fn render_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                
                ui.menu_button("View", |ui| {
                    if ui.button("Devices").clicked() {
                        self.active_panel = Panel::Devices;
                        ui.close_menu();
                    }
                    if ui.button("Scripts").clicked() {
                        self.active_panel = Panel::Scripts;
                        ui.close_menu();
                    }
                    if ui.button("Scans").clicked() {
                        self.active_panel = Panel::Scans;
                        ui.close_menu();
                    }
                    if ui.button("Storage").clicked() {
                        self.active_panel = Panel::Storage;
                        ui.close_menu();
                    }
                    if ui.button("Modules").clicked() {
                        self.active_panel = Panel::Modules;
                        ui.close_menu();
                    }
                });
            });
        });
    }

    /// Render the connection status bar
    fn render_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Connection status indicator
                let (color, text) = match &self.connection_state {
                    ConnectionState::Disconnected => (egui::Color32::GRAY, "Disconnected"),
                    ConnectionState::Connecting => (egui::Color32::YELLOW, "Connecting..."),
                    ConnectionState::Connected => (egui::Color32::GREEN, "Connected"),
                    ConnectionState::Error(_) => (egui::Color32::RED, "Error"),
                };
                
                ui.colored_label(color, "●");
                ui.label(text);
                
                ui.separator();
                
                // Address input
                ui.label("Daemon:");
                let response = ui.add_sized(
                    [200.0, 18.0],
                    egui::TextEdit::singleline(&mut self.daemon_address)
                        .hint_text("http://127.0.0.1:50051"),
                );
                
                // Connect/Disconnect button
                match &self.connection_state {
                    ConnectionState::Disconnected | ConnectionState::Error(_) => {
                        if ui.button("Connect").clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                            self.connect();
                        }
                    }
                    ConnectionState::Connected => {
                        if ui.button("Disconnect").clicked() {
                            self.disconnect();
                        }
                    }
                    ConnectionState::Connecting => {
                        ui.spinner();
                    }
                }
                
                // Show error message if any
                if let ConnectionState::Error(msg) = &self.connection_state {
                    ui.separator();
                    ui.colored_label(egui::Color32::RED, msg);
                }
            });
        });
    }

    /// Render the left navigation panel
    fn render_nav_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("nav_panel")
            .resizable(false)
            .default_width(120.0)
            .show(ctx, |ui| {
                ui.heading("Navigation");
                ui.separator();
                
                ui.vertical(|ui| {
                    ui.selectable_value(&mut self.active_panel, Panel::Devices, "🔧 Devices");
                    ui.selectable_value(&mut self.active_panel, Panel::Scripts, "📜 Scripts");
                    ui.selectable_value(&mut self.active_panel, Panel::Scans, "📊 Scans");
                    ui.selectable_value(&mut self.active_panel, Panel::Storage, "💾 Storage");
                    ui.selectable_value(&mut self.active_panel, Panel::Modules, "🧩 Modules");
                });
                
                ui.separator();
                ui.add_space(8.0);

                // Rerun visualization button
                if ui.button("📈 Open Rerun").clicked() {
                    // Launch Rerun viewer
                    let _ = std::process::Command::new("rerun")
                        .spawn();
                }

                // PVCAM live view via Rerun
                #[cfg(all(feature = "rerun_viewer", feature = "instrument_photometrics", feature = "pvcam_hardware"))]
                {
                    ui.add_space(8.0);
                    let label = if self.pvcam_streaming { "🛑 Stop PVCAM Live" } else { "🎥 PVCAM Live to Rerun" };
                    if ui.button(label).clicked() {
                        if self.pvcam_streaming {
                            if let Some(task) = self.pvcam_task.take() {
                                task.abort();
                            }
                            self.pvcam_streaming = false;
                        } else {
                            self.start_pvcam_stream();
                        }
                    }
                }
            });
    }

    /// Render the main content area
    fn render_content(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_panel {
                Panel::Devices => {
                    self.devices_panel.ui(ui, self.client.as_mut(), &self.runtime);
                }
                Panel::Scripts => {
                    self.scripts_panel.ui(ui, self.client.as_mut(), &self.runtime);
                }
                Panel::Scans => {
                    self.scans_panel.ui(ui, self.client.as_mut(), &self.runtime);
                }
                Panel::Storage => {
                    self.storage_panel.ui(ui, self.client.as_mut(), &self.runtime);
                }
                Panel::Modules => {
                    self.modules_panel.ui(ui, self.client.as_mut(), &self.runtime);
                }
            }
        });
    }

    #[cfg(all(feature = "rerun_viewer", feature = "instrument_photometrics", feature = "pvcam_hardware"))]
    fn start_pvcam_stream(&mut self) {
        use rerun::archetypes::Tensor;
        use rerun::components::TensorData;
        use rerun::RecordingStreamBuilder;
        use rust_daq::hardware::pvcam::PvcamDriver;

        let handle = self.runtime.handle().clone();
        self.pvcam_task = Some(handle.spawn(async move {
            // Connect PVCAM driver and open rerun stream
            let driver = match PvcamDriver::new_async("PrimeBSI".to_string()).await {
                Ok(d) => d,
                Err(err) => {
                    eprintln!("PVCAM init failed: {err}");
                    return;
                }
            };

            let mut rx = match driver.subscribe_frames().await {
                Some(r) => r,
                None => {
                    eprintln!("PVCAM frame subscription unavailable");
                    return;
                }
            };

            if let Err(err) = driver.start_stream().await {
                eprintln!("PVCAM start_stream failed: {err}");
                return;
            }

            let rec = match RecordingStreamBuilder::new("PVCAM Live")
                .connect("127.0.0.1:9876")
            {
                Ok(r) => r,
                Err(err) => {
                    eprintln!("Failed to connect to rerun viewer: {err}");
                    let _ = driver.stop_stream().await;
                    return;
                }
            };

            while let Ok(frame_arc) = rx.recv().await {
                let frame = frame_arc.as_ref();
                if frame.bit_depth != 16 {
                    continue;
                }
                let td = match TensorData::try_from_rows_cols(
                    frame.height as usize,
                    frame.width as usize,
                    bytemuck::cast_slice::<u8, u16>(&frame.data),
                ) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let tensor = Tensor::new(td);
                let _ = rec.log("/pvcam/image", &tensor);
            }

            let _ = driver.stop_stream().await;
        }));

        self.pvcam_streaming = true;
    }
}

impl eframe::App for DaqApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_menu_bar(ctx);
        self.render_status_bar(ctx);
        self.render_nav_panel(ctx);
        self.render_content(ctx);
    }
}
