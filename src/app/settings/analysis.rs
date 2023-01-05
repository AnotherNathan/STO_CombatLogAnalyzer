use eframe::egui::*;
use egui_extras::*;

use crate::{analyzer::settings::*, custom_widgets::popup_button::PopupButton};

use super::Settings;

#[derive(Default)]
pub struct AnalysisTab {}

impl AnalysisTab {
    pub fn show(&mut self, modified_settings: &mut Settings, ui: &mut Ui) {
        self.show_sub_source_grouping_reversal_rules(modified_settings, ui);
        ui.add_space(20.0);

        ui.separator();
        ui.push_id(line!(), |ui| {
            self.show_grouping_rules(modified_settings, ui);
        });
        ui.add_space(20.0);

        ui.separator();
        self.show_combat_name_rules(modified_settings, ui);
        ui.add_space(20.0);
    }

    fn show_sub_source_grouping_reversal_rules(
        &mut self,
        modified_settings: &mut Settings,
        ui: &mut Ui,
    ) {
        Self::show_rules_table(
            &mut modified_settings
                .analysis
                .summon_and_pet_grouping_revers_rules,
            "Sub-Source (e.g. pets or summons) Grouping Reversal rules",
            ui,
            39048765,
            [
                MatchAspect::DamageName,
                MatchAspect::SubSourceName,
                MatchAspect::SubUniqueSourceName,
            ],
        );
    }

    fn show_grouping_rules(&mut self, modified_settings: &mut Settings, ui: &mut Ui) {
        Self::show_group_rules_table(
            &mut modified_settings.analysis.custom_group_rules,
            "Custom Grouping rules",
            "Group Name",
            ui,
            293874,
            [
                MatchAspect::DamageName,
                MatchAspect::SubSourceName,
                MatchAspect::SubUniqueSourceName,
            ],
            100.0,
        );
    }

    fn show_combat_name_rules(&mut self, modified_settings: &mut Settings, ui: &mut Ui) {
        CollapsingHeader::new("Combat Name Detection rules").show_unindented(ui, |ui| {
            Self::show_group_rules_table(
                &mut modified_settings.analysis.combat_name_rules,
                "",
                "Combat Name",
                ui,
                023975,
                [
                    MatchAspect::DamageName,
                    MatchAspect::SubSourceName,
                    MatchAspect::SubUniqueSourceName,
                    MatchAspect::SourceOrTargetName,
                    MatchAspect::SourceOrTargetUniqueName,
                ],
                200.0,
            );
        });
    }

    fn show_group_rules_table(
        group_rules: &mut Vec<RulesGroup>,
        title: &str,
        name_header: &str,
        ui: &mut Ui,
        base_id: usize,
        match_aspect_set: impl IntoIterator<Item = MatchAspect> + Copy,
        popup_extra_space: f32,
    ) {
        ui.horizontal(|ui| {
            ui.label(title);
            if ui.button("✚").clicked() {
                group_rules.push(Default::default());
            }
        });
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto().at_least(600.0).resizable(true))
            .column(Column::auto())
            .cell_layout(Layout::left_to_right(Align::Center))
            .max_scroll_height(200.0)
            .header(0.0, |mut r| {
                r.col(|ui| {
                    ui.label("On");
                });
                r.col(|ui| {
                    ui.label("Edit");
                });
                r.col(|ui| {
                    ui.label(name_header);
                });
            })
            .body(|mut t| {
                let mut to_remove = Vec::new();
                for (id, group_rule) in group_rules.iter_mut().enumerate() {
                    t.row(25.0, |mut r| {
                        r.col(|ui| {
                            ui.checkbox(&mut group_rule.enabled, "");
                        });

                        r.col(|ui| {
                            PopupButton::new("✏")
                                .with_id_source(base_id + id)
                                .show(ui, |ui| {
                                    Self::show_rules_table(
                                        &mut group_rule.rules,
                                        &group_rule.name,
                                        ui,
                                        base_id + id,
                                        match_aspect_set,
                                    );
                                    // HACK: so that the popup does not close when clicking the in one of the combo boxes
                                    ui.add_space(popup_extra_space);
                                });
                        });

                        r.col(|ui| {
                            TextEdit::singleline(&mut group_rule.name)
                                .desired_width(600.0)
                                .show(ui);
                        });

                        r.col(|ui| {
                            if ui.selectable_label(false, "🗑").clicked() {
                                to_remove.push(id);
                            }
                        });
                    });
                }

                to_remove.into_iter().rev().for_each(|i| {
                    group_rules.remove(i);
                });
            });
    }

    fn show_rules_table(
        rules: &mut Vec<MatchRule>,
        title: &str,
        ui: &mut Ui,
        id: usize,
        match_aspect_set: impl IntoIterator<Item = MatchAspect> + Copy,
    ) {
        ui.horizontal(|ui| {
            ui.label(title);
            if ui.button("✚").clicked() {
                rules.push(Default::default());
            }
        });
        ui.push_id(id, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto().at_least(400.0).resizable(true))
                .column(Column::auto())
                .cell_layout(Layout::left_to_right(Align::Center))
                .max_scroll_height(100.0)
                .header(0.0, |mut r| {
                    r.col(|ui| {
                        ui.label("On");
                    });
                    r.col(|ui| {
                        ui.label("Aspect to match");
                    });
                    r.col(|ui| {
                        ui.label("Match Method");
                    });
                    r.col(|ui| {
                        ui.label("Text to match");
                    });
                })
                .body(|mut t| {
                    let mut to_remove = Vec::new();
                    for (id, rule) in rules.iter_mut().enumerate() {
                        t.row(25.0, |mut r| {
                            r.col(|ui| {
                                ui.checkbox(&mut rule.enabled, "");
                            });

                            r.col(|ui| {
                                ComboBox::from_id_source(id + 9387465)
                                    .selected_text(rule.aspect.display())
                                    .width(150.0)
                                    .show_ui(ui, |ui| {
                                        match_aspect_set.into_iter().for_each(|a| {
                                            ui.selectable_value(&mut rule.aspect, a, a.display());
                                        });
                                    });
                            });

                            r.col(|ui| {
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

                            r.col(|ui| {
                                TextEdit::singleline(&mut rule.expression)
                                    .desired_width(600.0)
                                    .show(ui);
                            });

                            r.col(|ui| {
                                if ui.selectable_label(false, "🗑").clicked() {
                                    to_remove.push(id);
                                }
                            });
                        });
                    }

                    to_remove.into_iter().rev().for_each(|i| {
                        rules.remove(i);
                    });
                });
        });
    }
}
