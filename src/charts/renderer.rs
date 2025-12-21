//! Static Chart Renderer
//! Generates chart images exactly matching the Python design.
//!
//! Layout:
//! 1. Title: "Analysis: {data_type}" centered
//! 2. Legend: Horizontal colored boxes + group names
//! 3. Two charts side-by-side:
//!    - Left: Distribution by Group (boxplot + scatter + mean line)
//!    - Right: Normal Quantile Plot (QQ plot)
//! 4. Statistics table with borders, pink highlight for significant P-values

use crate::charts::ChartData;
use crate::stats::DataTypeStats;
use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_filled_rect_mut, draw_hollow_circle_mut, draw_hollow_rect_mut,
    draw_line_segment_mut,
};
use imageproc::rect::Rect;
use rusttype::{Font, Scale};

// Colors (RGBA)
const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
const BLUE: Rgba<u8> = Rgba([91, 155, 213, 255]); // Control
const RED: Rgba<u8> = Rgba([237, 125, 49, 255]); // Test group 1
const GREEN: Rgba<u8> = Rgba([112, 173, 71, 255]); // Test group 2
const LIGHT_BLUE: Rgba<u8> = Rgba([189, 215, 238, 255]); // Box fill
const LIGHT_RED: Rgba<u8> = Rgba([248, 203, 173, 255]); // Box fill
const LIGHT_GREEN: Rgba<u8> = Rgba([198, 224, 180, 255]); // Box fill
const PINK_BG: Rgba<u8> = Rgba([255, 199, 206, 255]); // P-value highlight
const DARK_RED: Rgba<u8> = Rgba([156, 0, 6, 255]); // P-value text
const GRAY: Rgba<u8> = Rgba([200, 200, 200, 255]); // Grid lines

// QQ Plot X-axis labels (percentages) - reduced to 5 labels for small chart width
const QQ_LABELS: [(f64, &str); 5] = [
    (0.05, "5%"),
    (0.25, "25%"),
    (0.50, "50%"),
    (0.75, "75%"),
    (0.95, "95%"),
];

pub struct StaticChartRenderer;

impl StaticChartRenderer {
    /// Generate complete chart image with boxplot, QQ plot, and statistics table
    pub fn generate_complete_chart_image(data: &ChartData, width: u32) -> RgbaImage {
        // Layout dimensions scaled for requested width
        // Python uses figsize=(12,9) at 150 DPI = 1800x1350px
        // We scale proportionally for the given width
        let scale = width as f32 / 800.0; // Base scale factor

        let title_h = (35.0 * scale) as u32;
        let legend_h = (30.0 * scale) as u32;
        let chart_h = (350.0 * scale) as u32; // Larger chart area
        let table_row_h = (24.0 * scale) as u32;
        let n_rows = data.stats.group_stats.len() + 1; // +1 for header
        let table_h = (n_rows as u32 * table_row_h) + 15;
        let total_h = title_h + legend_h + chart_h + table_h + 50;

        let mut img = ImageBuffer::from_pixel(width, total_h, WHITE);

        // Load font
        let font_data = include_bytes!("/System/Library/Fonts/Supplemental/Arial.ttf");
        let font = Font::try_from_bytes(font_data as &[u8]).expect("Failed to load Arial font");

        let groups = data.stats.get_ordered_groups();
        let (y_min, y_max) = Self::get_y_range(data);

        // Draw components
        let mut y_offset = 5u32;

        // Title
        Self::draw_title(&mut img, &font, &data.data_type, width, y_offset);
        y_offset += title_h;

        // Legend
        Self::draw_legend(
            &mut img,
            &font,
            &groups,
            &data.stats.control_group,
            width,
            y_offset,
        );
        y_offset += legend_h + 10;

        // Charts area - boxplot on left, QQ plot on right
        let chart_width = (width - 15) / 2;

        // Left: Boxplot
        Self::draw_boxplot(
            &mut img,
            &font,
            data,
            5,
            y_offset,
            chart_width,
            chart_h,
            y_min,
            y_max,
        );

        // Right: QQ Plot
        Self::draw_qq_plot(
            &mut img,
            &font,
            data,
            10 + chart_width,
            y_offset,
            chart_width,
            chart_h,
            y_min,
            y_max,
        );
        y_offset += chart_h + 15;

        // Bottom: Statistics Table
        Self::draw_table(
            &mut img,
            &font,
            &data.stats,
            10,
            y_offset,
            width - 20,
            table_row_h,
        );

        img
    }

    fn get_y_range(data: &ChartData) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for vals in data.data_by_group.values() {
            for &v in vals {
                if !v.is_nan() {
                    min = min.min(v);
                    max = max.max(v);
                }
            }
        }
        if min.is_infinite() {
            return (0.0, 100.0);
        }
        let pad = (max - min) * 0.15;
        ((min - pad).floor(), (max + pad).ceil())
    }

    fn get_colors(idx: usize, is_control: bool, filled: bool) -> Rgba<u8> {
        if is_control {
            if filled {
                LIGHT_BLUE
            } else {
                BLUE
            }
        } else {
            match idx % 2 {
                0 => {
                    if filled {
                        LIGHT_RED
                    } else {
                        RED
                    }
                }
                _ => {
                    if filled {
                        LIGHT_GREEN
                    } else {
                        GREEN
                    }
                }
            }
        }
    }

    fn normal_ppf(p: f64) -> f64 {
        if p <= 0.0 {
            return -3.5;
        }
        if p >= 1.0 {
            return 3.5;
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
        if p < p_low {
            let q = (-2.0 * p.ln()).sqrt();
            (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
                / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
        } else if p <= 1.0 - p_low {
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

    fn draw_title(img: &mut RgbaImage, font: &Font, data_type: &str, width: u32, y: u32) {
        let title = format!("Analysis: {}", data_type);
        let scale = Scale::uniform(18.0);
        let text_width = Self::measure_text(font, scale, &title);
        let x = (width as i32 - text_width) / 2;
        Self::draw_text(img, font, scale, &title, x, y as i32, BLACK);
    }

    fn draw_legend(
        img: &mut RgbaImage,
        font: &Font,
        groups: &[String],
        control: &str,
        width: u32,
        y: u32,
    ) {
        let scale = Scale::uniform(12.0);
        let box_size = 14i32;
        let spacing = 80i32;

        let total_width = groups.len() as i32 * spacing;
        let mut x = (width as i32 - total_width) / 2;

        let mut non_ctrl_idx = 0;
        for group in groups {
            let is_ctrl = group == control;
            let color = Self::get_colors(non_ctrl_idx, is_ctrl, false);
            if !is_ctrl {
                non_ctrl_idx += 1;
            }

            // Color box
            draw_filled_rect_mut(
                img,
                Rect::at(x, y as i32).of_size(box_size as u32, box_size as u32),
                color,
            );

            // Text
            Self::draw_text(
                img,
                font,
                scale,
                group,
                x + box_size + 4,
                y as i32 + 2,
                BLACK,
            );

            x += spacing;
        }
    }

    fn draw_boxplot(
        img: &mut RgbaImage,
        font: &Font,
        data: &ChartData,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        y_min: f64,
        y_max: f64,
    ) {
        let plot_x = x + 35; // Reduced from 40 to save space
        let plot_y = y + 25;
        let plot_w = w - 40; // Reduced from 50 to use more width
        let plot_h = h - 60;

        // Title
        let scale = Scale::uniform(14.0);
        Self::draw_text(
            img,
            font,
            scale,
            "Distribution by Group",
            (plot_x + plot_w / 2 - 60) as i32,
            y as i32,
            BLACK,
        );

        // Draw axes
        draw_line_segment_mut(
            img,
            (plot_x as f32, (plot_y + plot_h) as f32),
            ((plot_x + plot_w) as f32, (plot_y + plot_h) as f32),
            BLACK,
        );
        draw_line_segment_mut(
            img,
            (plot_x as f32, plot_y as f32),
            (plot_x as f32, (plot_y + plot_h) as f32),
            BLACK,
        );

        // Y-axis labels and grid
        let scale_small = Scale::uniform(10.0);
        let y_range = y_max - y_min;
        let y_step = Self::nice_step(y_range, 8); // Increased from 5 to get more ticks
        let mut y_val = (y_min / y_step).ceil() * y_step;

        while y_val <= y_max {
            let py = Self::map_y(y_val, y_min, y_max, plot_y, plot_h);
            let label = format!("{:.0}", y_val);
            Self::draw_text(
                img,
                font,
                scale_small,
                &label,
                (plot_x - 25) as i32,
                py as i32 - 5,
                BLACK,
            );
            draw_line_segment_mut(
                img,
                (plot_x as f32, py as f32),
                ((plot_x + plot_w) as f32, py as f32),
                GRAY,
            );
            y_val += y_step;
        }

        // Y-axis label "Value" - rotated 90 degrees like matplotlib
        Self::draw_text_rotated(
            img,
            font,
            scale_small,
            "Value",
            (x + 2) as i32,
            (plot_y + plot_h / 2 + 15) as i32, // Centered on Y-axis
            BLACK,
        );

        let groups = data.stats.get_ordered_groups();
        let n = groups.len();
        let group_width = plot_w / n as u32;
        let box_width = (group_width as f32 * 0.5) as u32;

        let mut means: Vec<(u32, u32)> = Vec::new();
        let mut non_ctrl_idx = 0;

        for (i, group) in groups.iter().enumerate() {
            let vals = match data.data_by_group.get(group) {
                Some(v) => v,
                None => continue,
            };
            if vals.is_empty() {
                continue;
            }

            let is_ctrl = group == &data.stats.control_group;
            let fill_color = Self::get_colors(non_ctrl_idx, is_ctrl, true);
            let line_color = Self::get_colors(non_ctrl_idx, is_ctrl, false);
            if !is_ctrl {
                non_ctrl_idx += 1;
            }

            let cx = plot_x + i as u32 * group_width + group_width / 2;

            // Stats
            let mut sorted = vals.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let len = sorted.len();
            let q1 = sorted[len / 4];
            let q3 = sorted[3 * len / 4];
            let med = sorted[len / 2];
            let iqr = q3 - q1;
            let low = *sorted.iter().find(|&&v| v >= q1 - 1.5 * iqr).unwrap_or(&q1);
            let high = *sorted
                .iter()
                .rev()
                .find(|&&v| v <= q3 + 1.5 * iqr)
                .unwrap_or(&q3);
            let mean: f64 = sorted.iter().sum::<f64>() / len as f64;

            let y_q1 = Self::map_y(q1, y_min, y_max, plot_y, plot_h);
            let y_q3 = Self::map_y(q3, y_min, y_max, plot_y, plot_h);
            let y_med = Self::map_y(med, y_min, y_max, plot_y, plot_h);
            let y_low = Self::map_y(low, y_min, y_max, plot_y, plot_h);
            let y_high = Self::map_y(high, y_min, y_max, plot_y, plot_h);
            let y_mean = Self::map_y(mean, y_min, y_max, plot_y, plot_h);

            means.push((cx, y_mean));

            // Box (filled)
            let box_left = cx - box_width / 2;
            let box_h = if y_q1 > y_q3 {
                y_q1 - y_q3
            } else {
                y_q3 - y_q1
            };
            let box_top = y_q3.min(y_q1);
            draw_filled_rect_mut(
                img,
                Rect::at(box_left as i32, box_top as i32).of_size(box_width, box_h.max(1)),
                fill_color,
            );
            draw_hollow_rect_mut(
                img,
                Rect::at(box_left as i32, box_top as i32).of_size(box_width, box_h.max(1)),
                line_color,
            );

            // Median line
            draw_line_segment_mut(
                img,
                (box_left as f32, y_med as f32),
                ((box_left + box_width) as f32, y_med as f32),
                line_color,
            );

            // Whiskers
            draw_line_segment_mut(
                img,
                (cx as f32, y_low as f32),
                (cx as f32, y_q1 as f32),
                line_color,
            );
            draw_line_segment_mut(
                img,
                (cx as f32, y_q3 as f32),
                (cx as f32, y_high as f32),
                line_color,
            );
            // Caps
            let cap_w = box_width / 3;
            draw_line_segment_mut(
                img,
                ((cx - cap_w) as f32, y_low as f32),
                ((cx + cap_w) as f32, y_low as f32),
                line_color,
            );
            draw_line_segment_mut(
                img,
                ((cx - cap_w) as f32, y_high as f32),
                ((cx + cap_w) as f32, y_high as f32),
                line_color,
            );

            // Scatter points
            for &v in vals {
                let py = Self::map_y(v, y_min, y_max, plot_y, plot_h);
                draw_filled_circle_mut(img, (cx as i32, py as i32), 2, line_color);
            }

            // X-axis label (group name)
            Self::draw_text(
                img,
                font,
                scale_small,
                group,
                (cx - 15) as i32,
                (plot_y + plot_h + 8) as i32,
                BLACK,
            );
        }

        // Mean line (black)
        for i in 0..means.len().saturating_sub(1) {
            draw_line_segment_mut(
                img,
                (means[i].0 as f32, means[i].1 as f32),
                (means[i + 1].0 as f32, means[i + 1].1 as f32),
                BLACK,
            );
        }
        // Mean dots
        for (mx, my) in &means {
            draw_filled_circle_mut(img, (*mx as i32, *my as i32), 4, BLACK);
        }

        // X-axis label "Group"
        Self::draw_text(
            img,
            font,
            scale_small,
            "Group",
            (plot_x + plot_w / 2 - 15) as i32,
            (plot_y + plot_h + 22) as i32,
            BLACK,
        );
    }

    fn draw_qq_plot(
        img: &mut RgbaImage,
        font: &Font,
        data: &ChartData,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        y_min: f64,
        y_max: f64,
    ) {
        let plot_x = x + 35; // Reduced from 40 to save space
        let plot_y = y + 25;
        let plot_w = w - 40; // Reduced from 50 to use more width
        let plot_h = h - 60;

        // Title
        let scale = Scale::uniform(14.0);
        Self::draw_text(
            img,
            font,
            scale,
            "Normal Quantile Plot",
            (plot_x + plot_w / 2 - 60) as i32,
            y as i32,
            BLACK,
        );

        // X range
        let x_min = Self::normal_ppf(0.005);
        let x_max = Self::normal_ppf(0.995);

        // Draw axes
        draw_line_segment_mut(
            img,
            (plot_x as f32, (plot_y + plot_h) as f32),
            ((plot_x + plot_w) as f32, (plot_y + plot_h) as f32),
            BLACK,
        );
        draw_line_segment_mut(
            img,
            (plot_x as f32, plot_y as f32),
            (plot_x as f32, (plot_y + plot_h) as f32),
            BLACK,
        );

        // Y-axis labels
        let scale_small = Scale::uniform(10.0);
        let y_range = y_max - y_min;
        let y_step = Self::nice_step(y_range, 5);
        let mut y_val = (y_min / y_step).ceil() * y_step;

        while y_val <= y_max {
            let py = Self::map_y(y_val, y_min, y_max, plot_y, plot_h);
            let label = format!("{:.0}", y_val);
            Self::draw_text(
                img,
                font,
                scale_small,
                &label,
                (plot_x - 25) as i32,
                py as i32 - 5,
                BLACK,
            );
            draw_line_segment_mut(
                img,
                (plot_x as f32, py as f32),
                ((plot_x + plot_w) as f32, py as f32),
                GRAY,
            );
            y_val += y_step;
        }

        // Y-axis label
        Self::draw_text(
            img,
            font,
            scale_small,
            "Value",
            (x + 5) as i32,
            (plot_y + plot_h / 2) as i32,
            BLACK,
        );

        // X-axis labels (percentages) - center each label under tick mark
        let label_scale = Scale::uniform(9.0); // Smaller font for X labels
        for (p, label) in QQ_LABELS {
            let t = Self::normal_ppf(p);
            let px = Self::map_x(t, x_min, x_max, plot_x, plot_w);
            let label_w = Self::measure_text(font, label_scale, label);
            Self::draw_text(
                img,
                font,
                label_scale,
                label,
                px as i32 - label_w / 2,
                (plot_y + plot_h + 5) as i32,
                BLACK,
            );
        }

        // X-axis label
        Self::draw_text(
            img,
            font,
            scale_small,
            "Normal Quantile",
            (plot_x + plot_w / 2 - 40) as i32,
            (plot_y + plot_h + 22) as i32,
            BLACK,
        );

        // Draw data
        let groups = data.stats.get_ordered_groups();
        let mut non_ctrl_idx = 0;

        for group in groups {
            let vals = match data.data_by_group.get(&group) {
                Some(v) => v,
                None => continue,
            };
            if vals.is_empty() {
                continue;
            }

            let is_ctrl = group == data.stats.control_group;
            let color = Self::get_colors(non_ctrl_idx, is_ctrl, false);
            if !is_ctrl {
                non_ctrl_idx += 1;
            }

            let mut sorted = vals.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = sorted.len();

            let mut points: Vec<(u32, u32)> = Vec::new();
            for (i, &v) in sorted.iter().enumerate() {
                let p = (i as f64 + 0.5) / n as f64;
                let t = Self::normal_ppf(p);
                let px = Self::map_x(t, x_min, x_max, plot_x, plot_w);
                let py = Self::map_y(v, y_min, y_max, plot_y, plot_h);
                points.push((px, py));
            }

            // Draw lines
            for i in 0..points.len().saturating_sub(1) {
                draw_line_segment_mut(
                    img,
                    (points[i].0 as f32, points[i].1 as f32),
                    (points[i + 1].0 as f32, points[i + 1].1 as f32),
                    color,
                );
            }
            // Draw points
            for (px, py) in &points {
                draw_filled_circle_mut(img, (*px as i32, *py as i32), 3, color);
            }
        }
    }

    fn draw_table(
        img: &mut RgbaImage,
        font: &Font,
        stats: &DataTypeStats,
        x: u32,
        y: u32,
        w: u32,
        row_h: u32,
    ) {
        let cols = [
            "Group",
            "Count",
            "Mean",
            "Median",
            "Std",
            "P95",
            "P05",
            "(Mean-Ctrl)/σ",
            "P-value",
        ];
        let col_pcts = [0.12, 0.08, 0.10, 0.10, 0.10, 0.10, 0.10, 0.14, 0.16];

        let scale = Scale::uniform(10.0);
        let groups = stats.get_ordered_groups();

        // Header
        let mut cx = x;
        let mut cy = y;

        for (i, col) in cols.iter().enumerate() {
            let cw = (w as f64 * col_pcts[i]) as u32;
            draw_hollow_rect_mut(
                img,
                Rect::at(cx as i32, cy as i32).of_size(cw, row_h),
                BLACK,
            );
            Self::draw_text(
                img,
                font,
                scale,
                col,
                (cx + 4) as i32,
                (cy + 6) as i32,
                BLACK,
            );
            cx += cw;
        }
        cy += row_h;

        // Data rows
        for group in groups {
            let gs = match stats.group_stats.get(&group) {
                Some(g) => g,
                None => continue,
            };

            let is_sig = gs.is_significant;
            let is_ctrl = group == stats.control_group;

            let vals = [
                group.clone(),
                format!("{}", gs.count),
                format!("{:.3}", gs.mean),
                format!("{:.3}", gs.median),
                format!("{:.3}", gs.std),
                format!("{:.3}", gs.p95),
                format!("{:.3}", gs.p05),
                gs.std_diff_from_control
                    .map(|v| format!("{:.3}", v))
                    .unwrap_or("-".into()),
                gs.p_value
                    .map(|v| format!("{:.4}", v))
                    .unwrap_or("-".into()),
            ];

            let mut cx = x;
            for (i, val) in vals.iter().enumerate() {
                let cw = (w as f64 * col_pcts[i]) as u32;
                let is_pval = i == 8;

                // Background
                if is_sig && is_pval && !is_ctrl {
                    draw_filled_rect_mut(
                        img,
                        Rect::at(cx as i32, cy as i32).of_size(cw, row_h),
                        PINK_BG,
                    );
                }

                // Border
                draw_hollow_rect_mut(
                    img,
                    Rect::at(cx as i32, cy as i32).of_size(cw, row_h),
                    BLACK,
                );

                // Text
                let text_color = if is_sig && is_pval && !is_ctrl {
                    DARK_RED
                } else {
                    BLACK
                };
                Self::draw_text(
                    img,
                    font,
                    scale,
                    val,
                    (cx + 4) as i32,
                    (cy + 6) as i32,
                    text_color,
                );

                cx += cw;
            }
            cy += row_h;
        }
    }

    // Helper functions
    fn map_y(val: f64, y_min: f64, y_max: f64, plot_y: u32, plot_h: u32) -> u32 {
        let ratio = (val - y_min) / (y_max - y_min);
        plot_y + plot_h - (ratio * plot_h as f64) as u32
    }

    fn map_x(val: f64, x_min: f64, x_max: f64, plot_x: u32, plot_w: u32) -> u32 {
        let ratio = (val - x_min) / (x_max - x_min);
        plot_x + (ratio * plot_w as f64) as u32
    }

    fn nice_step(range: f64, target_steps: usize) -> f64 {
        let raw_step = range / target_steps as f64;
        let magnitude = 10f64.powf(raw_step.log10().floor());
        let normalized = raw_step / magnitude;

        let nice = if normalized <= 1.0 {
            1.0
        } else if normalized <= 2.0 {
            2.0
        } else if normalized <= 5.0 {
            5.0
        } else {
            10.0
        };

        nice * magnitude
    }

    fn measure_text(font: &Font, scale: Scale, text: &str) -> i32 {
        let v_metrics = font.v_metrics(scale);
        let glyphs: Vec<_> = font
            .layout(text, scale, rusttype::point(0.0, v_metrics.ascent))
            .collect();
        if let Some(last) = glyphs.last() {
            if let Some(bb) = last.pixel_bounding_box() {
                return bb.max.x;
            }
        }
        (text.len() * 6) as i32
    }

    fn draw_text(
        img: &mut RgbaImage,
        font: &Font,
        scale: Scale,
        text: &str,
        x: i32,
        y: i32,
        color: Rgba<u8>,
    ) {
        let v_metrics = font.v_metrics(scale);
        for glyph in font.layout(
            text,
            scale,
            rusttype::point(x as f32, y as f32 + v_metrics.ascent),
        ) {
            if let Some(bb) = glyph.pixel_bounding_box() {
                glyph.draw(|gx, gy, v| {
                    let px = (bb.min.x + gx as i32) as u32;
                    let py = (bb.min.y + gy as i32) as u32;
                    if px < img.width() && py < img.height() {
                        let alpha = (v * 255.0) as u8;
                        if alpha > 0 {
                            let pixel = img.get_pixel_mut(px, py);
                            // Simple alpha blend
                            let bg = *pixel;
                            pixel[0] = ((color[0] as u16 * alpha as u16
                                + bg[0] as u16 * (255 - alpha) as u16)
                                / 255) as u8;
                            pixel[1] = ((color[1] as u16 * alpha as u16
                                + bg[1] as u16 * (255 - alpha) as u16)
                                / 255) as u8;
                            pixel[2] = ((color[2] as u16 * alpha as u16
                                + bg[2] as u16 * (255 - alpha) as u16)
                                / 255) as u8;
                        }
                    }
                });
            }
        }
    }

    /// Draw text rotated 90 degrees counter-clockwise (for Y-axis labels)
    fn draw_text_rotated(
        img: &mut RgbaImage,
        font: &Font,
        scale: Scale,
        text: &str,
        x: i32,
        y: i32,
        color: Rgba<u8>,
    ) {
        // First, measure the text to create a temporary image
        let v_metrics = font.v_metrics(scale);
        let glyphs: Vec<_> = font
            .layout(text, scale, rusttype::point(0.0, v_metrics.ascent))
            .collect();

        if glyphs.is_empty() {
            return;
        }

        // Calculate text bounds
        let mut max_x = 0;
        let mut max_y = 0;
        for glyph in &glyphs {
            if let Some(bb) = glyph.pixel_bounding_box() {
                max_x = max_x.max(bb.max.x as u32);
                max_y = max_y.max(bb.max.y as u32);
            }
        }

        if max_x == 0 || max_y == 0 {
            return;
        }

        // Create a temporary image for the text (with some padding)
        let temp_w = max_x + 5;
        let temp_h = max_y + 5;
        let mut temp_img: RgbaImage =
            ImageBuffer::from_pixel(temp_w, temp_h, Rgba([255, 255, 255, 0]));

        // Draw text to temporary image
        for glyph in &glyphs {
            if let Some(bb) = glyph.pixel_bounding_box() {
                glyph.draw(|gx, gy, v| {
                    let px = (bb.min.x + gx as i32) as u32;
                    let py = (bb.min.y + gy as i32) as u32;
                    if px < temp_w && py < temp_h {
                        let alpha = (v * 255.0) as u8;
                        if alpha > 0 {
                            temp_img.put_pixel(px, py, Rgba([color[0], color[1], color[2], alpha]));
                        }
                    }
                });
            }
        }

        // Rotate 90 degrees counter-clockwise: (x, y) -> (y, width - x - 1)
        // New dimensions: width becomes height, height becomes width
        let rotated_w = temp_h;
        let rotated_h = temp_w;

        // Copy rotated pixels to main image
        for ty in 0..temp_h {
            for tx in 0..temp_w {
                let pixel = temp_img.get_pixel(tx, ty);
                if pixel[3] > 0 {
                    // Rotate: new_x = ty, new_y = temp_w - tx - 1
                    let new_x = ty as i32;
                    let new_y = (temp_w - tx - 1) as i32;

                    let dest_x = x + new_x;
                    let dest_y = y + new_y;

                    if dest_x >= 0
                        && dest_y >= 0
                        && (dest_x as u32) < img.width()
                        && (dest_y as u32) < img.height()
                    {
                        let dest_pixel = img.get_pixel_mut(dest_x as u32, dest_y as u32);
                        let alpha = pixel[3] as u16;
                        let bg = *dest_pixel;
                        dest_pixel[0] =
                            ((pixel[0] as u16 * alpha + bg[0] as u16 * (255 - alpha)) / 255) as u8;
                        dest_pixel[1] =
                            ((pixel[1] as u16 * alpha + bg[1] as u16 * (255 - alpha)) / 255) as u8;
                        dest_pixel[2] =
                            ((pixel[2] as u16 * alpha + bg[2] as u16 * (255 - alpha)) / 255) as u8;
                    }
                }
            }
        }
    }
}
