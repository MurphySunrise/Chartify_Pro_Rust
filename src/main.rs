//! Chartify Pro - CSV Data Analysis & Interactive Chart Viewer
//!
//! A Rust application for analyzing CSV data and displaying interactive charts.

// Hide console window on Windows in release builds
#![windows_subsystem = "windows"]

mod charts;
mod data;
mod gui;
mod ppt;
mod stats;

use eframe::egui;
use gui::ChartifyApp;

fn main() -> eframe::Result<()> {
    // Configure native options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([2000.0, 900.0]) // Width for 2 chart columns (800×2 + left panel + spacing)
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
