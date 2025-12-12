//! Modules panel - experiment module management.

use eframe::egui;
use tokio::runtime::Runtime;

use crate::client::DaqClient;

/// Pending action for modules panel
enum PendingAction {
    Refresh,
    CreateModule { type_id: String, name: String },
    StartModule { module_id: String },
    StopModule { module_id: String },
}

/// Modules panel state
#[derive(Default)]
pub struct ModulesPanel {
    /// Available module types
    module_types: Vec<daq_proto::daq::ModuleTypeSummary>,
    /// Active module instances
    modules: Vec<daq_proto::daq::ModuleStatus>,
    /// Selected module type for creation
    selected_type: Option<String>,
    /// New module name input
    new_module_name: String,
    /// Selected module instance
    selected_module: Option<String>,
    /// Last refresh timestamp
    last_refresh: Option<std::time::Instant>,
    /// Error message
    error: Option<String>,
    /// Status message
    status: Option<String>,
    /// Pending action
    pending_action: Option<PendingAction>,
}

impl ModulesPanel {
    /// Render the modules panel
    pub fn ui(&mut self, ui: &mut egui::Ui, client: Option<&mut DaqClient>, runtime: &Runtime) {
        self.pending_action = None;
        
        ui.heading("Modules");
        
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh").clicked() {
                self.pending_action = Some(PendingAction::Refresh);
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
        
        // Two-column layout
        ui.columns(2, |columns| {
            // Left column: Module types and creation
            columns[0].heading("Module Types");
            columns[0].separator();
            
            if self.module_types.is_empty() {
                columns[0].label("No module types available. Click Refresh.");
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("module_types")
                    .max_height(200.0)
                    .show(&mut columns[0], |ui| {
                        for mt in &self.module_types {
                            let selected = self.selected_type.as_ref() == Some(&mt.type_id);
                            let label = format!("{} ({})", mt.display_name, mt.type_id);
                            
                            if ui.selectable_label(selected, &label).clicked() {
                                self.selected_type = Some(mt.type_id.clone());
                            }
                            
                            if !mt.description.is_empty() {
                                ui.indent(mt.type_id.clone(), |ui| {
                                    ui.label(egui::RichText::new(&mt.description).small().weak());
                                });
                            }
                        }
                    });
            }
            
            columns[0].add_space(8.0);
            
            // Create module section
            columns[0].group(|ui| {
                ui.heading("Create Module");
                
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.new_module_name);
                });
                
                let can_create = self.selected_type.is_some() && !self.new_module_name.is_empty();
                
                if ui.add_enabled(can_create, egui::Button::new("➕ Create")).clicked() {
                    if let Some(type_id) = &self.selected_type {
                        self.pending_action = Some(PendingAction::CreateModule {
                            type_id: type_id.clone(),
                            name: self.new_module_name.clone(),
                        });
                    }
                }
            });
            
            // Right column: Active modules
            columns[1].heading("Active Modules");
            columns[1].separator();
            
            if self.modules.is_empty() {
                columns[1].label("No modules running");
            } else {
                // Clone modules to avoid borrow issues
                let modules: Vec<_> = self.modules.clone();
                egui::ScrollArea::vertical()
                    .id_salt("modules_list")
                    .show(&mut columns[1], |ui| {
                        for module in &modules {
                            self.render_module_card(ui, module);
                        }
                    });
            }
        });
        
        // Execute pending action
        if let Some(action) = self.pending_action.take() {
            self.execute_action(action, client, runtime);
        }
    }
    
    /// Render a module instance card
    fn render_module_card(&mut self, ui: &mut egui::Ui, module: &daq_proto::daq::ModuleStatus) {
        let state_color = match module.state {
            1 => egui::Color32::GRAY,    // CREATED
            2 => egui::Color32::BLUE,    // CONFIGURED
            3 => egui::Color32::GREEN,   // RUNNING
            4 => egui::Color32::YELLOW,  // PAUSED
            5 => egui::Color32::GRAY,    // STOPPED
            6 => egui::Color32::RED,     // ERROR
            7 => egui::Color32::LIGHT_BLUE, // STAGED
            _ => egui::Color32::WHITE,
        };
        
        let state_name = match module.state {
            1 => "Created",
            2 => "Configured",
            3 => "Running",
            4 => "Paused",
            5 => "Stopped",
            6 => "Error",
            7 => "Staged",
            _ => "Unknown",
        };
        
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.colored_label(state_color, "●");
                ui.strong(&module.instance_name);
                ui.label(format!("({})", module.type_id));
            });
            
            ui.label(format!("ID: {}", module.module_id));
            ui.label(format!("Status: {}", state_name));
            
            if module.required_roles_total > 0 {
                ui.label(format!(
                    "Roles: {}/{} filled",
                    module.required_roles_filled, module.required_roles_total
                ));
            }
            
            if module.state == 3 {
                // Running - show stats
                ui.label(format!("Events: {}", module.events_emitted));
                ui.label(format!("Data points: {}", module.data_points_produced));
            }
            
            if !module.error_message.is_empty() {
                ui.colored_label(egui::Color32::RED, &module.error_message);
            }
            
            // Control buttons
            ui.horizontal(|ui| {
                match module.state {
                    1 | 2 | 7 => {
                        // Created/Configured/Staged - can start
                        if module.ready_to_start {
                            if ui.button("▶ Start").clicked() {
                                self.pending_action = Some(PendingAction::StartModule {
                                    module_id: module.module_id.clone(),
                                });
                            }
                        } else {
                            ui.add_enabled(false, egui::Button::new("▶ Start (not ready)"));
                        }
                    }
                    3 => {
                        // Running - can stop
                        if ui.button("⏹ Stop").clicked() {
                            self.pending_action = Some(PendingAction::StopModule {
                                module_id: module.module_id.clone(),
                            });
                        }
                    }
                    _ => {}
                }
            });
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
            PendingAction::CreateModule { type_id, name } => {
                self.create_module(client, runtime, &type_id, &name);
            }
            PendingAction::StartModule { module_id } => {
                self.start_module(client, runtime, &module_id);
            }
            PendingAction::StopModule { module_id } => {
                self.stop_module(client, runtime, &module_id);
            }
        }
    }
    
    /// Refresh module data
    fn refresh(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime) {
        self.error = None;
        self.status = None;
        
        let Some(client) = client else {
            self.error = Some("Not connected to daemon".to_string());
            return;
        };
        
        let mut client = client.clone();
        match runtime.block_on(async {
            let types = client.list_module_types().await?;
            let modules = client.list_modules().await?;
            Ok::<_, anyhow::Error>((types, modules))
        }) {
            Ok((types, modules)) => {
                self.module_types = types;
                self.modules = modules;
                self.last_refresh = Some(std::time::Instant::now());
                self.status = Some(format!(
                    "Loaded {} types, {} modules",
                    self.module_types.len(),
                    self.modules.len()
                ));
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }
    
    /// Create a new module
    fn create_module(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime, type_id: &str, name: &str) {
        self.error = None;
        self.status = None;
        
        let Some(client) = client else {
            self.error = Some("Not connected to daemon".to_string());
            return;
        };
        
        let mut client = client.clone();
        let type_id = type_id.to_string();
        let name = name.to_string();
        
        match runtime.block_on(client.create_module(&type_id, &name)) {
            Ok(response) => {
                if response.success {
                    self.status = Some(format!("Created module: {}", response.module_id));
                    self.new_module_name.clear();
                    // Note: Would need to refresh to show new module
                } else {
                    self.error = Some(response.error_message);
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }
    
    /// Start a module
    fn start_module(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime, module_id: &str) {
        self.error = None;
        self.status = None;
        
        let Some(client) = client else {
            self.error = Some("Not connected to daemon".to_string());
            return;
        };
        
        let mut client = client.clone();
        let module_id = module_id.to_string();
        
        match runtime.block_on(client.start_module(&module_id)) {
            Ok(response) => {
                if response.success {
                    self.status = Some(format!("Started module: {}", module_id));
                } else {
                    self.error = Some(response.error_message);
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }
    
    /// Stop a module
    fn stop_module(&mut self, client: Option<&mut DaqClient>, runtime: &Runtime, module_id: &str) {
        self.error = None;
        self.status = None;
        
        let Some(client) = client else {
            self.error = Some("Not connected to daemon".to_string());
            return;
        };
        
        let mut client = client.clone();
        let module_id = module_id.to_string();
        
        match runtime.block_on(client.stop_module(&module_id)) {
            Ok(response) => {
                if response.success {
                    self.status = Some(format!("Stopped module: {}", module_id));
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
