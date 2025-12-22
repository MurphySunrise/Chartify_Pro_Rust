//! Control Panel Widget
//! Left side panel with all input controls and settings.

use crate::data::DataMode;
use egui::{Color32, ComboBox, RichText, ScrollArea};
use std::path::PathBuf;

/// User settings for analysis
#[derive(Default, Clone)]
pub struct UserSettings {
    pub csv_path: Option<PathBuf>,
    pub mode: DataMode,
    pub group_col: String,
    pub control_group: String,
    pub data_type_col: String,
    pub value_col: String,
    #[allow(dead_code)]
    pub data_cols: Vec<String>,
}

/// Left side control panel with file selection and processing controls.
pub struct ControlPanel {
    pub settings: UserSettings,
    pub columns: Vec<String>,
    pub groups: Vec<String>,
    pub selected_data_cols: Vec<bool>,
    pub progress: f32,
    pub status: String,
    pub calculate_enabled: bool,
}

impl Default for ControlPanel {
    fn default() -> Self {
        Self {
            settings: UserSettings::default(),
            columns: Vec::new(),
            groups: Vec::new(),
            selected_data_cols: Vec::new(),
            progress: 0.0,
            status: "Ready".to_string(),
            calculate_enabled: false,
        }
    }
}

impl ControlPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update available columns after CSV load
    pub fn update_columns(&mut self, columns: Vec<String>) {
        self.columns = columns.clone();
        self.selected_data_cols = vec![false; columns.len()];
        self.calculate_enabled = !columns.is_empty();
    }

    /// Update available groups
    pub fn update_groups(&mut self, groups: Vec<String>) {
        self.groups = groups;
        if !self.groups.is_empty() && self.settings.control_group.is_empty() {
            self.settings.control_group = self.groups[0].clone();
        }
    }

    /// Get selected data columns for multi mode
    pub fn get_selected_data_cols(&self) -> Vec<String> {
        self.columns
            .iter()
            .zip(self.selected_data_cols.iter())
            .filter(|(_, &selected)| selected)
            .map(|(col, _)| col.clone())
            .collect()
    }

    /// Draw the control panel
    pub fn show(&mut self, ui: &mut egui::Ui) -> ControlPanelAction {
        let mut action = ControlPanelAction::None;

        // Title
        ui.vertical_centered(|ui| {
            ui.add_space(5.0);
            ui.label(
                RichText::new("📊 Chartify Pro")
                    .size(22.0)
                    .color(Color32::from_rgb(100, 149, 237)),
            );
            ui.label(
                RichText::new("Rust Edition")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
        });
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(5.0);

        // ===== CSV File Section =====
        ui.label(RichText::new("📁 Data Source").size(14.0).strong());
        ui.add_space(5.0);

        egui::Frame::none()
            .fill(ui.visuals().widgets.noninteractive.bg_fill)
            .rounding(5.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let path_text = self
                        .settings
                        .csv_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "No file selected".to_string());

                    ui.label(RichText::new(&path_text).size(12.0).color(
                        if self.settings.csv_path.is_some() {
                            Color32::WHITE
                        } else {
                            Color32::GRAY
                        },
                    ));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("📂 Browse").clicked() {
                            action = ControlPanelAction::BrowseCsv;
                        }
                    });
                });
            });

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);

        // ===== Data Mode Section =====
        ui.label(RichText::new("⚙️ Data Mode").size(14.0).strong());
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.radio_value(&mut self.settings.mode, DataMode::Single, "Single Column");
            ui.radio_value(&mut self.settings.mode, DataMode::Multi, "Multi Column");
        });

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);

        // ===== Column Configuration Section =====
        ui.label(RichText::new("🔧 Column Configuration").size(14.0).strong());
        ui.add_space(8.0);

        let label_width = 110.0;
        let combo_width = 150.0;

        // Group column - aligned
        ui.horizontal(|ui| {
            ui.add_sized([label_width, 20.0], egui::Label::new("Group Column:"));
            ComboBox::from_id_salt("group_col")
                .width(combo_width)
                .selected_text(&self.settings.group_col)
                .show_ui(ui, |ui| {
                    for col in &self.columns {
                        if ui
                            .selectable_label(self.settings.group_col == *col, col)
                            .clicked()
                        {
                            self.settings.group_col = col.clone();
                            action = ControlPanelAction::GroupColumnChanged;
                        }
                    }
                });
        });

        ui.add_space(5.0);

        // Control group - aligned
        ui.horizontal(|ui| {
            ui.add_sized([label_width, 20.0], egui::Label::new("Control Group:"));
            ComboBox::from_id_salt("control_group")
                .width(combo_width)
                .selected_text(&self.settings.control_group)
                .show_ui(ui, |ui| {
                    for group in &self.groups {
                        if ui
                            .selectable_label(self.settings.control_group == *group, group)
                            .clicked()
                        {
                            self.settings.control_group = group.clone();
                        }
                    }
                });
        });

        ui.add_space(10.0);

        // Mode-specific columns
        match self.settings.mode {
            DataMode::Single => {
                ui.horizontal(|ui| {
                    ui.add_sized([label_width, 20.0], egui::Label::new("Data Type Col:"));
                    ComboBox::from_id_salt("data_type_col")
                        .width(combo_width)
                        .selected_text(&self.settings.data_type_col)
                        .show_ui(ui, |ui| {
                            for col in &self.columns {
                                if ui
                                    .selectable_label(self.settings.data_type_col == *col, col)
                                    .clicked()
                                {
                                    self.settings.data_type_col = col.clone();
                                }
                            }
                        });
                });

                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.add_sized([label_width, 20.0], egui::Label::new("Value Column:"));
                    ComboBox::from_id_salt("value_col")
                        .width(combo_width)
                        .selected_text(&self.settings.value_col)
                        .show_ui(ui, |ui| {
                            for col in &self.columns {
                                if ui
                                    .selectable_label(self.settings.value_col == *col, col)
                                    .clicked()
                                {
                                    self.settings.value_col = col.clone();
                                }
                            }
                        });
                });
            }
            DataMode::Multi => {
                ui.label("Data Columns:");
                egui::Frame::none()
                    .fill(ui.visuals().widgets.noninteractive.bg_fill)
                    .rounding(5.0)
                    .inner_margin(5.0)
                    .show(ui, |ui| {
                        ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                            for (i, col) in self.columns.iter().enumerate() {
                                if i < self.selected_data_cols.len() {
                                    ui.checkbox(&mut self.selected_data_cols[i], col);
                                }
                            }
                        });
                    });

                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    if ui.small_button("Select All").clicked() {
                        self.selected_data_cols.iter_mut().for_each(|v| *v = true);
                    }
                    if ui.small_button("Clear All").clicked() {
                        self.selected_data_cols.iter_mut().for_each(|v| *v = false);
                    }
                });
            }
        }

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);

        // ===== Action Buttons =====
        ui.vertical_centered(|ui| {
            ui.add_enabled_ui(self.calculate_enabled, |ui| {
                let button = egui::Button::new(RichText::new("▶ Start Calculation").size(16.0))
                    .min_size(egui::vec2(200.0, 35.0));
                if ui.add(button).clicked() {
                    action = ControlPanelAction::Calculate;
                }
            });

            ui.add_space(8.0);

            // Export PPT button (enabled after calculation complete)
            let ppt_enabled = self.progress >= 100.0 && self.status.contains("Complete");
            ui.add_enabled_ui(ppt_enabled, |ui| {
                let ppt_button = egui::Button::new(RichText::new("📄 Export PPT").size(14.0))
                    .min_size(egui::vec2(150.0, 30.0));
                if ui.add(ppt_button).clicked() {
                    action = ControlPanelAction::ExportPpt;
                }
            });
        });

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);

        // ===== Progress Section =====
        ui.label(RichText::new("📊 Progress").size(14.0).strong());
        ui.add_space(5.0);

        ui.add(
            egui::ProgressBar::new(self.progress / 100.0)
                .show_percentage()
                .animate(self.progress > 0.0 && self.progress < 100.0),
        );

        ui.add_space(5.0);

        let status_color = if self.status.contains("Error") {
            Color32::from_rgb(220, 53, 69)
        } else if self.status.contains("Complete") {
            Color32::from_rgb(40, 167, 69)
        } else {
            Color32::GRAY
        };
        ui.label(RichText::new(&self.status).size(11.0).color(status_color));

        action
    }

    /// Set progress and status
    pub fn set_progress(&mut self, progress: f32, status: &str) {
        self.progress = progress;
        self.status = status.to_string();
    }
}

/// Actions triggered by control panel
#[derive(Debug, Clone, PartialEq)]
pub enum ControlPanelAction {
    None,
    BrowseCsv,
    GroupColumnChanged,
    Calculate,
    ExportPpt,
}
