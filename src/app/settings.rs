use std::fmt::Write;
use std::path::PathBuf;

use eframe::egui::*;
use egui_extras::{Column, Table, TableBuilder, TableRow};
use rfd::FileDialog;
use serde::*;
use serde_json::*;

use crate::analyzer::settings::{
    self, AnalysisSettings, MatchAspect, MatchMethod, MatchRule, Rule,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Settings {
    pub analysis: AnalysisSettings,
    pub auto_refresh: AutoRefresh,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutoRefresh {
    pub enable: bool,
    pub interval_seconds: f64,
}

#[derive(Default)]
pub struct SettingsWindow {
    is_open: bool,
    modified_settings: Settings,
    result: SettingsResult,
    selected_tab: SettingsTab,
    combat_separation_time: String,
    auto_refresh_interval: String,
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
            self.update_slider_displays();
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
                            if self.modified_settings != *settings {
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

        ui.separator();

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

        ui.separator();

        ui.checkbox(
            &mut self.modified_settings.auto_refresh.enable,
            "auto refresh when log changes",
        );
        ui.label("auto refresh interval in seconds");
        ui.horizontal(|ui| {
            if Slider::new(
                &mut self.modified_settings.auto_refresh.interval_seconds,
                1.0..=10.0,
            )
            .clamp_to_range(false)
            .show_value(false)
            .step_by(1.0)
            .ui(ui)
            .changed()
            {
                self.update_auto_refresh_interval_display();
            }

            if TextEdit::singleline(&mut self.auto_refresh_interval)
                .desired_width(40.0)
                .show(ui)
                .response
                .changed()
            {
                if let Ok(auto_refresh_interval) = self.auto_refresh_interval.parse::<f64>() {
                    self.modified_settings.auto_refresh.interval_seconds =
                        auto_refresh_interval.max(0.0);
                }
            }
        });

        ui.add_space(100.0);
    }

    fn show_analysis_tab(&mut self, ui: &mut Ui) {
        self.show_sub_source_grouping_reversal_rules(ui);
        ui.add_space(20.0);

        ui.separator();
        ui.push_id(line!(), |ui| {
            self.show_grouping_rules(ui);
        });
        ui.add_space(20.0);

        ui.separator();
        self.show_combat_name_rules(ui);
        ui.add_space(20.0);
    }

    fn show_sub_source_grouping_reversal_rules(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("sub source (e.g. pets or summons) grouping reversal rules");
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

                        Self::show_match_rule(
                            &mut r,
                            rule,
                            id + line!() as usize,
                            600.0,
                            [
                                MatchAspect::DamageName,
                                MatchAspect::SubSourceName,
                                MatchAspect::SubUniqueSourceName,
                            ],
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
    }

    fn show_grouping_rules(&mut self, ui: &mut Ui) {
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
                            [
                                MatchAspect::DamageName,
                                MatchAspect::SubSourceName,
                                MatchAspect::SubUniqueSourceName,
                            ],
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
    }

    fn show_combat_name_rules(&mut self, ui: &mut Ui) {
        CollapsingHeader::new("combat name detection rules").show_unindented(ui, |ui| {
            if ui.button("✚").clicked() {
                self.modified_settings
                    .analysis
                    .combat_name_rules
                    .push(Default::default());
            }

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
                        ui.label("combat name");
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
                        .combat_name_rules
                        .iter_mut()
                        .enumerate()
                    {
                        t.row(25.0, |mut r| {
                            r.col(|ui| {
                                ui.checkbox(&mut rule.enabled, "");
                            });

                            r.col(|ui| {
                                TextEdit::singleline(&mut rule.combat_name)
                                    .desired_width(800.0)
                                    .show(ui);
                            });

                            Self::show_match_rule(
                                &mut r,
                                &mut rule.match_rule,
                                id + line!() as usize,
                                400.0,
                                [
                                    MatchAspect::DamageName,
                                    MatchAspect::SubSourceName,
                                    MatchAspect::SubUniqueSourceName,
                                    MatchAspect::SourceOrTargetName,
                                    MatchAspect::SourceOrTargetUniqueName,
                                ],
                            );

                            r.col(|ui| {
                                if ui.selectable_label(false, "🗑").clicked() {
                                    to_remove.push(id);
                                }
                            });
                        });
                    }

                    to_remove.into_iter().rev().for_each(|i| {
                        self.modified_settings.analysis.combat_name_rules.remove(i);
                    });
                });
        });
    }

    fn show_match_rule(
        row: &mut TableRow,
        rule: &mut MatchRule,
        id: usize,
        desired_expression_width: f32,
        aspect_set: impl IntoIterator<Item = MatchAspect>,
    ) {
        row.col(|ui| {
            ComboBox::from_id_source(id + 9387465)
                .selected_text(rule.aspect.display())
                .width(150.0)
                .show_ui(ui, |ui| {
                    aspect_set.into_iter().for_each(|a| {
                        ui.selectable_value(&mut rule.aspect, a, a.display());
                    });
                });
        });

        row.col(|ui| {
            ComboBox::from_id_source(id + 394857)
                .selected_text(rule.method.display())
                .width(150.0)
                .show_ui(ui, |ui| {
                    [
                        MatchMethod::Equals,
                        MatchMethod::StartsWith,
                        MatchMethod::EndsWith,
                        MatchMethod::Contains,
                    ]
                    .into_iter()
                    .for_each(|m| {
                        ui.selectable_value(&mut rule.method, m, m.display());
                    });
                });
        });

        row.col(|ui| {
            TextEdit::singleline(&mut rule.expression)
                .desired_width(desired_expression_width)
                .show(ui);
        });
    }

    fn update_slider_displays(&mut self) {
        self.update_combat_separation_time_display();
        self.update_auto_refresh_interval_display();
    }

    fn update_combat_separation_time_display(&mut self) {
        Self::update_slider_display(
            &mut self.combat_separation_time,
            self.modified_settings
                .analysis
                .combat_separation_time_seconds,
        );
    }

    fn update_auto_refresh_interval_display(&mut self) {
        Self::update_slider_display(
            &mut self.auto_refresh_interval,
            self.modified_settings.auto_refresh.interval_seconds,
        );
    }

    fn update_slider_display(display: &mut String, value: f64) {
        display.clear();
        write!(display, "{}", value).unwrap();
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
        let data = match serde_json::to_string_pretty(self) {
            Ok(d) => d,
            Err(_) => {
                return;
            }
        };

        let _ = std::fs::write(&file_path, data);
    }
}

impl AutoRefresh {
    pub fn interval_seconds(&self) -> Option<f64> {
        if self.enable {
            Some(self.interval_seconds)
        } else {
            None
        }
    }
}

impl Default for AutoRefresh {
    fn default() -> Self {
        Self {
            enable: false,
            interval_seconds: 4.0,
        }
    }
}
