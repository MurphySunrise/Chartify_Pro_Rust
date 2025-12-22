//! Static Chart Renderer Module
//! Generates SVG vector images matching the dynamic chart layout.

use crate::charts::ChartData;
use crate::stats::DataTypeStats;
use plotters::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
        let font_size = 18;

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

            // Draw group name - vertically centered with box middle
            // Text baseline should be at box vertical center + half font height
            area.draw(&Text::new(
                group.clone(),
                (x_offset + box_size + 8, box_y + box_size / 2 - 3),
                ("sans-serif", font_size).into_font().color(&BLACK),
            ))?;

            x_offset += 130;
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
            .y_label_area_size(60)
            .caption("Distribution by Group", ("sans-serif", 20))
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

            // Draw scatter points
            let scatter_points: Vec<_> = values
                .iter()
                .enumerate()
                .filter(|(_, v)| !v.is_nan())
                .map(|(j, &v)| {
                    let jitter = ((j as f64 * 0.618).fract() - 0.5) * 0.5;
                    (x + jitter, v)
                })
                .collect();

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

        // X-axis: theoretical normal quantiles (Z-scores), typically -3 to +3
        let mut chart = ChartBuilder::on(area)
            .margin(20)
            .x_label_area_size(50)
            .y_label_area_size(60)
            .caption("Normal Quantile Plot", ("sans-serif", 20))
            .build_cartesian_2d(-3.0f64..3.0f64, (y_min - y_margin)..(y_max + y_margin))?;

        chart
            .configure_mesh()
            .x_desc("Theoretical Quantiles (Z)")
            .y_desc("Sample Value")
            .x_label_formatter(&|x| format!("{:.1}", x))
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
        if p <= 0.0 {
            return -3.0;
        }
        if p >= 1.0 {
            return 3.0;
        }

        // Rational approximation for probit function
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

    /// Render statistics table with grid lines
    fn render_stats_table<DB: DrawingBackend>(
        area: &DrawingArea<DB, plotters::coord::Shift>,
        stats: &DataTypeStats,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        DB::ErrorType: 'static,
    {
        let headers = [
            "Group", "N", "Mean", "Median", "Std", "P95", "P05", "(M-C)/σ", "P-value",
        ];
        // Wider columns to match two charts width (~1320px total)
        let col_widths: [i32; 9] = [180, 100, 140, 140, 120, 120, 120, 140, 140];
        let row_height = 40;
        let start_x = 40;
        let start_y = 10;
        let num_rows = stats.group_stats.len() + 1; // +1 for header
        let table_width: i32 = col_widths.iter().sum();
        let table_height = row_height * num_rows as i32;
        let font_size = 28;

        let light_gray = RGBColor(180, 180, 180);

        // Draw table outer border
        area.draw(&Rectangle::new(
            [
                (start_x, start_y),
                (start_x + table_width, start_y + table_height),
            ],
            BLACK.stroke_width(2),
        ))?;

        // Draw horizontal lines
        for row in 0..=num_rows {
            let y = start_y + row as i32 * row_height;
            let stroke = if row == 0 || row == 1 {
                BLACK.stroke_width(2)
            } else {
                light_gray.stroke_width(1)
            };
            area.draw(&PathElement::new(
                vec![(start_x, y), (start_x + table_width, y)],
                stroke,
            ))?;
        }

        // Draw vertical lines
        let mut x = start_x;
        for width in col_widths.iter() {
            area.draw(&PathElement::new(
                vec![(x, start_y), (x, start_y + table_height)],
                light_gray.stroke_width(1),
            ))?;
            x += width;
        }
        // Right border
        area.draw(&PathElement::new(
            vec![
                (start_x + table_width, start_y),
                (start_x + table_width, start_y + table_height),
            ],
            BLACK.stroke_width(2),
        ))?;

        // Draw table header text - centered in cells
        let mut x_offset = start_x;
        for (i, header) in headers.iter().enumerate() {
            // Center horizontally: col_width/2 - approx text width/2
            let text_width_approx = header.len() as i32 * 8;
            let text_x = x_offset + (col_widths[i] - text_width_approx) / 2;
            let text_y = start_y + row_height / 2 - 2;
            area.draw(&Text::new(
                *header,
                (text_x.max(x_offset + 4), text_y),
                ("sans-serif", font_size).into_font().color(&BLACK),
            ))?;
            x_offset += col_widths[i];
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
                    format!("{:.3}", gs.p95),
                    format!("{:.3}", gs.p05),
                    gs.std_diff_from_control
                        .map(|d| format!("{:.3}", d))
                        .unwrap_or("-".to_string()),
                    gs.p_value
                        .map(|p| format!("{:.4}", p))
                        .unwrap_or("-".to_string()),
                ];

                // Center text vertically in row (raised higher)
                let text_y = start_y + row_idx * row_height + row_height / 2 - 2;
                let mut x_offset = start_x;
                for (i, value) in row_data.iter().enumerate() {
                    // P-value and group name use text_color, others use black
                    let color = if i == 0 || i == 8 { text_color } else { BLACK };

                    // Center horizontally in cell
                    let text_width_approx = value.len() as i32 * 7;
                    let text_x = x_offset + (col_widths[i] - text_width_approx) / 2;

                    area.draw(&Text::new(
                        value.clone(),
                        (text_x.max(x_offset + 4), text_y),
                        ("sans-serif", font_size).into_font().color(&color),
                    ))?;
                    x_offset += col_widths[i];
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
