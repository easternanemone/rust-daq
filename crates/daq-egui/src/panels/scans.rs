//! Scans panel - configure and monitor multi-axis scans.

use eframe::egui;
use tokio::runtime::Runtime;

use crate::client::DaqClient;
use daq_proto::daq::{ScanConfig, AxisConfig};

/// Axis configuration for the wizard
#[derive(Clone)]
struct AxisWizardConfig {
    device_id: String,
    start: f64,
    end: f64,
    num_points: u32,
}

impl Default for AxisWizardConfig {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            start: 0.0,
            end: 10.0,
            num_points: 11,
        }
    }
}

/// Pending action for scans panel
enum PendingAction {
    Refresh,
    CreateScan,
    StartScan { scan_id: String },
    PauseScan { scan_id: String },
    ResumeScan { scan_id: String },
    StopScan { scan_id: String },
}

/// Scans panel state
#[derive(Default)]
pub struct ScansPanel {
    /// Cached scan list
    scans: Vec<daq_proto::daq::ScanStatus>,
    /// Available devices (for wizard)
    devices: Vec<daq_proto::daq::DeviceInfo>,
    /// Last refresh timestamp
    last_refresh: Option<std::time::Instant>,
    /// Error message
    error: Option<String>,
    /// Status message
    status: Option<String>,
    /// Pending action
    pending_action: Option<PendingAction>,
    
    // Scan wizard state
    /// Show wizard
    show_wizard: bool,
    /// Scan name
    wizard_name: String,
    /// Scan type
    wizard_scan_type: i32,
    /// Axes configuration
    wizard_axes: Vec<AxisWizardConfig>,
    /// Acquire devices
    wizard_acquire_devices: Vec<String>,
    /// Dwell time in ms
    wizard_dwell_ms: f64,
    /// Triggers per point
    wizard_triggers_per_point: u32,
}

impl ScansPanel {
    /// Render the scans panel
    pub fn ui(&mut self, ui: &mut egui::Ui, client: Option<&mut DaqClient>, runtime: &Runtime) {
        self.pending_action = None;
        
        ui.heading("Scans");
        
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh").clicked() {
                self.pending_action = Some(PendingAction::Refresh);
            }
            
            if ui.button("➕ New Scan").clicked() {
                self.show_wizard = true;
                self.wizard_name = format!("scan_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
                self.wizard_scan_type = 1; // LINE_SCAN
                self.wizard_axes = vec![AxisWizardConfig::default()];
                self.wizard_acquire_devices.clear();
                self.wizard_dwell_ms = 100.0;
                self.wizard_triggers_per_point = 1;
            }
            
            if let Some(last) = self.last_refresh {
                let elapsed = last.elapsed();
                ui.label(format!("Updated {}s ago", elapsed.as_secs()));
            }
        });
        
        ui.separator();
        
        // Show error/status messages
        if let Some(err) = &self.error {
            ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
        }
        if let Some(status) = &self.status {
            ui.colored_label(egui::Color32::GREEN, status);
        }
        
        ui.add_space(8.0);
        
        // Scan wizard
        if self.show_wizard {
            self.render_wizard(ui);
            ui.separator();
        }
        
        // Scan list
        if self.scans.is_empty() {
            ui.label("No scans found. Click 'New Scan' to create one.");
        } else {
            egui::ScrollArea::vertical()
                .id_salt("scans_list")
                .show(ui, |ui| {
                    // Clone scans to avoid borrow issues
                    let scans: Vec<_> = self.scans.clone();
                    for scan in &scans {
                        self.render_scan_card(ui, scan);
                    }
                });
        }
        
        // Execute pending action
        if let Some(action) = self.pending_action.take() {
            self.execute_action(action, client, runtime);
        }
    }
    
    /// Render the scan creation wizard
    fn render_wizard(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Create New Scan");
            
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.wizard_name);
            });
            
            ui.horizontal(|ui| {
                ui.label("Scan Type:");
                egui::ComboBox::from_id_salt("scan_type")
                    .selected_text(match self.wizard_scan_type {
                        1 => "Line Scan (1D)",
                        2 => "Grid Scan (2D)",
                        3 => "Snake Scan (2D bidirectional)",
                        _ => "Unknown",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.wizard_scan_type, 1, "Line Scan (1D)");
                        ui.selectable_value(&mut self.wizard_scan_type, 2, "Grid Scan (2D)");
                        ui.selectable_value(&mut self.wizard_scan_type, 3, "Snake Scan (2D)");
                    });
            });
            
            ui.separator();
            ui.label("Scan Axes:");
            
            // Axis configurations - use index-based iteration to avoid borrow issues
            let num_axes = self.wizard_axes.len();
            let mut remove_idx = None;
            
            for i in 0..num_axes {
                let axis = &mut self.wizard_axes[i];
                ui.horizontal(|ui| {
                    ui.label(format!("Axis {}:", i + 1));
                    
                    ui.label("Device:");
                    ui.add_sized([120.0, 18.0], egui::TextEdit::singleline(&mut axis.device_id));
                    
                    ui.label("Start:");
                    ui.add(egui::DragValue::new(&mut axis.start).speed(0.1));
                    
                    ui.label("End:");
                    ui.add(egui::DragValue::new(&mut axis.end).speed(0.1));
                    
                    ui.label("Points:");
                    ui.add(egui::DragValue::new(&mut axis.num_points).range(2..=10000));
                    
                    if num_axes > 1 && ui.button("✕").clicked() {
                        remove_idx = Some(i);
                    }
                });
            }
            
            if let Some(idx) = remove_idx {
                self.wizard_axes.remove(idx);
            }
            
            if self.wizard_axes.len() < 3 && ui.button("➕ Add Axis").clicked() {
                self.wizard_axes.push(AxisWizardConfig::default());
            }
            
            ui.separator();
            ui.label("Acquisition Settings:");
            
            ui.horizontal(|ui| {
                ui.label("Dwell time (ms):");
                ui.add(egui::DragValue::new(&mut self.wizard_dwell_ms).range(0.0..=10000.0));
            });
            
            ui.horizontal(|ui| {
                ui.label("Triggers per point:");
                ui.add(egui::DragValue::new(&mut self.wizard_triggers_per_point).range(1..=1000));
            });
            
            ui.separator();
            
            ui.horizontal(|ui| {
                let can_create = !self.wizard_axes.is_empty() 
                    && self.wizard_axes.iter().all(|a| !a.device_id.is_empty());
                
                if ui.add_enabled(can_create, egui::Button::new("✓ Create Scan")).clicked() {
                    self.pending_action = Some(PendingAction::CreateScan);
                }
                
                if ui.button("✕ Cancel").clicked() {
                    self.show_wizard = false;
                }
            });
        });
    }
    
    /// Render a single scan as a card
    fn render_scan_card(&mut self, ui: &mut egui::Ui, scan: &daq_proto::daq::ScanStatus) {
        let state_color = match scan.state {
            1 => egui::Color32::GRAY,    // CREATED
            2 => egui::Color32::YELLOW,  // RUNNING
            3 => egui::Color32::BLUE,    // PAUSED
            4 => egui::Color32::GREEN,   // COMPLETED
            5 => egui::Color32::GRAY,    // STOPPED
            6 => egui::Color32::RED,     // ERROR
            _ => egui::Color32::WHITE,
        };
        
        let state_name = match scan.state {
            1 => "Created",
            2 => "Running",
            3 => "Paused",
            4 => "Completed",
            5 => "Stopped",
            6 => "Error",
            _ => "Unknown",
        };
        
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.colored_label(state_color, "●");
                ui.strong(&scan.scan_id);
                ui.label(format!("- {}", state_name));
            });
            
            // Progress bar
            if scan.total_points > 0 {
                let progress = scan.current_point as f32 / scan.total_points as f32;
                let progress_bar = egui::ProgressBar::new(progress)
                    .text(format!("{}/{} points ({:.1}%)", 
                        scan.current_point, 
                        scan.total_points,
                        scan.progress_percent
                    ));
                ui.add(progress_bar);
            }
            
            // Control buttons based on state
            ui.horizontal(|ui| {
                match scan.state {
                    1 => {
                        // Created - can start
                        if ui.button("▶ Start").clicked() {
                            self.pending_action = Some(PendingAction::StartScan {
                                scan_id: scan.scan_id.clone(),
                            });
                        }
                    }
                    2 => {
                        // Running - can pause/stop
                        if ui.button("⏸ Pause").clicked() {
                            self.pending_action = Some(PendingAction::PauseScan {
                                scan_id: scan.scan_id.clone(),
                            });
                        }
                        if ui.button("⏹ Stop").clicked() {
                            self.pending_action = Some(PendingAction::StopScan {
                                scan_id: scan.scan_id.clone(),
                            });
                        }
                    }
                    3 => {
                        // Paused - can resume/stop
                        if ui.button("▶ Resume").clicked() {
                            self.pending_action = Some(PendingAction::ResumeScan {
                                scan_id: scan.scan_id.clone(),
                            });
                        }
                        if ui.button("⏹ Stop").clicked() {
                            self.pending_action = Some(PendingAction::StopScan {
                                scan_id: scan.scan_id.clone(),
                            });
                        }
                    }
                    _ => {}
                }
            });
            
            // Error message
            if !scan.error_message.is_empty() {
                ui.colored_label(egui::Color32::RED, &scan.error_message);
            }
        });
    }
    
    /// Execute a pending action
    fn execute_action(
        &mut self,
        action: PendingAction,
        client: Option<&mut DaqClient>,
        runtime: &Runtime,
    ) {
        match action {
            PendingAction::Refresh => self.refresh(client, runtime),
            PendingAction::CreateScan => self.create_scan(client, runtime),
            PendingAction::StartScan { scan_id } => self.start_scan(client, runtime, &scan_id),
            PendingAction::PauseScan { scan_id } => self.pause_scan(client, runtime, &scan_id),
            PendingAction::ResumeScan { scan_id } => self.resume_scan(client, runtime, &scan_id),
            PendingAction::StopScan { scan_id } => self.stop_scan(client, runtime, &scan_id),
        }
    }
    
    /// Refresh the scan list
    fn refresh(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime) {
        self.error = None;
        self.status = None;
        
        let Some(client) = client else {
            self.error = Some("Not connected to daemon".to_string());
            return;
        };
        
        let mut client = client.clone();
        match runtime.block_on(async {
            let scans = client.list_scans().await?;
            let devices = client.list_devices().await.unwrap_or_default();
            Ok::<_, anyhow::Error>((scans, devices))
        }) {
            Ok((scans, devices)) => {
                self.scans = scans;
                self.devices = devices;
                self.last_refresh = Some(std::time::Instant::now());
                self.status = Some(format!("Loaded {} scans", self.scans.len()));
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }
    
    /// Create a new scan from wizard config
    fn create_scan(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime) {
        self.error = None;
        self.status = None;
        
        let Some(client) = client else {
            self.error = Some("Not connected to daemon".to_string());
            return;
        };
        
        // Build scan config
        let axes: Vec<AxisConfig> = self.wizard_axes.iter().map(|a| AxisConfig {
            device_id: a.device_id.clone(),
            start_position: a.start,
            end_position: a.end,
            num_points: a.num_points,
        }).collect();
        
        let config = ScanConfig {
            axes,
            scan_type: self.wizard_scan_type,
            acquire_device_ids: self.wizard_acquire_devices.clone(),
            triggers_per_point: self.wizard_triggers_per_point,
            dwell_time_ms: self.wizard_dwell_ms,
            camera_device_id: None,
            arm_camera: None,
            name: self.wizard_name.clone(),
            metadata: Default::default(),
        };
        
        let mut client = client.clone();
        match runtime.block_on(client.create_scan(config)) {
            Ok(response) => {
                if response.success {
                    self.status = Some(format!(
                        "Created scan: {} ({} points)",
                        response.scan_id, response.total_points
                    ));
                    self.show_wizard = false;
                } else {
                    self.error = Some(response.error_message);
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }
    
    /// Start a scan
    fn start_scan(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime, scan_id: &str) {
        self.error = None;
        
        let Some(client) = client else {
            self.error = Some("Not connected to daemon".to_string());
            return;
        };
        
        let mut client = client.clone();
        let scan_id = scan_id.to_string();
        
        match runtime.block_on(client.start_scan(&scan_id)) {
            Ok(response) => {
                if response.success {
                    self.status = Some(format!("Started scan: {}", scan_id));
                } else {
                    self.error = Some(response.error_message);
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }
    
    /// Pause a scan
    fn pause_scan(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime, scan_id: &str) {
        self.error = None;
        
        let Some(client) = client else {
            self.error = Some("Not connected to daemon".to_string());
            return;
        };
        
        let mut client = client.clone();
        let scan_id = scan_id.to_string();
        
        match runtime.block_on(client.pause_scan(&scan_id)) {
            Ok(response) => {
                if response.success {
                    self.status = Some(format!("Paused scan at point {}", response.paused_at_point));
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }
    
    /// Resume a scan
    fn resume_scan(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime, scan_id: &str) {
        self.error = None;
        
        let Some(client) = client else {
            self.error = Some("Not connected to daemon".to_string());
            return;
        };
        
        let mut client = client.clone();
        let scan_id = scan_id.to_string();
        
        match runtime.block_on(client.resume_scan(&scan_id)) {
            Ok(response) => {
                if response.success {
                    self.status = Some(format!("Resumed scan: {}", scan_id));
                } else {
                    self.error = Some(response.error_message);
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }
    
    /// Stop a scan
    fn stop_scan(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime, scan_id: &str) {
        self.error = None;
        
        let Some(client) = client else {
            self.error = Some("Not connected to daemon".to_string());
            return;
        };
        
        let mut client = client.clone();
        let scan_id = scan_id.to_string();
        
        match runtime.block_on(client.stop_scan(&scan_id, false)) {
            Ok(response) => {
                if response.success {
                    self.status = Some(format!(
                        "Stopped scan: {} ({} points completed)",
                        scan_id, response.points_completed
                    ));
                } else {
                    self.error = Some(response.error_message);
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }
}
