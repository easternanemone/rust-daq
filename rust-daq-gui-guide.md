# GUI Development Guide: egui-based Scientific Interface

## Overview

This guide covers developing the graphical user interface for the scientific data acquisition application using egui, a modern immediate-mode GUI framework that excels in scientific applications due to its performance and ease of use.

## Core GUI Architecture

### Main Window Structure
```rust
use eframe::egui;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::core::Application;

pub struct MainWindow {
    app: Arc<RwLock<Application>>,
    ui_state: UiState,
    panels: GuiPanels,
}

#[derive(Default)]
pub struct UiState {
    pub selected_instrument: Option<String>,
    pub show_settings: bool,
    pub plot_paused: bool,
    pub auto_scale: bool,
    pub data_recording: bool,
}

pub struct GuiPanels {
    pub instrument_panel: InstrumentPanel,
    pub plot_panel: PlotPanel,
    pub control_panel: ControlPanel,
    pub status_panel: StatusPanel,
    pub log_panel: LogPanel,
}

impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.setup_style(ctx);
        self.create_menu_bar(ctx, frame);
        self.create_main_panels(ctx);
        
        // Request repaint for real-time updates
        ctx.request_repaint();
    }
}
```

### Panel-Based Layout
```rust
impl MainWindow {
    fn create_main_panels(&mut self, ctx: &egui::Context) {
        // Left sidebar for instruments and controls
        egui::SidePanel::left("instruments")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                self.panels.instrument_panel.show(ui, &mut self.ui_state);
                ui.separator();
                self.panels.control_panel.show(ui, &mut self.ui_state);
            });

        // Bottom panel for logs and status
        egui::TopBottomPanel::bottom("status")
            .resizable(true)
            .default_height(150.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.panels.log_panel.show(ui);
                });
            });

        // Central panel for plots and data visualization
        egui::CentralPanel::default().show(ctx, |ui| {
            self.panels.plot_panel.show(ui, &mut self.ui_state);
        });
    }
}
```

## Real-Time Data Visualization

### Plot Panel Implementation
```rust
use egui_plot::{Line, Plot, PlotPoints, PlotResponse, PlotUi};
use std::collections::VecDeque;

pub struct PlotPanel {
    data_buffer: VecDeque<DataPoint>,
    max_points: usize,
    plot_bounds: PlotBounds,
    cursors: Vec<Cursor>,
}

#[derive(Clone)]
pub struct DataPoint {
    pub timestamp: f64,
    pub value: f64,
    pub channel: usize,
}

#[derive(Clone)]
pub struct PlotBounds {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub auto_scale: bool,
}

impl PlotPanel {
    pub fn new() -> Self {
        Self {
            data_buffer: VecDeque::with_capacity(10000),
            max_points: 10000,
            plot_bounds: PlotBounds::default(),
            cursors: Vec::new(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, ui_state: &mut UiState) {
        ui.horizontal(|ui| {
            ui.label("Data Visualization");
            ui.separator();
            
            if ui.button("⏸ Pause").clicked() {
                ui_state.plot_paused = !ui_state.plot_paused;
            }
            
            ui.checkbox(&mut ui_state.auto_scale, "Auto Scale");
            
            if ui.button("📋 Export").clicked() {
                self.export_data();
            }
        });

        let plot = Plot::new("main_plot")
            .view_aspect(2.0)
            .auto_bounds_x()
            .auto_bounds_y()
            .allow_zoom(true)
            .allow_drag(true)
            .allow_scroll(true)
            .show_axes([true, true])
            .show_grid(true);

        let response = plot.show(ui, |plot_ui| {
            self.draw_data_lines(plot_ui);
            self.draw_cursors(plot_ui);
            self.handle_plot_interactions(plot_ui);
        });

        self.handle_plot_response(response, ui_state);
    }

    fn draw_data_lines(&self, plot_ui: &mut PlotUi) {
        // Group data by channel
        let mut channels: std::collections::HashMap<usize, Vec<[f64; 2]>> = 
            std::collections::HashMap::new();

        for point in &self.data_buffer {
            channels.entry(point.channel)
                .or_insert_with(Vec::new)
                .push([point.timestamp, point.value]);
        }

        // Draw lines for each channel
        for (channel, points) in channels {
            let color = self.get_channel_color(channel);
            let line = Line::new(PlotPoints::new(points))
                .color(color)
                .width(2.0)
                .name(format!("Channel {}", channel));
            
            plot_ui.line(line);
        }
    }

    fn get_channel_color(&self, channel: usize) -> egui::Color32 {
        match channel % 8 {
            0 => egui::Color32::from_rgb(255, 0, 0),     // Red
            1 => egui::Color32::from_rgb(0, 255, 0),     // Green
            2 => egui::Color32::from_rgb(0, 0, 255),     // Blue
            3 => egui::Color32::from_rgb(255, 255, 0),   // Yellow
            4 => egui::Color32::from_rgb(255, 0, 255),   // Magenta
            5 => egui::Color32::from_rgb(0, 255, 255),   // Cyan
            6 => egui::Color32::from_rgb(255, 165, 0),   // Orange
            7 => egui::Color32::from_rgb(128, 0, 128),   // Purple
            _ => egui::Color32::GRAY,
        }
    }

    pub fn add_data_point(&mut self, point: DataPoint) {
        self.data_buffer.push_back(point);
        
        // Maintain buffer size
        while self.data_buffer.len() > self.max_points {
            self.data_buffer.pop_front();
        }
    }
}
```

### Advanced Plot Features
```rust
impl PlotPanel {
    fn draw_cursors(&mut self, plot_ui: &mut PlotUi) {
        for cursor in &mut self.cursors {
            let line = egui_plot::VLine::new(cursor.x_position)
                .color(egui::Color32::WHITE)
                .width(1.0)
                .style(egui_plot::LineStyle::Dashed { length: 10.0 });
            
            plot_ui.vline(line);
            
            // Show cursor value
            let text = format!("t={:.3}s", cursor.x_position);
            plot_ui.text(
                egui_plot::Text::new([cursor.x_position, cursor.y_position], text)
                    .color(egui::Color32::WHITE)
            );
        }
    }

    fn handle_plot_interactions(&mut self, plot_ui: &mut PlotUi) {
        // Handle cursor placement
        if plot_ui.response().clicked() {
            if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                self.cursors.push(Cursor {
                    x_position: pointer_pos.x,
                    y_position: pointer_pos.y,
                });
            }
        }

        // Handle double-click to clear cursors
        if plot_ui.response().double_clicked() {
            self.cursors.clear();
        }
    }
}

#[derive(Clone)]
pub struct Cursor {
    pub x_position: f64,
    pub y_position: f64,
}
```

## Instrument Control Panel

### Interactive Controls
```rust
use crate::core::{Instrument, InstrumentConfig};

pub struct InstrumentPanel {
    instruments: Vec<InstrumentInfo>,
    selected_instrument: Option<usize>,
}

#[derive(Clone)]
pub struct InstrumentInfo {
    pub name: String,
    pub status: InstrumentStatus,
    pub config: InstrumentConfig,
    pub controls: Vec<ParameterControl>,
}

#[derive(Clone, PartialEq)]
pub enum InstrumentStatus {
    Disconnected,
    Connecting,
    Connected,
    Acquiring,
    Error(String),
}

#[derive(Clone)]
pub struct ParameterControl {
    pub name: String,
    pub value: ParameterValue,
    pub range: Option<(f64, f64)>,
    pub units: Option<String>,
}

#[derive(Clone)]
pub enum ParameterValue {
    Float(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
    Choice { options: Vec<String>, selected: usize },
}

impl InstrumentPanel {
    pub fn show(&mut self, ui: &mut egui::Ui, ui_state: &mut UiState) {
        ui.heading("Instruments");
        
        // Instrument list
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for (idx, instrument) in self.instruments.iter().enumerate() {
                    let selected = ui_state.selected_instrument
                        .as_ref()
                        .map(|s| s == &instrument.name)
                        .unwrap_or(false);
                    
                    if ui.selectable_label(selected, &instrument.name).clicked() {
                        ui_state.selected_instrument = Some(instrument.name.clone());
                        self.selected_instrument = Some(idx);
                    }
                    
                    ui.same_line();
                    self.show_status_indicator(ui, &instrument.status);
                }
            });

        ui.separator();

        // Selected instrument controls
        if let Some(idx) = self.selected_instrument {
            if let Some(instrument) = self.instruments.get_mut(idx) {
                self.show_instrument_controls(ui, instrument);
            }
        }
    }

    fn show_status_indicator(&self, ui: &mut egui::Ui, status: &InstrumentStatus) {
        let (color, text) = match status {
            InstrumentStatus::Disconnected => (egui::Color32::RED, "●"),
            InstrumentStatus::Connecting => (egui::Color32::YELLOW, "●"),
            InstrumentStatus::Connected => (egui::Color32::GREEN, "●"),
            InstrumentStatus::Acquiring => (egui::Color32::BLUE, "●"),
            InstrumentStatus::Error(_) => (egui::Color32::DARK_RED, "●"),
        };
        
        ui.colored_label(color, text);
    }

    fn show_instrument_controls(&mut self, ui: &mut egui::Ui, instrument: &mut InstrumentInfo) {
        ui.heading(&instrument.name);
        
        // Connection controls
        ui.horizontal(|ui| {
            match instrument.status {
                InstrumentStatus::Disconnected => {
                    if ui.button("Connect").clicked() {
                        // Send connect command
                    }
                }
                InstrumentStatus::Connected => {
                    if ui.button("Disconnect").clicked() {
                        // Send disconnect command
                    }
                    
                    if ui.button("Start Acquisition").clicked() {
                        // Start data acquisition
                    }
                }
                InstrumentStatus::Acquiring => {
                    if ui.button("Stop Acquisition").clicked() {
                        // Stop data acquisition
                    }
                }
                _ => {}
            }
        });

        ui.separator();

        // Parameter controls
        egui::ScrollArea::vertical().show(ui, |ui| {
            for control in &mut instrument.controls {
                self.show_parameter_control(ui, control);
            }
        });
    }

    fn show_parameter_control(&mut self, ui: &mut egui::Ui, control: &mut ParameterControl) {
        ui.horizontal(|ui| {
            ui.label(&control.name);
            
            match &mut control.value {
                ParameterValue::Float(ref mut value) => {
                    let mut drag = egui::DragValue::new(value);
                    
                    if let Some((min, max)) = control.range {
                        drag = drag.clamp_range(min..=max);
                    }
                    
                    ui.add(drag);
                    
                    if let Some(units) = &control.units {
                        ui.label(units);
                    }
                }
                ParameterValue::Integer(ref mut value) => {
                    ui.add(egui::DragValue::new(value));
                }
                ParameterValue::Boolean(ref mut value) => {
                    ui.checkbox(value, "");
                }
                ParameterValue::String(ref mut value) => {
                    ui.text_edit_singleline(value);
                }
                ParameterValue::Choice { options, ref mut selected } => {
                    egui::ComboBox::from_label("")
                        .selected_text(&options[*selected])
                        .show_ui(ui, |ui| {
                            for (idx, option) in options.iter().enumerate() {
                                ui.selectable_value(selected, idx, option);
                            }
                        });
                }
            }
        });
    }
}
```

## Styling and Themes

### Custom Styling
```rust
impl MainWindow {
    fn setup_style(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        
        // Scientific dark theme
        style.visuals = egui::Visuals {
            dark_mode: true,
            override_text_color: Some(egui::Color32::from_gray(240)),
            widgets: egui::style::Widgets {
                noninteractive: egui::style::WidgetVisuals {
                    bg_fill: egui::Color32::from_gray(27),
                    bg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
                    fg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(240)),
                    rounding: egui::Rounding::same(4.0),
                    expansion: 0.0,
                },
                inactive: egui::style::WidgetVisuals {
                    bg_fill: egui::Color32::from_gray(35),
                    bg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
                    fg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(180)),
                    rounding: egui::Rounding::same(4.0),
                    expansion: 0.0,
                },
                hovered: egui::style::WidgetVisuals {
                    bg_fill: egui::Color32::from_gray(45),
                    bg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
                    fg_stroke: egui::Stroke::new(1.5, egui::Color32::from_gray(240)),
                    rounding: egui::Rounding::same(4.0),
                    expansion: 1.0,
                },
                active: egui::style::WidgetVisuals {
                    bg_fill: egui::Color32::from_gray(55),
                    bg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
                    fg_stroke: egui::Stroke::new(2.0, egui::Color32::WHITE),
                    rounding: egui::Rounding::same(4.0),
                    expansion: 1.0,
                },
                open: egui::style::WidgetVisuals {
                    bg_fill: egui::Color32::from_gray(27),
                    bg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
                    fg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(210)),
                    rounding: egui::Rounding::same(4.0),
                    expansion: 0.0,
                },
            },
            selection: egui::style::Selection {
                bg_fill: egui::Color32::from_rgb(0, 92, 128),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(192, 222, 255)),
            },
            ..egui::Visuals::dark()
        };
        
        ctx.set_style(style);
    }
}
```

## Performance Optimization

### Efficient Rendering
```rust
impl PlotPanel {
    pub fn show_optimized(&mut self, ui: &mut egui::Ui, ui_state: &UiState) {
        // Only update if not paused and data has changed
        if !ui_state.plot_paused && self.data_changed {
            self.update_plot_cache();
            self.data_changed = false;
        }
        
        // Use cached plot data for rendering
        let plot = Plot::new("main_plot")
            .auto_bounds_x()
            .auto_bounds_y();
            
        plot.show(ui, |plot_ui| {
            // Render cached lines
            for line in &self.cached_lines {
                plot_ui.line(line.clone());
            }
        });
    }
    
    fn update_plot_cache(&mut self) {
        self.cached_lines.clear();
        
        // Downsample data for better performance
        let downsampled_data = self.downsample_data(1000);
        
        // Create cached line objects
        for (channel, points) in downsampled_data {
            let line = Line::new(PlotPoints::new(points))
                .color(self.get_channel_color(channel))
                .width(2.0);
            self.cached_lines.push(line);
        }
    }
}
```

This GUI development guide provides the foundation for creating a professional, responsive, and feature-rich interface for scientific data acquisition applications using egui's immediate-mode approach.