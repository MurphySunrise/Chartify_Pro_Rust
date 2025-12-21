//! Chartify Pro - CSV Data Analysis & Interactive Chart Viewer
//!
//! A Rust application for analyzing CSV data and displaying interactive charts.

mod data;
mod stats;
mod charts;
mod gui;

use eframe::egui;
use gui::ChartifyApp;

fn main() -> eframe::Result<()> {
    // Configure native options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 800.0])
            .with_min_inner_size([1200.0, 700.0])
            .with_title("Chartify Pro"),
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "Chartify Pro",
        options,
        Box::new(|cc| Ok(Box::new(ChartifyApp::new(cc)))),
    )
}
