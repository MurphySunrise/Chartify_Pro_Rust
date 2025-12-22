//! Chart Viewer Widget
//! Right side scrollable panel for displaying interactive charts using egui_plot.
//! Supports responsive multi-column layout based on available width.

use crate::charts::{ChartData, ChartPlotter};
use egui::{Color32, RichText, ScrollArea};
use std::collections::HashMap;

/// Chart card configuration
const CHART_SPACING: f32 = 15.0;
const CARD_HEIGHT: f32 = 450.0; // Height for each card
const CHART_WIDTH: f32 = 780.0; // Fixed width for each chart card

/// Scrollable chart display area with responsive multi-column layout.
/// Automatically arranges charts into columns based on available width.
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

    /// Draw the chart viewer with responsive multi-column layout
    /// Charts have fixed width and automatically wrap to multiple columns
    pub fn show(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.chart_data.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("No Data").size(20.0));
            });
            return;
        }

        // Calculate how many columns fit in available width
        let avail_width = ui.available_width();
        let card_total_width = CHART_WIDTH + CHART_SPACING;
        let num_columns = ((avail_width / card_total_width).floor() as usize).max(1);

        // Calculate number of rows needed
        let total_items = self.data_type_order.len();
        let total_rows = (total_items + num_columns - 1) / num_columns; // Ceiling division
        let row_height = CARD_HEIGHT + CHART_SPACING;

        // Clone data for use in closure
        let order = self.data_type_order.clone();
        let chart_data = self.chart_data.clone();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                for row in row_range {
                    ui.horizontal(|ui| {
                        for col in 0..num_columns {
                            let idx = row * num_columns + col;
                            if idx < total_items {
                                if let Some(dt) = order.get(idx) {
                                    if let Some(data) = chart_data.get(dt) {
                                        let is_sig = data.stats.has_significant_results();
                                        Self::draw_chart_card_fixed_width(ui, data, is_sig);
                                    }
                                }
                                ui.add_space(CHART_SPACING);
                            }
                        }
                    });
                    ui.add_space(CHART_SPACING);
                }
            });
    }

    /// Draw a single chart card with fixed width
    fn draw_chart_card_fixed_width(ui: &mut egui::Ui, chart_data: &ChartData, is_sig: bool) {
        let border_color = if is_sig {
            Color32::from_rgb(220, 53, 69) // Red for significant
        } else {
            Color32::from_rgb(40, 167, 69) // Green for match
        };

        // Fixed card width
        let card_width = CHART_WIDTH - 20.0;
        let chart_width = (card_width - 40.0) / 2.0; // Two charts side by side

        egui::Frame::none()
            .rounding(8.0)
            .stroke(egui::Stroke::new(2.0, border_color))
            .fill(ui.visuals().widgets.noninteractive.bg_fill)
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

                    // Two charts side by side - adjusted widths
                    ui.horizontal(|ui| {
                        // Boxplot - narrower by 15px
                        ui.vertical(|ui| {
                            ui.set_width(chart_width - 15.0);
                            ui.label(RichText::new("Distribution by Group").size(14.0).strong());
                            ChartPlotter::draw_boxplot_chart(ui, chart_data, true);
                        });

                        ui.add_space(10.0);

                        // QQ Plot - wider by 15px
                        ui.vertical(|ui| {
                            ui.set_width(chart_width + 15.0);
                            ui.label(RichText::new("Normal Quantile Plot").size(14.0).strong());
                            ChartPlotter::draw_qq_chart(ui, chart_data, true);
                        });
                    });

                    ui.add_space(10.0);

                    // Statistics table
                    ChartPlotter::draw_stats_table(ui, &chart_data.stats);
                });
            });
    }
}
