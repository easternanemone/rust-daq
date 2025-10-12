//! The eframe/egui implementation for the GUI.
use crate::app::{DaqApp, DaqAppInner};
use crate::core::DataPoint;
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use log::error;
use std::collections::VecDeque;
use tokio::sync::broadcast;

const PLOT_DATA_CAPACITY: usize = 1000;

/// The main GUI struct.
pub struct Gui {
    app: DaqApp,
    data_receiver: broadcast::Receiver<DataPoint>,
    plot_data: VecDeque<[f64; 2]>,
    last_timestamp: f64,
}

impl Gui {
    /// Creates a new GUI.
    pub fn new(_cc: &eframe::CreationContext<'_>, app: DaqApp) -> Self {
        let data_receiver = app.with_inner(|inner| inner.data_sender.subscribe());
        Self {
            app,
            data_receiver,
            plot_data: VecDeque::with_capacity(PLOT_DATA_CAPACITY),
            last_timestamp: 0.0,
        }
    }

    /// Fetches new data points from the broadcast channel.
    fn update_data(&mut self) {
        while let Ok(data_point) = self.data_receiver.try_recv() {
            if data_point.channel == "sine_wave" {
                if self.plot_data.len() >= PLOT_DATA_CAPACITY {
                    self.plot_data.pop_front();
                }
                let timestamp = data_point.timestamp.timestamp_micros() as f64 / 1_000_000.0;
                if self.last_timestamp == 0.0 {
                    self.last_timestamp = timestamp;
                }
                self.plot_data
                    .push_back([timestamp - self.last_timestamp, data_point.value]);
            }
        }
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_data();

        self.app.with_inner(|inner| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Rust DAQ Control Panel");
                ui.separator();
                instrument_control_panel(ui, inner);
                ui.separator();
                live_plot(ui, &self.plot_data);
            });
        });

        // Request a repaint to ensure the GUI is continuously updated
        ctx.request_repaint();
    }
}

fn instrument_control_panel(ui: &mut egui::Ui, inner: &mut DaqAppInner) {
    ui.heading("Instruments");
    let available_instruments: Vec<String> = inner.instrument_registry.list().collect();

    for id in available_instruments {
        ui.horizontal(|ui| {
            ui.label(id.clone());
            let is_running = inner.instruments.contains_key(&id);
            if is_running {
                if ui.button("Stop").clicked() {
                    inner.stop_instrument(&id);
                }
            } else if ui.button("Start").clicked() {
                if let Err(e) = inner.spawn_instrument(&id) {
                    error!("Failed to start instrument '{}': {}", id, e);
                }
            }
        });
    }
}

fn live_plot(ui: &mut egui::Ui, data: &VecDeque<[f64; 2]>) {
    ui.heading("Live Data (Mock Instrument Sine Wave)");
    let line = Line::new(PlotPoints::from_iter(data.iter().copied()));
    Plot::new("live_plot").view_aspect(2.0).show(ui, |plot_ui| {
        plot_ui.line(line);
    });
}
