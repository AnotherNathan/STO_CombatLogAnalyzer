use std::fmt::Write;
use std::path::PathBuf;

use eframe::egui::*;
use egui_extras::{Column, Table, TableBuilder, TableRow};
use rfd::FileDialog;
use serde::*;
use serde_json::*;

use crate::analyzer::settings::{
    self, AnalysisSettings, CustomGroupingRule, MatchAspect, MatchMethod, MatchRule,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Settings {
    pub analysis: AnalysisSettings,
}

#[derive(Default)]
pub struct SettingsWindow {
    is_open: bool,
    modified_settings: Settings,
    result: SettingsResult,
    selected_tab: SettingsTab,
    combat_separation_time: String,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    #[default]
    File,
    Analysis,
}

#[derive(Default, Clone, Copy)]
pub enum SettingsResult {
    #[default]
    NoChanges,
    ReloadLog,
}

impl SettingsWindow {
    pub fn show(&mut self, ctx: &Context, ui: &mut Ui, settings: &mut Settings) -> SettingsResult {
        self.result = SettingsResult::NoChanges;
        if ui.selectable_label(self.is_open, "Settings").clicked() && !self.is_open {
            self.is_open = true;
            self.modified_settings = settings.clone();
            self.update_combat_separation_time_display();
        }

        if self.is_open {
            Window::new("Settings")
                .collapsible(false)
                .default_size([800.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(self.selected_tab == SettingsTab::File, "file")
                            .clicked()
                        {
                            self.selected_tab = SettingsTab::File;
                        }

                        if ui
                            .selectable_label(
                                self.selected_tab == SettingsTab::Analysis,
                                "analysis",
                            )
                            .clicked()
                        {
                            self.selected_tab = SettingsTab::Analysis;
                        }
                    });

                    ui.separator();

                    match self.selected_tab {
                        SettingsTab::File => self.show_file_tab(ui),
                        SettingsTab::Analysis => self.show_analysis_tab(ui),
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Ok").clicked() {
                            if self.modified_settings.analysis != settings.analysis {
                                self.modified_settings.save();
                                self.result = SettingsResult::ReloadLog;
                            }

                            self.is_open = false;
                            *settings = self.modified_settings.clone();
                        }

                        if ui.button("Cancel").clicked() {
                            self.is_open = false;
                        }
                    })
                });
        }
        self.result
    }

    fn show_file_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("combatlog file");
            if ui.button("browse").clicked() {
                // TODO find out how to set the parent
                if let Some(new_combatlog_file) = FileDialog::new()
                    .add_filter("combatlog", &["log"])
                    .pick_file()
                {
                    self.modified_settings.analysis.combatlog_file =
                        new_combatlog_file.display().to_string();
                }
            }
        });
        TextEdit::singleline(&mut self.modified_settings.analysis.combatlog_file)
            .desired_width(f32::MAX)
            .show(ui);

        ui.label("combat separation time in seconds");
        ui.horizontal(|ui| {
            if Slider::new(
                &mut self
                    .modified_settings
                    .analysis
                    .combat_separation_time_seconds,
                15.0..=240.0,
            )
            .clamp_to_range(false)
            .show_value(false)
            .step_by(15.0)
            .ui(ui)
            .changed()
            {
                self.update_combat_separation_time_display();
            }

            if TextEdit::singleline(&mut self.combat_separation_time)
                .desired_width(40.0)
                .show(ui)
                .response
                .changed()
            {
                if let Ok(combat_separation_time) = self.combat_separation_time.parse::<f64>() {
                    self.modified_settings
                        .analysis
                        .combat_separation_time_seconds = combat_separation_time.max(0.0);
                }
            }
        });

        ui.add_space(100.0);
    }

    fn show_analysis_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("summon or pet grouping reversal rules");
            if ui.button("✚").clicked() {
                self.modified_settings
                    .analysis
                    .summon_and_pet_grouping_revers_rules
                    .push(Default::default());
            }
        });

        TableBuilder::new(ui)
            .striped(true)
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto().at_least(400.0).resizable(true))
            .column(Column::auto())
            .cell_layout(Layout::left_to_right(Align::Center))
            .max_scroll_height(200.0)
            .header(0.0, |mut r| {
                r.col(|ui| {
                    ui.label("on");
                });
                r.col(|ui| {
                    ui.label("aspect to match");
                });
                r.col(|ui| {
                    ui.label("match method");
                });
                r.col(|ui| {
                    ui.label("text to match");
                });
            })
            .body(|mut t| {
                let mut to_remove = Vec::new();
                for (id, rule) in self
                    .modified_settings
                    .analysis
                    .summon_and_pet_grouping_revers_rules
                    .iter_mut()
                    .enumerate()
                {
                    t.row(25.0, |mut r| {
                        r.col(|ui| {
                            ui.checkbox(&mut rule.enabled, "");
                        });

                        Self::show_match_rule(&mut r, rule, id + line!() as usize, 600.0);

                        r.col(|ui| {
                            if ui.selectable_label(false, "🗑").clicked() {
                                to_remove.push(id);
                            }
                        });
                    });
                }

                to_remove.into_iter().rev().for_each(|i| {
                    self.modified_settings.analysis.custom_group_rules.remove(i);
                });
            });

        ui.add_space(20.0);

        ui.separator();

        ui.push_id(line!(), |ui| {
            ui.horizontal(|ui| {
                ui.label("custom grouping rules");
                if ui.button("✚").clicked() {
                    self.modified_settings
                        .analysis
                        .custom_group_rules
                        .push(Default::default());
                }
            });

            TableBuilder::new(ui)
                .column(Column::auto())
                .column(Column::auto().at_least(200.0).resizable(true))
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto().at_least(200.0).resizable(true))
                .column(Column::auto())
                .cell_layout(Layout::left_to_right(Align::Center))
                .max_scroll_height(200.0)
                .header(0.0, |mut r| {
                    r.col(|ui| {
                        ui.label("on");
                    });
                    r.col(|ui| {
                        ui.label("group name");
                    });
                    r.col(|ui| {
                        ui.label("aspect to match");
                    });
                    r.col(|ui| {
                        ui.label("match method");
                    });
                    r.col(|ui| {
                        ui.label("text to match");
                    });
                })
                .body(|mut t| {
                    let mut to_remove = Vec::new();
                    for (id, rule) in self
                        .modified_settings
                        .analysis
                        .custom_group_rules
                        .iter_mut()
                        .enumerate()
                    {
                        t.row(25.0, |mut r| {
                            r.col(|ui| {
                                ui.checkbox(&mut rule.enabled, "");
                            });

                            r.col(|ui| {
                                TextEdit::singleline(&mut rule.group_name)
                                    .desired_width(800.0)
                                    .show(ui);
                            });

                            Self::show_match_rule(
                                &mut r,
                                &mut rule.match_rule,
                                id + line!() as usize,
                                400.0,
                            );

                            r.col(|ui| {
                                if ui.selectable_label(false, "🗑").clicked() {
                                    to_remove.push(id);
                                }
                            });
                        });
                    }

                    to_remove.into_iter().rev().for_each(|i| {
                        self.modified_settings.analysis.custom_group_rules.remove(i);
                    });
                });

            ui.add_space(20.0);
        });
    }

    fn show_match_rule(
        row: &mut TableRow,
        rule: &mut MatchRule,
        id: usize,
        desired_expression_width: f32,
    ) {
        row.col(|ui| {
            ComboBox::from_id_source(id + 9387465)
                .selected_text(rule.aspect.display())
                .width(150.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut rule.aspect,
                        MatchAspect::PetOrSummonName,
                        MatchAspect::PetOrSummonName.display(),
                    );
                    ui.selectable_value(
                        &mut rule.aspect,
                        MatchAspect::DamageName,
                        MatchAspect::DamageName.display(),
                    );
                });
        });

        row.col(|ui| {
            ComboBox::from_id_source(id + 394857)
                .selected_text(rule.method.display())
                .width(150.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut rule.method,
                        MatchMethod::Equals,
                        MatchMethod::Equals.display(),
                    );
                    ui.selectable_value(
                        &mut rule.method,
                        MatchMethod::StartsWith,
                        MatchMethod::StartsWith.display(),
                    );
                    ui.selectable_value(
                        &mut rule.method,
                        MatchMethod::EndsWith,
                        MatchMethod::EndsWith.display(),
                    );
                    ui.selectable_value(
                        &mut rule.method,
                        MatchMethod::Contains,
                        MatchMethod::Contains.display(),
                    );
                });
        });

        row.col(|ui| {
            TextEdit::singleline(&mut rule.expression)
                .desired_width(desired_expression_width)
                .show(ui);
        });
    }

    fn update_combat_separation_time_display(&mut self) {
        self.combat_separation_time.clear();
        write!(
            &mut self.combat_separation_time,
            "{}",
            self.modified_settings
                .analysis
                .combat_separation_time_seconds
        )
        .unwrap();
    }
}

impl Settings {
    fn file_path() -> Option<PathBuf> {
        let mut path = std::env::current_exe().ok()?;
        path.pop();
        path.push("settings.json");
        Some(path)
    }

    pub fn load_or_default() -> Self {
        Self::file_path()
            .and_then(|f| std::fs::read_to_string(&f).ok())
            .map(|d| serde_json::from_str(&d).ok())
            .flatten()
            .unwrap_or_else(|| Self::default())
    }

    fn save(&self) {
        let file_path = match Self::file_path() {
            Some(p) => p,
            None => {
                return;
            }
        };
        let data = match serde_json::to_string(self) {
            Ok(d) => d,
            Err(_) => {
                return;
            }
        };

        let _ = std::fs::write(&file_path, data);
    }
}
