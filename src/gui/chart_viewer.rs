//! Chart Viewer Widget
//! Right side scrollable panel for displaying interactive charts using egui_plot.
//! Optimized with virtual scrolling and lazy loading for performance.

use crate::charts::{ChartData, ChartPlotter};
use egui::{Color32, RichText, ScrollArea};
use std::collections::HashMap;

/// Chart card configuration
const CHART_SPACING: f32 = 15.0;
const CARD_HEIGHT: f32 = 450.0; // Larger height for better visibility

/// Scrollable chart display area with virtual scrolling.
/// Only renders charts that are visible in the viewport for optimal performance.
pub struct ChartViewer {
    /// Chart data for all data types
    pub chart_data: HashMap<String, ChartData>,
    /// Order of data types (mismatch first, then match)
    pub data_type_order: Vec<String>,
}

impl Default for ChartViewer {
    fn default() -> Self {
        Self {
            chart_data: HashMap::new(),
            data_type_order: Vec::new(),
        }
    }
}

impl ChartViewer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all charts
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.chart_data.clear();
        self.data_type_order.clear();
    }

    /// Set chart data with proper ordering (mismatch first)
    pub fn set_chart_data(&mut self, chart_data: HashMap<String, ChartData>) {
        let mut mismatch: Vec<String> = Vec::new();
        let mut match_items: Vec<String> = Vec::new();

        for (data_type, data) in &chart_data {
            if data.stats.has_significant_results() {
                mismatch.push(data_type.clone());
            } else {
                match_items.push(data_type.clone());
            }
        }

        mismatch.sort();
        match_items.sort();

        self.data_type_order = mismatch;
        self.data_type_order.extend(match_items);
        self.chart_data = chart_data;
    }

    /// Draw the chart viewer with virtual scrolling
    /// Only renders charts visible in the current viewport
    /// Each row displays ONE chart that fills the available width
    pub fn show(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.chart_data.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("No Data").size(20.0));
            });
            return;
        }

        // Get available width for full-width cards
        let avail_width = ui.available_width();

        // One chart per row
        let total_rows = self.data_type_order.len();
        let row_height = CARD_HEIGHT + CHART_SPACING;

        // Clone data for use in closure
        let order = self.data_type_order.clone();
        let chart_data = self.chart_data.clone();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                // Only render visible rows (one chart per row)
                for row in row_range {
                    if let Some(dt) = order.get(row) {
                        if let Some(data) = chart_data.get(dt) {
                            let is_sig = data.stats.has_significant_results();
                            Self::draw_chart_card_full_width(ui, data, is_sig, avail_width);
                        }
                    }
                    ui.add_space(CHART_SPACING);
                }
            });
    }

    /// Draw a single chart card at full width
    fn draw_chart_card_full_width(
        ui: &mut egui::Ui,
        chart_data: &ChartData,
        is_sig: bool,
        available_width: f32,
    ) {
        let border_color = if is_sig {
            Color32::from_rgb(220, 53, 69) // Red for significant
        } else {
            Color32::from_rgb(40, 167, 69) // Green for match
        };

        // Card width with padding
        let card_width = available_width - 20.0;
        let chart_width = (card_width - 40.0) / 2.0; // Two charts side by side

        egui::Frame::none()
            .rounding(8.0)
            .stroke(egui::Stroke::new(2.0, border_color))
            .fill(Color32::from_rgb(30, 30, 35))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.set_width(card_width);

                ui.vertical(|ui| {
                    // Title with icon - larger font
                    let icon = if is_sig { "⚠" } else { "✓" };
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{} Analysis: {}", icon, chart_data.data_type))
                                .size(18.0)
                                .strong()
                                .color(border_color),
                        );
                    });

                    ui.add_space(8.0);

                    // Legend - larger
                    ui.horizontal(|ui| {
                        let mut non_ctrl_idx = 0;
                        for group in chart_data.stats.get_ordered_groups() {
                            let color = ChartPlotter::get_group_color(
                                &group,
                                &chart_data.stats.control_group,
                                non_ctrl_idx,
                            );
                            if group != chart_data.stats.control_group {
                                non_ctrl_idx += 1;
                            }

                            // Color square - larger
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 3.0, color);
                            ui.label(RichText::new(&group).size(13.0));
                            ui.add_space(12.0);
                        }
                    });

                    ui.add_space(10.0);

                    // Two charts side by side - larger
                    ui.horizontal(|ui| {
                        // Boxplot
                        ui.vertical(|ui| {
                            ui.set_width(chart_width);
                            ui.label(RichText::new("Distribution by Group").size(14.0).strong());
                            ChartPlotter::draw_boxplot_chart(ui, chart_data, true);
                            // true = full size
                        });

                        ui.add_space(10.0);

                        // QQ Plot
                        ui.vertical(|ui| {
                            ui.set_width(chart_width);
                            ui.label(RichText::new("Normal Quantile Plot").size(14.0).strong());
                            ChartPlotter::draw_qq_chart(ui, chart_data, true); // true = full size
                        });
                    });

                    ui.add_space(10.0);

                    // Statistics table
                    ChartPlotter::draw_stats_table(ui, &chart_data.stats);
                });
            });
    }
}
