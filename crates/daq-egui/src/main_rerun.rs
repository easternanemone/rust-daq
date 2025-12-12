//! DAQ Control Panel with embedded Rerun Viewer
//!
//! This binary embeds the Rerun viewer alongside DAQ control panels.
//! Run with: cargo run --bin daq-rerun --features rerun_viewer

#[cfg(feature = "rerun_viewer")]
mod rerun_app {
    use rerun::external::{eframe, egui, re_viewer, re_log, re_crash_handler, re_memory, re_grpc_server, tokio};
    use daq_egui::client::DaqClient;

    // Use Rerun's memory tracking allocator
    #[global_allocator]
    static GLOBAL: re_memory::AccountingAllocator<mimalloc::MiMalloc> =
        re_memory::AccountingAllocator::new(mimalloc::MiMalloc);

    /// DAQ Control state shared with the viewer
    pub struct DaqControlState {
        pub daemon_address: String,
        pub connection_status: String,
        pub devices: Vec<DeviceInfo>,
        pub selected_device: Option<usize>,
        pub move_target: f64,
        pub status_message: Option<String>,
        pub error_message: Option<String>,
    }

    #[derive(Clone)]
    pub struct DeviceInfo {
        pub id: String,
        pub name: String,
        pub driver: String,
        pub is_movable: bool,
        pub is_readable: bool,
        pub position: Option<f64>,
        pub reading: Option<f64>,
    }

    impl Default for DaqControlState {
        fn default() -> Self {
            Self {
                daemon_address: "http://127.0.0.1:50051".to_string(),
                connection_status: "Disconnected".to_string(),
                devices: Vec::new(),
                selected_device: None,
                move_target: 0.0,
                status_message: None,
                error_message: None,
            }
        }
    }

    /// App that wraps Rerun viewer with DAQ control panels
    pub struct DaqRerunApp {
        rerun_app: re_viewer::App,
        daq_state: DaqControlState,
        client: Option<DaqClient>,
        runtime: tokio::runtime::Runtime,
    }

    impl DaqRerunApp {
        pub fn new(
            cc: &eframe::CreationContext<'_>,
            rerun_app: re_viewer::App,
        ) -> Self {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            Self {
                rerun_app,
                daq_state: DaqControlState::default(),
                client: None,
                runtime,
            }
        }

        fn connect(&mut self) {
            let address = self.daq_state.daemon_address.clone();
            match self.runtime.block_on(DaqClient::connect(&address)) {
                Ok(client) => {
                    self.client = Some(client);
                    self.daq_state.connection_status = "Connected".to_string();
                    self.daq_state.status_message = Some("Connected to daemon".to_string());
                    self.daq_state.error_message = None;
                }
                Err(e) => {
                    self.daq_state.connection_status = "Error".to_string();
                    self.daq_state.error_message = Some(e.to_string());
                }
            }
        }

        fn disconnect(&mut self) {
            self.client = None;
            self.daq_state.connection_status = "Disconnected".to_string();
            self.daq_state.devices.clear();
        }

        fn refresh_devices(&mut self) {
            let Some(client) = &mut self.client else {
                self.daq_state.error_message = Some("Not connected".to_string());
                return;
            };

            let mut client = client.clone();
            match self.runtime.block_on(async {
                let devices = client.list_devices().await?;
                let mut infos = Vec::new();
                for d in devices {
                    let state = client.get_device_state(&d.id).await.ok();
                    infos.push(DeviceInfo {
                        id: d.id,
                        name: d.name,
                        driver: d.driver_type,
                        is_movable: d.is_movable,
                        is_readable: d.is_readable,
                        position: state.as_ref().and_then(|s| s.position),
                        reading: state.as_ref().and_then(|s| s.last_reading),
                    });
                }
                Ok::<_, anyhow::Error>(infos)
            }) {
                Ok(devices) => {
                    self.daq_state.status_message = Some(format!("Loaded {} devices", devices.len()));
                    self.daq_state.devices = devices;
                    self.daq_state.error_message = None;
                }
                Err(e) => {
                    self.daq_state.error_message = Some(e.to_string());
                }
            }
        }

        fn move_device(&mut self, device_id: &str, value: f64, relative: bool) {
            let Some(client) = &mut self.client else { return };
            
            let mut client = client.clone();
            let device_id = device_id.to_string();
            
            let result = self.runtime.block_on(async {
                if relative {
                    client.move_relative(&device_id, value).await
                } else {
                    client.move_absolute(&device_id, value).await
                }
            });

            match result {
                Ok(response) if response.success => {
                    self.daq_state.status_message = Some(format!("Moved to {:.4}", response.final_position));
                    // Update device position
                    if let Some(dev) = self.daq_state.devices.iter_mut().find(|d| d.id == device_id) {
                        dev.position = Some(response.final_position);
                    }
                }
                Ok(response) => {
                    self.daq_state.error_message = Some(response.error_message);
                }
                Err(e) => {
                    self.daq_state.error_message = Some(e.to_string());
                }
            }
        }

        fn read_device(&mut self, device_id: &str) {
            let Some(client) = &mut self.client else { return };
            
            let mut client = client.clone();
            let device_id = device_id.to_string();
            
            match self.runtime.block_on(client.read_value(&device_id)) {
                Ok(response) if response.success => {
                    self.daq_state.status_message = Some(format!("{}: {:.4} {}", device_id, response.value, response.units));
                    if let Some(dev) = self.daq_state.devices.iter_mut().find(|d| d.id == device_id) {
                        dev.reading = Some(response.value);
                    }
                }
                Ok(response) => {
                    self.daq_state.error_message = Some(response.error_message);
                }
                Err(e) => {
                    self.daq_state.error_message = Some(e.to_string());
                }
            }
        }

        /// Render DAQ control panel
        fn daq_control_panel(&mut self, ui: &mut egui::Ui) {
            ui.heading("DAQ Control");
            
            // Connection
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Daemon:");
                    ui.text_edit_singleline(&mut self.daq_state.daemon_address);
                });
                
                ui.horizontal(|ui| {
                    let connected = self.client.is_some();
                    let status_color = if connected { egui::Color32::GREEN } else { egui::Color32::GRAY };
                    ui.colored_label(status_color, "●");
                    ui.label(&self.daq_state.connection_status);
                    
                    if connected {
                        if ui.button("Disconnect").clicked() {
                            self.disconnect();
                        }
                    } else {
                        if ui.button("Connect").clicked() {
                            self.connect();
                        }
                    }
                });
            });

            // Messages
            if let Some(err) = &self.daq_state.error_message {
                ui.colored_label(egui::Color32::RED, err);
            }
            if let Some(status) = &self.daq_state.status_message {
                ui.colored_label(egui::Color32::GREEN, status);
            }

            ui.separator();

            // Devices
            if ui.button("🔄 Refresh Devices").clicked() {
                self.refresh_devices();
            }

            if self.daq_state.devices.is_empty() {
                ui.label("No devices");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        let devices = self.daq_state.devices.clone();
                        for (i, device) in devices.iter().enumerate() {
                            let selected = self.daq_state.selected_device == Some(i);
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(selected, &device.name).clicked() {
                                        self.daq_state.selected_device = Some(i);
                                    }
                                    ui.label(format!("({})", device.driver));
                                });
                                
                                if let Some(pos) = device.position {
                                    ui.label(format!("Position: {:.4}", pos));
                                }
                                if let Some(reading) = device.reading {
                                    ui.label(format!("Reading: {:.4}", reading));
                                }

                                ui.horizontal(|ui| {
                                    if device.is_movable {
                                        ui.add(egui::DragValue::new(&mut self.daq_state.move_target).speed(0.1));
                                        if ui.small_button("Go").clicked() {
                                            self.move_device(&device.id, self.daq_state.move_target, false);
                                        }
                                        for delta in [-1.0, -0.1, 0.1, 1.0] {
                                            let label = if delta > 0.0 { format!("+{}", delta) } else { format!("{}", delta) };
                                            if ui.small_button(label).clicked() {
                                                self.move_device(&device.id, delta, true);
                                            }
                                        }
                                    }
                                    if device.is_readable {
                                        if ui.small_button("📖").clicked() {
                                            self.read_device(&device.id);
                                        }
                                    }
                                });
                            });
                        }
                    });
            }
        }
    }

    impl eframe::App for DaqRerunApp {
        fn save(&mut self, storage: &mut dyn eframe::Storage) {
            self.rerun_app.save(storage);
        }

        fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
            // Left panel for DAQ controls
            egui::SidePanel::left("daq_control_panel")
                .default_width(300.0)
                .resizable(true)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.daq_control_panel(ui);
                    });
                });

            // Rest is Rerun viewer
            self.rerun_app.update(ctx, frame);
        }
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let main_thread_token = re_viewer::MainThreadToken::i_promise_i_am_on_the_main_thread();

        re_log::setup_logging();
        re_crash_handler::install_crash_handlers(re_viewer::build_info());

        // Create tokio runtime for the gRPC server
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        
        // Listen for gRPC connections from logging SDKs (including our daemon)
        // Must be called from within tokio runtime context
        let (data_rx, _table_rx) = runtime.block_on(async {
            re_grpc_server::spawn_with_recv(
                "0.0.0.0:9876".parse().unwrap(),
                Default::default(),
                re_grpc_server::shutdown::never(),
            )
        });

        let mut native_options = re_viewer::native::eframe_options(None);
        native_options.viewport = native_options
            .viewport
            .with_app_id("daq_control_panel")
            .with_inner_size([1400.0, 900.0]);

        let startup_options = re_viewer::StartupOptions::default();
        let app_env = re_viewer::AppEnvironment::Custom("DAQ Control Panel".to_owned());

        // Keep runtime alive
        let _runtime_guard = runtime.enter();

        eframe::run_native(
            "DAQ Control Panel + Rerun",
            native_options,
            Box::new(move |cc| {
                re_viewer::customize_eframe_and_setup_renderer(cc)?;

                let mut rerun_app = re_viewer::App::new(
                    main_thread_token,
                    re_viewer::build_info(),
                    app_env,
                    startup_options,
                    cc,
                    None,
                    re_viewer::AsyncRuntimeHandle::from_current_tokio_runtime_or_wasmbindgen()?,
                );
                rerun_app.add_log_receiver(data_rx);
                
                Ok(Box::new(DaqRerunApp::new(cc, rerun_app)))
            }),
        )?;

        Ok(())
    }
}

#[cfg(feature = "rerun_viewer")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    rerun_app::run()
}

#[cfg(not(feature = "rerun_viewer"))]
fn main() {
    eprintln!("This binary requires the 'rerun_viewer' feature.");
    eprintln!("Run with: cargo run --bin daq-rerun --features rerun_viewer");
    std::process::exit(1);
}
