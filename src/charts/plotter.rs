//! Chart Plotter Module
//! Creates interactive visualizations using egui_plot.

use crate::stats::DataTypeStats;
use egui::{Color32, RichText};
use egui_plot::{BoxElem, BoxPlot, BoxSpread, Legend, Line, Plot, PlotPoints, Points};
use std::collections::HashMap;

/// Color palette for groups
pub const CONTROL_COLOR: Color32 = Color32::from_rgb(52, 152, 219); // Blue

pub const PALETTE: [Color32; 10] = [
    Color32::from_rgb(231, 76, 60),  // Red
    Color32::from_rgb(46, 204, 113), // Green
    Color32::from_rgb(155, 89, 182), // Purple
    Color32::from_rgb(243, 156, 18), // Orange
    Color32::from_rgb(26, 188, 156), // Teal
    Color32::from_rgb(233, 30, 99),  // Pink
    Color32::from_rgb(0, 188, 212),  // Cyan
    Color32::from_rgb(255, 87, 34),  // Deep Orange
    Color32::from_rgb(121, 85, 72),  // Brown
    Color32::from_rgb(96, 125, 139), // Blue Grey
];

/// Chart data for a single data type
#[derive(Clone)]
pub struct ChartData {
    pub data_type: String,
    pub data_by_group: HashMap<String, Vec<f64>>,
    pub stats: DataTypeStats,
}

/// Creates scientific visualization charts using egui_plot.
pub struct ChartPlotter;

impl ChartPlotter {
    /// Get color for a group.
    pub fn get_group_color(group: &str, control_group: &str, group_index: usize) -> Color32 {
        if group == control_group {
            CONTROL_COLOR
        } else {
            PALETTE[group_index % PALETTE.len()]
        }
    }

    /// Calculate beeswarm positions for points with duplicate values.
    pub fn beeswarm_positions(y_values: &[f64], center: f64, width: f64) -> Vec<f64> {
        let n = y_values.len();
        if n == 0 {
            return Vec::new();
        }

        let mut positions = vec![center; n];

        // Round values and find duplicates
        let precision = 1e6;
        let mut value_indices: HashMap<i64, Vec<usize>> = HashMap::new();

        for (i, &y) in y_values.iter().enumerate() {
            let key = (y * precision).round() as i64;
            value_indices.entry(key).or_default().push(i);
        }

        // Spread duplicates symmetrically
        for indices in value_indices.values() {
            if indices.len() > 1 {
                let count = indices.len();
                let step = width / (count.max(2) - 1) as f64;
                let start = center - width / 2.0;

                for (i, &idx) in indices.iter().enumerate() {
                    positions[idx] = start + i as f64 * step;
                }
            }
        }

        positions
    }

    /// Draw boxplot with scatter overlay for a chart
    /// X-axis: groups, Y-axis: values
    pub fn draw_boxplot_chart(ui: &mut egui::Ui, chart_data: &ChartData, full_size: bool) {
        let ordered_groups = chart_data.stats.get_ordered_groups();
        let control_group = &chart_data.stats.control_group;

        let height = if full_size { 300.0 } else { 180.0 };

        // Create custom x-axis labels
        let x_labels: Vec<String> = ordered_groups.clone();

        Plot::new(format!("boxplot_{}", chart_data.data_type))
            .height(height)
            .allow_zoom(full_size)
            .allow_drag(full_size)
            .allow_scroll(false)
            .x_axis_label("Group")
            .y_axis_label("Value")
            .x_axis_formatter(move |mark, _range| {
                let idx = mark.value.round() as usize;
                if idx < x_labels.len() {
                    x_labels[idx].clone()
                } else {
                    String::new()
                }
            })
            .show(ui, |plot_ui| {
                let mut non_control_idx = 0;
                let mut means: Vec<(f64, f64)> = Vec::new();

                for (i, group) in ordered_groups.iter().enumerate() {
                    let values = chart_data
                        .data_by_group
                        .get(group)
                        .cloned()
                        .unwrap_or_default();
                    if values.is_empty() {
                        continue;
                    }

                    let color = Self::get_group_color(group, control_group, non_control_idx);
                    if group != control_group {
                        non_control_idx += 1;
                    }

                    // Calculate statistics for boxplot
                    let mut sorted = values.clone();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                    let n = sorted.len();
                    let q1_idx = n / 4;
                    let q2_idx = n / 2;
                    let q3_idx = 3 * n / 4;

                    let q1 = sorted.get(q1_idx).copied().unwrap_or(0.0);
                    let median = sorted.get(q2_idx).copied().unwrap_or(0.0);
                    let q3 = sorted.get(q3_idx).copied().unwrap_or(0.0);
                    let iqr = q3 - q1;
                    let whisker_low = sorted
                        .iter()
                        .copied()
                        .find(|&v| v >= q1 - 1.5 * iqr)
                        .unwrap_or(q1);
                    let whisker_high = sorted
                        .iter()
                        .rev()
                        .copied()
                        .find(|&v| v <= q3 + 1.5 * iqr)
                        .unwrap_or(q3);

                    let mean = values.iter().sum::<f64>() / values.len() as f64;
                    means.push((i as f64, mean));

                    // Draw boxplot
                    let box_elem = BoxElem::new(
                        i as f64,
                        BoxSpread::new(whisker_low, q1, median, q3, whisker_high),
                    )
                    .box_width(0.5)
                    .fill(color.gamma_multiply(0.3))
                    .stroke(egui::Stroke::new(1.5, color));

                    plot_ui.box_plot(BoxPlot::new(vec![box_elem]).name(group));

                    // Draw scatter points (all points, no sampling)
                    let x_positions = Self::beeswarm_positions(&values, i as f64, 0.35);
                    let points: PlotPoints = x_positions
                        .iter()
                        .zip(values.iter())
                        .map(|(&x, &y)| [x, y])
                        .collect();

                    plot_ui.points(
                        Points::new(points)
                            .radius(3.0)
                            .color(color.gamma_multiply(0.7))
                            .name(format!("{} points", group)),
                    );
                }

                // Draw mean line
                if means.len() > 1 {
                    let line_points: PlotPoints = means.iter().map(|&(x, y)| [x, y]).collect();
                    plot_ui.line(
                        Line::new(line_points)
                            .color(Color32::BLACK)
                            .width(1.5)
                            .name("Mean"),
                    );
                }
            });
    }

    /// Draw Normal Quantile Plot for a chart
    /// X-axis: quantile (0 to 1) with percentage labels, Y-axis: sample value at that quantile
    pub fn draw_qq_chart(ui: &mut egui::Ui, chart_data: &ChartData, full_size: bool) {
        let ordered_groups = chart_data.stats.get_ordered_groups();
        let control_group = &chart_data.stats.control_group;

        let height = if full_size { 300.0 } else { 180.0 };

        Plot::new(format!("qq_{}", chart_data.data_type))
            .height(height)
            .x_axis_label("Quantile")
            .y_axis_label("Value")
            .allow_zoom(full_size)
            .allow_drag(full_size)
            .allow_scroll(false)
            .clamp_grid(true) // Prevent axis labels from extending outside plot area
            // Set x-axis range
            .include_x(0.0)
            .include_x(1.0)
            // Force specific quantile tick marks on x-axis
            // Use same large step_size for all to ensure they all display
            .x_grid_spacer(|_input| {
                vec![
                    egui_plot::GridMark {
                        value: 0.01,
                        step_size: 1.0,
                    },
                    egui_plot::GridMark {
                        value: 0.05,
                        step_size: 1.0,
                    },
                    egui_plot::GridMark {
                        value: 0.1,
                        step_size: 1.0,
                    },
                    egui_plot::GridMark {
                        value: 0.25,
                        step_size: 1.0,
                    },
                    egui_plot::GridMark {
                        value: 0.5,
                        step_size: 1.0,
                    },
                    egui_plot::GridMark {
                        value: 0.75,
                        step_size: 1.0,
                    },
                    egui_plot::GridMark {
                        value: 0.9,
                        step_size: 1.0,
                    },
                    egui_plot::GridMark {
                        value: 0.95,
                        step_size: 1.0,
                    },
                    egui_plot::GridMark {
                        value: 0.99,
                        step_size: 1.0,
                    },
                ]
            })
            // Custom x-axis formatter to show short decimal values (avoid overlap)
            .x_axis_formatter(|mark, _range| {
                let v = mark.value;
                if v >= 0.0 && v <= 1.0 {
                    // Format with short notation (e.g., ".05" instead of "0.05")
                    let formatted = format!("{:.2}", v);
                    let trimmed = formatted
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .to_string();
                    // Remove leading "0" for values < 1 to save space
                    if trimmed.starts_with("0.") {
                        trimmed[1..].to_string()
                    } else if trimmed == "0" {
                        "0".to_string()
                    } else if trimmed == "1" {
                        "1".to_string()
                    } else {
                        trimmed
                    }
                } else {
                    String::new()
                }
            })
            .show(ui, |plot_ui| {
                let mut non_control_idx = 0;

                for group in &ordered_groups {
                    let values = chart_data
                        .data_by_group
                        .get(group)
                        .cloned()
                        .unwrap_or_default();
                    if values.is_empty() {
                        continue;
                    }

                    let color = Self::get_group_color(group, control_group, non_control_idx);
                    if group != control_group {
                        non_control_idx += 1;
                    }

                    // Sort values for quantile plot
                    let mut sorted = values.clone();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                    // Calculate quantile positions (0 to 1)
                    // Using (i) / (n-1) formula so first point is at 0% and last at 100%
                    let n = sorted.len();
                    let points_vec: Vec<[f64; 2]> = sorted
                        .iter()
                        .enumerate()
                        .map(|(i, &val)| {
                            // Quantile position from 0 to 1
                            let quantile = if n > 1 {
                                i as f64 / (n - 1) as f64
                            } else {
                                0.5
                            };
                            [quantile, val]
                        })
                        .collect();

                    plot_ui.line(
                        Line::new(PlotPoints::from_iter(points_vec.iter().copied()))
                            .color(color)
                            .width(1.5)
                            .name(group),
                    );

                    plot_ui.points(
                        Points::new(PlotPoints::from_iter(points_vec.iter().copied()))
                            .radius(3.0)
                            .color(color),
                    );
                }
            });
    }

    /// Standard normal CDF (cumulative distribution function)
    fn normal_cdf(x: f64) -> f64 {
        0.5 * (1.0 + Self::erf(x / std::f64::consts::SQRT_2))
    }

    /// Error function approximation
    fn erf(x: f64) -> f64 {
        let a1 = 0.254829592;
        let a2 = -0.284496736;
        let a3 = 1.421413741;
        let a4 = -1.453152027;
        let a5 = 1.061405429;
        let p = 0.3275911;

        let sign = if x < 0.0 { -1.0 } else { 1.0 };
        let x = x.abs();

        let t = 1.0 / (1.0 + p * x);
        let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

        sign * y
    }

    /// Standard normal quantile function (inverse CDF) - approximation
    fn normal_ppf(p: f64) -> f64 {
        if p <= 0.0 {
            return f64::NEG_INFINITY;
        }
        if p >= 1.0 {
            return f64::INFINITY;
        }

        let a = [
            -3.969683028665376e+01,
            2.209460984245205e+02,
            -2.759285104469687e+02,
            1.383577518672690e+02,
            -3.066479806614716e+01,
            2.506628277459239e+00,
        ];
        let b = [
            -5.447609879822406e+01,
            1.615858368580409e+02,
            -1.556989798598866e+02,
            6.680131188771972e+01,
            -1.328068155288572e+01,
        ];
        let c = [
            -7.784894002430293e-03,
            -3.223964580411365e-01,
            -2.400758277161838e+00,
            -2.549732539343734e+00,
            4.374664141464968e+00,
            2.938163982698783e+00,
        ];
        let d = [
            7.784695709041462e-03,
            3.224671290700398e-01,
            2.445134137142996e+00,
            3.754408661907416e+00,
        ];

        let p_low = 0.02425;
        let p_high = 1.0 - p_low;

        if p < p_low {
            let q = (-2.0 * p.ln()).sqrt();
            (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
                / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
        } else if p <= p_high {
            let q = p - 0.5;
            let r = q * q;
            (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q
                / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1.0)
        } else {
            let q = (-2.0 * (1.0 - p).ln()).sqrt();
            -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
                / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
        }
    }

    /// Draw statistics table
    pub fn draw_stats_table(ui: &mut egui::Ui, stats: &DataTypeStats) {
        egui::Frame::none()
            .fill(ui.visuals().widgets.noninteractive.bg_fill)
            .rounding(5.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                // Use data_type to create unique Grid ID
                egui::Grid::new(ui.make_persistent_id(format!("stats_table_{}", &stats.data_type)))
                    .striped(true)
                    .min_col_width(55.0)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        // Headers
                        ui.label(RichText::new("Group").strong().size(11.0));
                        ui.label(RichText::new("N").strong().size(11.0));
                        ui.label(RichText::new("Mean").strong().size(11.0));
                        ui.label(RichText::new("Median").strong().size(11.0));
                        ui.label(RichText::new("Std").strong().size(11.0));
                        ui.label(RichText::new("P95").strong().size(11.0));
                        ui.label(RichText::new("P05").strong().size(11.0));
                        ui.label(RichText::new("(M-C)/σ").strong().size(11.0));
                        ui.label(RichText::new("P-value").strong().size(11.0));
                        ui.end_row();

                        // Get default text color from theme
                        let default_text_color = ui.visuals().text_color();

                        // Data rows
                        for group_name in stats.get_ordered_groups() {
                            if let Some(gs) = stats.group_stats.get(&group_name) {
                                let is_control = group_name == stats.control_group;
                                let text_color = if is_control {
                                    CONTROL_COLOR
                                } else if gs.is_significant {
                                    Color32::from_rgb(220, 53, 69)
                                } else {
                                    default_text_color
                                };

                                ui.label(
                                    RichText::new(&gs.group_name).size(11.0).color(text_color),
                                );
                                ui.label(RichText::new(gs.count.to_string()).size(11.0));
                                ui.label(RichText::new(format!("{:.3}", gs.mean)).size(11.0));
                                ui.label(RichText::new(format!("{:.3}", gs.median)).size(11.0));
                                ui.label(RichText::new(format!("{:.3}", gs.std)).size(11.0));
                                ui.label(RichText::new(format!("{:.3}", gs.p95)).size(11.0));
                                ui.label(RichText::new(format!("{:.3}", gs.p05)).size(11.0));

                                if let Some(diff) = gs.std_diff_from_control {
                                    ui.label(RichText::new(format!("{:.3}", diff)).size(11.0));
                                } else {
                                    ui.label(RichText::new("-").size(11.0));
                                }

                                if let Some(p) = gs.p_value {
                                    let p_color = if gs.is_significant {
                                        Color32::from_rgb(220, 53, 69)
                                    } else {
                                        default_text_color
                                    };
                                    ui.label(
                                        RichText::new(format!("{:.4}", p))
                                            .size(11.0)
                                            .color(p_color),
                                    );
                                } else {
                                    ui.label(RichText::new("-").size(11.0));
                                }
                                ui.end_row();
                            }
                        }
                    });
            });
    }
}
