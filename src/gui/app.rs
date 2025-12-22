//! Chartify Pro Main Application
//! Main window with control panel and chart viewer.

use crate::charts::ChartData;
use crate::data::{DataLoader, DataMode, DataProcessor};
use crate::gui::{ChartViewer, ControlPanel, ControlPanelAction};
use crate::stats::StatsCalculator;
use egui::SidePanel;
use polars::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// Calculation result from background thread
enum CalcResult {
    Progress(f32, String),
    Complete(HashMap<String, ChartData>),
    Error(String),
}

/// CSV loading result from background thread
enum LoadResult {
    Progress(String),
    Complete {
        df: DataFrame,
        columns: Vec<String>,
        row_count: usize,
    },
    Error(String),
}

/// Main application window.
pub struct ChartifyApp {
    loader: DataLoader,
    control_panel: ControlPanel,
    chart_viewer: ChartViewer,

    // Async calculation
    calc_rx: Option<Receiver<CalcResult>>,
    is_calculating: bool,

    // Async CSV loading
    load_rx: Option<Receiver<LoadResult>>,
    is_loading: bool,
}

impl ChartifyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            loader: DataLoader::new(),
            control_panel: ControlPanel::new(),
            chart_viewer: ChartViewer::new(),
            calc_rx: None,
            is_calculating: false,
            load_rx: None,
            is_loading: false,
        }
    }

    /// Handle CSV file selection - now async!
    fn handle_browse_csv(&mut self) {
        if self.is_loading {
            return; // Already loading
        }

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV Files", &["csv"])
            .pick_file()
        {
            // Clear previous charts
            self.chart_viewer.clear();
            self.control_panel.settings.csv_path = Some(path.clone());
            self.control_panel.set_progress(0.0, "Loading CSV file...");
            self.is_loading = true;

            let (tx, rx) = channel();
            self.load_rx = Some(rx);

            let path_str = path.to_string_lossy().to_string();

            // Load CSV in background thread
            thread::spawn(move || {
                let _ = tx.send(LoadResult::Progress("Reading CSV file...".to_string()));

                let result = LazyCsvReader::new(&path_str)
                    .with_infer_schema_length(Some(10000))
                    .with_ignore_errors(true)
                    .finish()
                    .and_then(|lazy| lazy.collect());

                match result {
                    Ok(df) => {
                        let columns: Vec<String> = df
                            .get_column_names()
                            .iter()
                            .map(|s| s.to_string())
                            .collect();
                        let row_count = df.height();
                        let _ = tx.send(LoadResult::Complete {
                            df,
                            columns,
                            row_count,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(LoadResult::Error(e.to_string()));
                    }
                }
            });
        }
    }

    /// Check for CSV loading results
    fn check_load_results(&mut self) {
        let rx = self.load_rx.take();
        if let Some(rx) = rx {
            let mut should_keep_receiver = true;

            while let Ok(result) = rx.try_recv() {
                match result {
                    LoadResult::Progress(status) => {
                        self.control_panel.set_progress(0.0, &status);
                    }
                    LoadResult::Complete {
                        df,
                        columns,
                        row_count,
                    } => {
                        self.loader.set_dataframe(df);
                        self.control_panel.update_columns(columns.clone());
                        self.control_panel.set_progress(
                            0.0,
                            &format!("Loaded {} rows, {} columns", row_count, columns.len()),
                        );
                        self.is_loading = false;
                        should_keep_receiver = false;
                    }
                    LoadResult::Error(error) => {
                        self.control_panel
                            .set_progress(0.0, &format!("Error: {}", error));
                        self.is_loading = false;
                        should_keep_receiver = false;
                    }
                }
            }

            if should_keep_receiver {
                self.load_rx = Some(rx);
            }
        }
    }

    /// Handle group column change - update available groups
    fn handle_group_column_changed(&mut self) {
        let group_col = &self.control_panel.settings.group_col;
        if !group_col.is_empty() {
            let groups = self.loader.get_unique_values(group_col);
            self.control_panel.update_groups(groups);
        }
    }

    /// Start calculation in background thread
    fn start_calculation(&mut self) {
        let settings = self.control_panel.settings.clone();
        let data_cols = self.control_panel.get_selected_data_cols();

        // Get DataFrame clone
        let Some(df) = self.loader.get_dataframe().cloned() else {
            self.control_panel.set_progress(0.0, "No data loaded");
            return;
        };

        let (tx, rx) = channel();
        self.calc_rx = Some(rx);
        self.is_calculating = true;
        self.control_panel.set_progress(5.0, "Processing data...");

        // Run calculation in background thread
        thread::spawn(move || {
            Self::run_calculation(tx, df, settings, data_cols);
        });
    }

    /// Run calculation (called from background thread)
    fn run_calculation(
        tx: Sender<CalcResult>,
        df: DataFrame,
        settings: crate::gui::control_panel::UserSettings,
        data_cols: Vec<String>,
    ) {
        let _ = tx.send(CalcResult::Progress(10.0, "Processing data...".to_string()));

        // Process data
        let processed_df = match settings.mode {
            DataMode::Single => DataProcessor::prepare_data(
                &df,
                DataMode::Single,
                &settings.group_col,
                Some(&settings.data_type_col),
                Some(&settings.value_col),
                None,
            ),
            DataMode::Multi => DataProcessor::prepare_data(
                &df,
                DataMode::Multi,
                &settings.group_col,
                None,
                None,
                Some(&data_cols),
            ),
        };

        let processed_df = match processed_df {
            Ok(df) => df,
            Err(e) => {
                let _ = tx.send(CalcResult::Error(e.to_string()));
                return;
            }
        };

        let _ = tx.send(CalcResult::Progress(
            30.0,
            "Calculating statistics...".to_string(),
        ));

        // Calculate statistics in parallel
        let stats =
            StatsCalculator::compute_all_stats_parallel(&processed_df, &settings.control_group);

        let _ = tx.send(CalcResult::Progress(
            50.0,
            "Generating charts...".to_string(),
        ));

        // Generate chart data in parallel
        let data_types: Vec<String> = stats.keys().cloned().collect();

        let chart_data: HashMap<String, ChartData> = data_types
            .par_iter()
            .map(|data_type| {
                let stat = stats.get(data_type).unwrap();
                let mut data_by_group = HashMap::new();

                for group in stat.get_ordered_groups() {
                    // Use the new function that filters by BOTH data_type AND group
                    // This ensures quantile plot data matches the statistics table
                    let values = StatsCalculator::get_values_for_data_type_and_group(
                        &processed_df,
                        data_type,
                        &group,
                    )
                    .into_iter()
                    .filter(|v| !v.is_nan())
                    .collect();
                    data_by_group.insert(group, values);
                }

                (
                    data_type.clone(),
                    ChartData {
                        data_type: data_type.clone(),
                        data_by_group,
                        stats: stat.clone(),
                    },
                )
            })
            .collect();

        let _ = tx.send(CalcResult::Complete(chart_data));
    }

    /// Check for calculation results
    fn check_calculation_results(&mut self) {
        // Take the receiver temporarily to avoid borrow issues
        let rx = self.calc_rx.take();
        if let Some(rx) = rx {
            let mut should_keep_receiver = true;

            while let Ok(result) = rx.try_recv() {
                match result {
                    CalcResult::Progress(progress, status) => {
                        self.control_panel.set_progress(progress, &status);
                    }
                    CalcResult::Complete(chart_data) => {
                        let count = chart_data.len();
                        self.chart_viewer.set_chart_data(chart_data);
                        self.control_panel
                            .set_progress(100.0, &format!("Complete! {} charts ready", count));
                        self.is_calculating = false;
                        should_keep_receiver = false;
                    }
                    CalcResult::Error(error) => {
                        self.control_panel
                            .set_progress(0.0, &format!("Error: {}", error));
                        self.is_calculating = false;
                        should_keep_receiver = false;
                    }
                }
            }

            // Put receiver back if still needed
            if should_keep_receiver {
                self.calc_rx = Some(rx);
            }
        }
    }

    /// Handle PPT export - render charts to memory and create PPT directly
    fn handle_export_ppt(&mut self) {
        use crate::charts::ChartRenderer;
        use crate::ppt::PptGenerator;

        // Check if we have chart data
        if self.chart_viewer.chart_data.is_empty() {
            self.control_panel.set_progress(0.0, "No charts to export");
            return;
        }

        // Ask user for output location
        let output_path = match rfd::FileDialog::new()
            .add_filter("PowerPoint", &["pptx"])
            .set_file_name("chartify_report.pptx")
            .save_file()
        {
            Some(path) => path,
            None => return, // User cancelled
        };

        self.control_panel.set_progress(10.0, "Rendering charts...");

        // Render charts to in-memory PNG bytes
        let width = 1400u32;
        let height = 1000u32;
        let mut image_data: Vec<Vec<u8>> = Vec::new();
        let total = self.chart_viewer.data_type_order.len();

        for (idx, data_type) in self.chart_viewer.data_type_order.iter().enumerate() {
            if let Some(chart_data) = self.chart_viewer.chart_data.get(data_type) {
                match ChartRenderer::render_chart_card_to_bytes(chart_data, width, height) {
                    Ok(png_bytes) => {
                        image_data.push(png_bytes);
                        let progress = 10.0 + (idx as f32 / total as f32) * 40.0;
                        self.control_panel.set_progress(
                            progress,
                            &format!("Rendering chart {}/{}...", idx + 1, total),
                        );
                    }
                    Err(e) => {
                        self.control_panel
                            .set_progress(0.0, &format!("Render error: {}", e));
                        return;
                    }
                }
            }
        }

        if image_data.is_empty() {
            self.control_panel.set_progress(0.0, "No charts rendered");
            return;
        }

        self.control_panel.set_progress(60.0, "Generating PPT...");

        // Generate PPT with in-memory images
        match PptGenerator::generate_ppt_from_bytes(
            &image_data,
            &output_path,
            "Chartify Pro Report",
        ) {
            Ok(()) => {
                let slide_count = image_data.len().div_ceil(4);
                self.control_panel.set_progress(
                    100.0,
                    &format!(
                        "PPT exported: {} slides, {} charts",
                        slide_count,
                        image_data.len()
                    ),
                );
            }
            Err(e) => {
                self.control_panel
                    .set_progress(0.0, &format!("PPT error: {}", e));
            }
        }
    }
}

impl eframe::App for ChartifyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for background results
        self.check_load_results();
        self.check_calculation_results();

        // Request repaint while loading or calculating
        if self.is_loading || self.is_calculating {
            ctx.request_repaint();
        }

        // Left panel - Control Panel
        SidePanel::left("control_panel")
            .min_width(300.0)
            .max_width(350.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let action = self.control_panel.show(ui);

                    match action {
                        ControlPanelAction::BrowseCsv => self.handle_browse_csv(),
                        ControlPanelAction::GroupColumnChanged => {
                            self.handle_group_column_changed()
                        }
                        ControlPanelAction::Calculate => {
                            if !self.is_calculating {
                                self.start_calculation();
                            }
                        }
                        ControlPanelAction::ExportPpt => {
                            self.handle_export_ppt();
                        }
                        ControlPanelAction::None => {}
                    }
                });
            });

        // Central panel - Chart Viewer
        egui::CentralPanel::default().show(ctx, |ui| {
            self.chart_viewer.show(ctx, ui);
        });
    }
}
