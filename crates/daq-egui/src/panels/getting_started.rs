//! Getting Started panel with demo mode instructions.

use eframe::egui;

/// Getting Started panel state
#[derive(Default)]
pub struct GettingStartedPanel {}

impl GettingStartedPanel {
    /// Render the Getting Started panel
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("🚀 Getting Started with rust-daq");
        ui.separator();
        ui.add_space(8.0);

        // Demo Mode Section
        ui.group(|ui| {
            ui.heading("🎯 Demo Mode (No Hardware Required)");
            ui.add_space(4.0);
            
            ui.label("Try rust-daq without physical hardware using mock devices:");
            ui.add_space(8.0);
            
            ui.horizontal(|ui| {
                ui.label("1️⃣");
                ui.vertical(|ui| {
                    ui.strong("Start the daemon in demo mode:");
                    ui.add_space(2.0);
                    ui.code("cargo run --bin rust-daq-daemon -- daemon --hardware-config config/demo.toml");
                    ui.add_space(4.0);
                    ui.label("Or if using pre-built binaries:");
                    ui.label("Unix/macOS:");
                    ui.code("./rust-daq-daemon daemon --hardware-config config/demo.toml");
                    ui.label("Windows:");
                    ui.code(".\\rust-daq-daemon.exe daemon --hardware-config config/demo.toml");
                });
            });
            
            ui.add_space(8.0);
            
            ui.horizontal(|ui| {
                ui.label("2️⃣");
                ui.vertical(|ui| {
                    ui.strong("Connect this GUI:");
                    ui.add_space(2.0);
                    ui.label("Use the connection bar at the bottom to connect to:");
                    ui.code("http://127.0.0.1:50051");
                });
            });
            
            ui.add_space(8.0);
            
            ui.horizontal(|ui| {
                ui.label("3️⃣");
                ui.vertical(|ui| {
                    ui.strong("Explore mock devices (daemon mode):");
                    ui.add_space(2.0);
                    ui.label("• mock_stage - Simulated linear stage");
                    ui.label("• mock_power_meter - Simulated optical power sensor");
                    ui.label("• mock_camera - Simulated camera (640x480 in config)");
                    ui.label("Note: One-shot mode uses 1920x1080 camera");
                });
            });
        });
        
        ui.add_space(12.0);
        
        // Example Scripts Section
        ui.group(|ui| {
            ui.heading("📜 Try the Working Demo (v0.5.0)");
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("✅");
                ui.vertical(|ui| {
                    ui.strong("Camera Demo (works now!):");
                    ui.code("cargo run --bin rust-daq-daemon -- run examples/demo_camera.rhai");
                    ui.add_space(2.0);
                    ui.label("• No daemon needed - runs in one-shot mode");
                    ui.label("• Demonstrates camera arm() and trigger()");
                    ui.label("• Uses mock camera (1920x1080)");
                });
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("⏳");
                ui.vertical(|ui| {
                    ui.strong("Multi-Device Scan (coming in v0.6.0):");
                    ui.label("examples/demo_scan.rhai will demonstrate:");
                    ui.label("• Stage + power meter synchronized workflow");
                    ui.label("• Config-based device loading in daemon mode");
                    ui.label("• gRPC script execution");
                });
            });
        });
        
        ui.add_space(12.0);
        
        // Visualization Section
        ui.group(|ui| {
            ui.heading("📊 Live Data Visualization");
            ui.add_space(4.0);
            
            ui.label("View camera frames in real-time using Rerun:");
            ui.add_space(8.0);
            
            ui.horizontal(|ui| {
                ui.label("1️⃣");
                ui.vertical(|ui| {
                    ui.strong("Launch Rerun viewer:");
                    ui.code("rerun");
                    ui.label("(Install via: pip install rerun-sdk)");
                });
            });
            
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                ui.label("2️⃣");
                ui.vertical(|ui| {
                    ui.strong("Start camera stream:");
                    ui.label("Go to Devices panel → Select mock_camera → Click 'Start Stream'");
                });
            });
        });
        
        ui.add_space(12.0);
        
        // Next Steps Section
        ui.group(|ui| {
            ui.heading("⚙️ Next Steps");
            ui.add_space(4.0);
            
            ui.label("Ready to use real hardware?");
            ui.add_space(8.0);
            
            ui.horizontal(|ui| {
                ui.label("📝");
                ui.label("Edit crates/rust-daq/config/hardware.example.toml with your device settings");
            });
            
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                ui.label("🔌");
                ui.label("Connect physical devices and update serial ports");
            });
            
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                ui.label("🚀");
                ui.label("Start daemon with: --hardware-config config/hardware.toml");
            });
            
            ui.add_space(8.0);
            
            if ui.button("📖 View Documentation").clicked() {
                ui.ctx().copy_text("See project documentation at https://github.com/easternanemone/rust-daq".to_string());
            }
        });
        
        ui.add_space(12.0);
        
        // Advanced Options
        ui.collapsing("Advanced Demo Options", |ui| {
            ui.add_space(4.0);

            ui.label("Enable debug logging:");
            ui.label("Unix/macOS:");
            ui.code("RUST_LOG=debug cargo run --bin rust-daq-daemon -- daemon --hardware-config config/demo.toml");
            ui.add_space(4.0);
            ui.label("Windows PowerShell:");
            ui.code("$env:RUST_LOG=\"debug\"; cargo run --bin rust-daq-daemon -- daemon --hardware-config config/demo.toml");

            ui.add_space(8.0);

            ui.label("Customize daemon port:");
            ui.code("cargo run --bin rust-daq-daemon -- daemon --hardware-config config/demo.toml --port 50052");

            ui.add_space(8.0);

            ui.label("View example scripts:");
            ui.label("• examples/demo_camera.rhai - One-shot camera demo");
            ui.label("• examples/demo_scan.rhai - Daemon-based scan demo");
            ui.label("• crates/daq-examples/examples/ - More advanced examples");
        });
    }
}
