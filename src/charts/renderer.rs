//! Static Chart Renderer Module
//! Generates SVG vector images matching the dynamic chart layout.

use crate::charts::ChartData;
use crate::stats::DataTypeStats;
use plotters::coord::ranged1d::{KeyPointHint, NoDefaultFormatting, Ranged, ValueFormatter};
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use std::collections::HashMap;
use std::fs;
use std::ops::Range;
use std::path::Path;

/// Custom X-axis range for QQ plot that displays p-values at specific Z positions
#[derive(Clone)]
struct ProbabilityAxisRange {
    range: Range<f64>,
}

impl ProbabilityAxisRange {
    fn new(start: f64, end: f64) -> Self {
        Self { range: start..end }
    }
}

impl Ranged for ProbabilityAxisRange {
    type FormatOption = NoDefaultFormatting;
    type ValueType = f64;

    fn map(&self, value: &f64, limit: (i32, i32)) -> i32 {
        // Linear mapping from Z-score to pixel
        let range_len = self.range.end - self.range.start;
        let normalized = (*value - self.range.start) / range_len;
        ((limit.1 - limit.0) as f64 * normalized) as i32 + limit.0
    }

    fn key_points<Hint: KeyPointHint>(&self, _hint: Hint) -> Vec<f64> {
        // Return Z-scores corresponding to specific p-values:
        // p=0.01, 0.05, 0.20, 0.25, 0.50, 0.75, 0.80, 0.95, 0.99
        vec![
            -2.326, -1.645, -0.842, -0.674, 0.0, 0.674, 0.842, 1.645, 2.326,
        ]
    }

    fn range(&self) -> Range<f64> {
        self.range.clone()
    }
}

impl ValueFormatter<f64> for ProbabilityAxisRange {
    fn format_ext(&self, value: &f64) -> String {
        // Convert Z-score to p-value for display
        use statrs::distribution::{ContinuousCDF, Normal};
        let normal = Normal::new(0.0, 1.0).unwrap();
        let p = normal.cdf(*value);
        format!("{:.2}", p)
    }
}

/// Color constants matching the dynamic chart colors
const CONTROL_COLOR: RGBColor = RGBColor(52, 152, 219); // Blue
const SIGNIFICANT_COLOR: RGBColor = RGBColor(220, 53, 69); // Red
const MATCH_COLOR: RGBColor = RGBColor(40, 167, 69); // Green

/// Color palette for non-control groups
const PALETTE: [RGBColor; 10] = [
    RGBColor(231, 76, 60),  // Red
    RGBColor(46, 204, 113), // Green
    RGBColor(155, 89, 182), // Purple
    RGBColor(243, 156, 18), // Orange
    RGBColor(26, 188, 156), // Teal
    RGBColor(233, 30, 99),  // Pink
    RGBColor(0, 188, 212),  // Cyan
    RGBColor(255, 87, 34),  // Deep Orange
    RGBColor(121, 85, 72),  // Brown
    RGBColor(96, 125, 139), // Blue Grey
];

/// Chart renderer for static SVG output
pub struct ChartRenderer;

impl ChartRenderer {
    /// Get color for a group
    fn get_group_color(group: &str, control_group: &str, group_index: usize) -> RGBColor {
        if group == control_group {
            CONTROL_COLOR
        } else {
            PALETTE[group_index % PALETTE.len()]
        }
    }

    /// Render a complete chart card to SVG file
    pub fn render_chart_card(
        chart_data: &ChartData,
        output_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let width = 1400u32;
        let height = 1000u32;

        let root = SVGBackend::new(output_path, (width, height)).into_drawing_area();
        root.fill(&WHITE)?;

        let is_sig = chart_data.stats.has_significant_results();
        let border_color = if is_sig {
            SIGNIFICANT_COLOR
        } else {
            MATCH_COLOR
        };

        // Draw border
        root.draw(&Rectangle::new(
            [(5, 5), (width as i32 - 5, height as i32 - 5)],
            border_color.stroke_width(3),
        ))?;

        // Split into regions
        let (title_area, rest) = root.split_vertically(60);
        let (legend_area, rest) = rest.split_vertically(40);
        let (charts_area, table_area) = rest.split_vertically(550);
        let (boxplot_area, qq_area) = charts_area.split_horizontally(width / 2);

        // Draw title
        Self::render_title(&title_area, chart_data, is_sig, border_color)?;

        // Draw legend
        Self::render_legend(&legend_area, chart_data)?;

        // Draw charts
        Self::render_boxplot(&boxplot_area, chart_data)?;
        Self::render_qq_plot(&qq_area, chart_data)?;

        // Draw stats table
        Self::render_stats_table(&table_area, &chart_data.stats)?;

        root.present()?;
        Ok(())
    }

    /// Render a complete chart card to PNG file (for PPT embedding)
    pub fn render_chart_card_png(
        chart_data: &ChartData,
        output_path: &Path,
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use plotters::prelude::BitMapBackend;

        let root = BitMapBackend::new(output_path, (width, height)).into_drawing_area();
        root.fill(&WHITE)?;

        let is_sig = chart_data.stats.has_significant_results();
        let border_color = if is_sig {
            SIGNIFICANT_COLOR
        } else {
            MATCH_COLOR
        };

        // Draw border
        root.draw(&Rectangle::new(
            [(3, 3), (width as i32 - 3, height as i32 - 3)],
            border_color.stroke_width(2),
        ))?;

        // Split into regions (proportionally scaled)
        let title_height = (height as f64 * 0.06) as u32;
        let legend_height = (height as f64 * 0.04) as u32;
        let charts_height = (height as f64 * 0.55) as u32;

        let (title_area, rest) = root.split_vertically(title_height);
        let (legend_area, rest) = rest.split_vertically(legend_height);
        let (charts_area, table_area) = rest.split_vertically(charts_height);
        let (boxplot_area, qq_area) = charts_area.split_horizontally(width / 2);

        // Draw title
        Self::render_title(&title_area, chart_data, is_sig, border_color)?;

        // Draw legend
        Self::render_legend(&legend_area, chart_data)?;

        // Draw charts
        Self::render_boxplot(&boxplot_area, chart_data)?;
        Self::render_qq_plot(&qq_area, chart_data)?;

        // Draw stats table
        Self::render_stats_table(&table_area, &chart_data.stats)?;

        root.present()?;
        Ok(())
    }

    /// Render a complete chart card to in-memory PNG bytes (for PPT embedding without disk I/O)
    pub fn render_chart_card_to_bytes(
        chart_data: &ChartData,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use plotters::prelude::BitMapBackend;

        // Create in-memory buffer for the image
        let mut buffer = vec![0u8; (width * height * 3) as usize];

        {
            let root = BitMapBackend::with_buffer(&mut buffer, (width, height)).into_drawing_area();
            root.fill(&WHITE)?;

            let is_sig = chart_data.stats.has_significant_results();
            let border_color = if is_sig {
                SIGNIFICANT_COLOR
            } else {
                MATCH_COLOR
            };

            // Draw border
            root.draw(&Rectangle::new(
                [(3, 3), (width as i32 - 3, height as i32 - 3)],
                border_color.stroke_width(2),
            ))?;

            // Split into regions
            let title_height = (height as f64 * 0.06) as u32;
            let legend_height = (height as f64 * 0.04) as u32;
            let charts_height = (height as f64 * 0.55) as u32;

            let (title_area, rest) = root.split_vertically(title_height);
            let (legend_area, rest) = rest.split_vertically(legend_height);
            let (charts_area, table_area) = rest.split_vertically(charts_height);
            let (boxplot_area, qq_area) = charts_area.split_horizontally(width / 2);

            Self::render_title(&title_area, chart_data, is_sig, border_color)?;
            Self::render_legend(&legend_area, chart_data)?;
            Self::render_boxplot(&boxplot_area, chart_data)?;
            Self::render_qq_plot(&qq_area, chart_data)?;
            Self::render_stats_table(&table_area, &chart_data.stats)?;

            root.present()?;
        }

        // Convert RGB buffer to PNG bytes
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, _> =
            ImageBuffer::from_raw(width, height, buffer).ok_or("Failed to create image buffer")?;

        let mut png_bytes = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut png_bytes);
        img.write_to(&mut cursor, image::ImageFormat::Png)?;

        Ok(png_bytes)
    }

    /// Render title with icon
    fn render_title<DB: DrawingBackend>(
        area: &DrawingArea<DB, plotters::coord::Shift>,
        chart_data: &ChartData,
        is_sig: bool,
        color: RGBColor,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        DB::ErrorType: 'static,
    {
        let icon = if is_sig { "!" } else { "OK" };
        let title = format!("[{}] Analysis: {}", icon, chart_data.data_type);

        area.draw(&Text::new(
            title,
            (30, 20),
            ("sans-serif", 28).into_font().color(&color),
        ))?;

        Ok(())
    }

    /// Render legend showing group colors
    fn render_legend<DB: DrawingBackend>(
        area: &DrawingArea<DB, plotters::coord::Shift>,
        chart_data: &ChartData,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        DB::ErrorType: 'static,
    {
        let mut x_offset = 30i32;
        let mut non_ctrl_idx = 0;
        let box_size = 20i32;
        let box_y = 8i32;
        let font_size = 24;

        for group in chart_data.stats.get_ordered_groups() {
            let color =
                Self::get_group_color(&group, &chart_data.stats.control_group, non_ctrl_idx);
            if group != chart_data.stats.control_group {
                non_ctrl_idx += 1;
            }

            // Draw color square
            area.draw(&Rectangle::new(
                [(x_offset, box_y), (x_offset + box_size, box_y + box_size)],
                color.filled(),
            ))?;

            // Draw group name - use center anchor for natural vertical alignment with box center
            let text_y = box_y + box_size / 2;
            let style = TextStyle::from(("sans-serif", font_size).into_font())
                .color(&BLACK)
                .pos(Pos::new(HPos::Left, VPos::Center));

            area.draw(&Text::new(
                group.clone(),
                (x_offset + box_size + 8, text_y),
                style,
            ))?;

            x_offset += 150;
        }

        Ok(())
    }

    /// Render boxplot chart
    fn render_boxplot<DB: DrawingBackend>(
        area: &DrawingArea<DB, plotters::coord::Shift>,
        chart_data: &ChartData,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        DB::ErrorType: 'static,
    {
        let ordered_groups = chart_data.stats.get_ordered_groups();
        let control_group = &chart_data.stats.control_group;

        // Calculate y range from data
        let mut all_values: Vec<f64> = Vec::new();
        for values in chart_data.data_by_group.values() {
            all_values.extend(values.iter().filter(|v| !v.is_nan()));
        }

        if all_values.is_empty() {
            return Ok(());
        }

        let y_min = all_values.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_max = all_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let y_margin = (y_max - y_min) * 0.1;

        let mut chart = ChartBuilder::on(area)
            .margin(20)
            .x_label_area_size(50)
            .y_label_area_size(80)
            .caption("Distribution by Group", ("sans-serif", 24))
            .build_cartesian_2d(
                -0.5f64..(ordered_groups.len() as f64 - 0.5),
                (y_min - y_margin)..(y_max + y_margin),
            )?;

        chart
            .configure_mesh()
            .x_labels(ordered_groups.len())
            .x_label_formatter(&|x| {
                let idx = x.round() as usize;
                ordered_groups.get(idx).cloned().unwrap_or_default()
            })
            .y_desc("Value")
            .label_style(("sans-serif", 18))
            .axis_desc_style(("sans-serif", 24))
            .draw()?;

        let mut non_ctrl_idx = 0;
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

            let color = Self::get_group_color(group, control_group, non_ctrl_idx);
            if group != control_group {
                non_ctrl_idx += 1;
            }

            // Calculate boxplot statistics
            let mut sorted: Vec<f64> = values.iter().filter(|v| !v.is_nan()).cloned().collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let n = sorted.len();
            if n == 0 {
                continue;
            }

            let q1 = sorted[n / 4];
            let median = sorted[n / 2];
            let q3 = sorted[3 * n / 4];
            let iqr = q3 - q1;
            let whisker_low = sorted
                .iter()
                .find(|&&v| v >= q1 - 1.5 * iqr)
                .copied()
                .unwrap_or(q1);
            let whisker_high = sorted
                .iter()
                .rev()
                .find(|&&v| v <= q3 + 1.5 * iqr)
                .copied()
                .unwrap_or(q3);
            let mean = sorted.iter().sum::<f64>() / n as f64;
            means.push((i as f64, mean));

            let x = i as f64;
            let box_width = 0.35;

            // Draw box (Q1 to Q3)
            chart.draw_series(std::iter::once(Rectangle::new(
                [(x - box_width, q1), (x + box_width, q3)],
                color.mix(0.3).filled(),
            )))?;

            // Draw box outline
            chart.draw_series(std::iter::once(Rectangle::new(
                [(x - box_width, q1), (x + box_width, q3)],
                color.stroke_width(2),
            )))?;

            // Draw median line
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x - box_width, median), (x + box_width, median)],
                color.stroke_width(2),
            )))?;

            // Draw whiskers
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x, q1), (x, whisker_low)],
                color.stroke_width(1),
            )))?;
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x, q3), (x, whisker_high)],
                color.stroke_width(1),
            )))?;

            // Draw whisker caps
            let cap_width = 0.15;
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x - cap_width, whisker_low), (x + cap_width, whisker_low)],
                color.stroke_width(1),
            )))?;
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x - cap_width, whisker_high), (x + cap_width, whisker_high)],
                color.stroke_width(1),
            )))?;

            // Draw scatter points with beeswarm-style distribution
            // Similar values spread outward from center
            let mut sorted_with_idx: Vec<(usize, f64)> = values
                .iter()
                .enumerate()
                .filter(|(_, v)| !v.is_nan())
                .map(|(i, &v)| (i, v))
                .collect();
            sorted_with_idx
                .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            // Group by proximity and assign horizontal offsets
            let y_range = y_max - y_min;
            let bin_size = y_range * 0.02; // Points within 2% of range are "similar"
            let max_jitter = 0.35;

            let mut scatter_points: Vec<(f64, f64)> = Vec::new();
            let mut current_bin_y = f64::NEG_INFINITY;
            let mut bin_count = 0;

            for (_idx, y_val) in &sorted_with_idx {
                if (*y_val - current_bin_y).abs() > bin_size {
                    // New bin
                    current_bin_y = *y_val;
                    bin_count = 0;
                }

                // First point in bin at center, then alternate left/right
                let jitter = if bin_count == 0 {
                    0.0 // First point at center
                } else {
                    let offset_idx = (bin_count - 1) / 2 + 1;
                    let sign = if bin_count % 2 == 1 { 1.0 } else { -1.0 };
                    sign * offset_idx as f64 * 0.06
                };
                let jitter = jitter.clamp(-max_jitter, max_jitter);

                scatter_points.push((x + jitter, *y_val));
                bin_count += 1;
            }

            chart.draw_series(
                scatter_points
                    .iter()
                    .map(|&(px, py)| Circle::new((px, py), 3, color.mix(0.6).filled())),
            )?;
        }

        // Draw mean line
        if means.len() > 1 {
            chart.draw_series(std::iter::once(PathElement::new(
                means.clone(),
                BLACK.stroke_width(2),
            )))?;
        }

        Ok(())
    }

    /// Render QQ plot with normal quantiles on x-axis
    fn render_qq_plot<DB: DrawingBackend>(
        area: &DrawingArea<DB, plotters::coord::Shift>,
        chart_data: &ChartData,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        DB::ErrorType: 'static,
    {
        let ordered_groups = chart_data.stats.get_ordered_groups();
        let control_group = &chart_data.stats.control_group;

        // Calculate y range from data
        let mut all_values: Vec<f64> = Vec::new();
        for values in chart_data.data_by_group.values() {
            all_values.extend(values.iter().filter(|v| !v.is_nan()));
        }

        if all_values.is_empty() {
            return Ok(());
        }

        let y_min = all_values.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_max = all_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let y_margin = (y_max - y_min) * 0.1;

        // X-axis: using custom ProbabilityAxisRange for specific p-value tick positions
        let x_axis = ProbabilityAxisRange::new(-3.0, 3.0);
        let mut chart = ChartBuilder::on(area)
            .margin(20)
            .x_label_area_size(50)
            .y_label_area_size(80)
            .caption("Normal Quantile Plot", ("sans-serif", 24))
            .build_cartesian_2d(x_axis, (y_min - y_margin)..(y_max + y_margin))?;

        // Configure mesh - tick positions come from ProbabilityAxisRange::key_points()
        chart
            .configure_mesh()
            .x_desc("Probability")
            .label_style(("sans-serif", 18))
            .axis_desc_style(("sans-serif", 24))
            .draw()?;

        let mut non_ctrl_idx = 0;

        for group in &ordered_groups {
            let values = chart_data
                .data_by_group
                .get(group)
                .cloned()
                .unwrap_or_default();
            if values.is_empty() {
                continue;
            }

            let color = Self::get_group_color(group, control_group, non_ctrl_idx);
            if group != control_group {
                non_ctrl_idx += 1;
            }

            // Sort values for quantile plot
            let mut sorted: Vec<f64> = values.iter().filter(|v| !v.is_nan()).cloned().collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let n = sorted.len();
            if n == 0 {
                continue;
            }

            // Calculate theoretical normal quantiles (Z-scores)
            let points: Vec<(f64, f64)> = sorted
                .iter()
                .enumerate()
                .map(|(i, &val)| {
                    // Use (i + 0.5) / n for probability, then convert to Z-score
                    let p = (i as f64 + 0.5) / n as f64;
                    let z = Self::probit(p);
                    (z, val)
                })
                .collect();

            // Draw line
            chart.draw_series(std::iter::once(PathElement::new(
                points.clone(),
                color.stroke_width(2),
            )))?;

            // Draw points
            chart.draw_series(
                points
                    .iter()
                    .map(|&(x, y)| Circle::new((x, y), 3, color.filled())),
            )?;
        }

        Ok(())
    }

    /// Probit function (inverse normal CDF) - converts probability to Z-score
    fn probit(p: f64) -> f64 {
        use statrs::distribution::{ContinuousCDF, Normal};
        let normal = Normal::new(0.0, 1.0).unwrap();
        normal.inverse_cdf(p.clamp(0.001, 0.999))
    }

    /// Render statistics table with grid lines - centered with even column widths
    fn render_stats_table<DB: DrawingBackend>(
        area: &DrawingArea<DB, plotters::coord::Shift>,
        stats: &DataTypeStats,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        DB::ErrorType: 'static,
    {
        let headers = [
            "Group", "N", "Mean", "Median", "Std", "P05", "P95", "(M-C)/σ", "P-value",
        ];
        let num_cols = headers.len();
        let num_rows = stats.group_stats.len() + 1; // +1 for header

        // Table dimensions - centered with 900px width
        let table_width = 900i32;
        let (canvas_width, canvas_height) = area.dim_in_pixel();
        let start_x = (canvas_width as i32 - table_width) / 2; // Center horizontally

        // Evenly split column widths
        let col_width = table_width / num_cols as i32;
        let actual_table_width = col_width * num_cols as i32; // Use actual width to avoid rounding issues

        let row_height = 40;
        let table_height = row_height * num_rows as i32;

        // Center table vertically in the area
        let start_y = (canvas_height as i32 - table_height) / 2;

        let font_size = 24;

        let light_gray = RGBColor(180, 180, 180);

        // Draw horizontal lines (including top and bottom borders)
        for row in 0..=num_rows {
            let y = start_y + row as i32 * row_height;
            let stroke = if row == 0 || row == 1 || row == num_rows {
                BLACK.stroke_width(2)
            } else {
                light_gray.stroke_width(1)
            };
            area.draw(&PathElement::new(
                vec![(start_x, y), (start_x + actual_table_width, y)],
                stroke,
            ))?;
        }

        // Draw vertical lines (evenly spaced, including left and right borders)
        for col in 0..=num_cols {
            let x = start_x + col as i32 * col_width;
            let stroke = if col == 0 || col == num_cols {
                BLACK.stroke_width(2)
            } else {
                light_gray.stroke_width(1)
            };
            area.draw(&PathElement::new(
                vec![(x, start_y), (x, start_y + table_height)],
                stroke,
            ))?;
        }

        // Draw table header text - centered in cells using Pos anchor
        for (i, header) in headers.iter().enumerate() {
            let cell_center_x = start_x + i as i32 * col_width + col_width / 2;
            let cell_center_y = start_y + row_height / 2;

            let style = TextStyle::from(("sans-serif", font_size).into_font())
                .color(&BLACK)
                .pos(Pos::new(HPos::Center, VPos::Center));

            area.draw(&Text::new(*header, (cell_center_x, cell_center_y), style))?;
        }

        // Draw data rows
        let mut row_idx = 1;

        for group_name in stats.get_ordered_groups() {
            if let Some(gs) = stats.group_stats.get(&group_name) {
                let is_control = group_name == stats.control_group;
                let text_color = if is_control {
                    CONTROL_COLOR
                } else if gs.is_significant {
                    SIGNIFICANT_COLOR
                } else {
                    BLACK
                };

                let row_data = [
                    gs.group_name.clone(),
                    gs.count.to_string(),
                    format!("{:.3}", gs.mean),
                    format!("{:.3}", gs.median),
                    format!("{:.3}", gs.std),
                    format!("{:.3}", gs.p05),
                    format!("{:.3}", gs.p95),
                    gs.std_diff_from_control
                        .map(|d| format!("{:.3}", d))
                        .unwrap_or("-".to_string()),
                    gs.p_value
                        .map(|p| format!("{:.4}", p))
                        .unwrap_or("-".to_string()),
                ];

                // Cell center Y position for this row
                let cell_center_y = start_y + row_idx * row_height + row_height / 2;

                for (i, value) in row_data.iter().enumerate() {
                    let cell_center_x = start_x + i as i32 * col_width + col_width / 2;
                    // P-value and group name use text_color, others use black
                    let color = if i == 0 || i == 8 { text_color } else { BLACK };

                    let style = TextStyle::from(("sans-serif", font_size).into_font())
                        .color(&color)
                        .pos(Pos::new(HPos::Center, VPos::Center));

                    area.draw(&Text::new(
                        value.clone(),
                        (cell_center_x, cell_center_y),
                        style,
                    ))?;
                }

                row_idx += 1;
            }
        }

        Ok(())
    }

    /// Export all charts to tem folder (PNG format)
    pub fn export_all_charts(
        chart_data: &HashMap<String, ChartData>,
        data_type_order: &[String],
        base_path: &Path,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        // Create tem folder
        let tem_path = base_path.join("tem");
        fs::create_dir_all(&tem_path)?;

        let width = 1400u32;
        let height = 1000u32;

        let mut count = 0;
        for data_type in data_type_order {
            if let Some(data) = chart_data.get(data_type) {
                // Sanitize filename
                let safe_name: String = data_type
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '_' || c == '-' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();

                let file_path = tem_path.join(format!("{}.png", safe_name));
                Self::render_chart_card_png(data, &file_path, width, height)?;
                count += 1;
            }
        }

        Ok(count)
    }
}
