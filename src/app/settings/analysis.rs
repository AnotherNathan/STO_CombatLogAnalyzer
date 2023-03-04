use std::borrow::BorrowMut;

use eframe::egui::*;

use super::Settings;
use crate::analyzer::Combat;
use crate::custom_widgets::table::Table;
use crate::unwrap_or_return;
use crate::{analyzer::settings::*, custom_widgets::popup_button::PopupButton};

const HEADER_HEIGHT: f32 = 15.0;
const ROW_HEIGHT: f32 = 25.0;

#[derive(Default)]
pub struct AnalysisTab {
    list_selected_combat_occurred_names: bool,
    occurred_combat_names_search_term: String,
}

impl AnalysisTab {
    pub fn show(
        &mut self,
        modified_settings: &mut Settings,
        selected_combat: Option<&Combat>,
        ui: &mut Ui,
    ) {
        self.show_indirect_source_grouping_reversal_rules(modified_settings, ui);
        ui.add_space(20.0);

        ui.separator();
        ui.push_id(line!(), |ui| {
            self.show_grouping_rules(modified_settings, ui);
        });
        ui.add_space(20.0);

        ui.separator();
        self.show_combat_name_rules(modified_settings, selected_combat, ui);
    }

    fn show_indirect_source_grouping_reversal_rules(
        &mut self,
        modified_settings: &mut Settings,
        ui: &mut Ui,
    ) {
        Self::show_rules_table(
            &mut modified_settings
                .analysis
                .indirect_source_grouping_revers_rules,
            "Indirect Source Grouping Reversal Rules\n(e.g. pets, anomalies, certain traits etc.)",
            ui,
            [
                MatchAspect::DamageOrHealName,
                MatchAspect::IndirectSourceName,
                MatchAspect::IndirectUniqueSourceName,
            ],
        );
    }

    fn show_grouping_rules(&mut self, modified_settings: &mut Settings, ui: &mut Ui) {
        Self::show_group_rules_table(
            &mut modified_settings.analysis.custom_group_rules,
            "Custom Grouping Rules",
            "Group Name",
            ui,
            100.0,
            |r, ui| {
                Self::show_rules_table(
                    &mut r.rules,
                    &r.name,
                    ui,
                    [
                        MatchAspect::DamageOrHealName,
                        MatchAspect::IndirectSourceName,
                        MatchAspect::IndirectUniqueSourceName,
                    ],
                );
            },
        );
    }

    fn show_combat_name_rules(
        &mut self,
        modified_settings: &mut Settings,
        selected_combat: Option<&Combat>,
        ui: &mut Ui,
    ) {
        CollapsingHeader::new("Combat Name Detection Rules").show_unindented(ui, |ui| {
            if ui
                .add_enabled(
                    selected_combat.is_some(),
                    Button::new("List Selected Combat Occurred Names"),
                )
                .clicked()
            {
                self.list_selected_combat_occurred_names = true;
            }

            Self::show_group_rules_table(
                &mut modified_settings.analysis.combat_name_rules,
                "",
                "Combat Name",
                ui,
                200.0,
                |r, ui| {
                    Self::show_rules_table(
                        &mut r.name_rule.rules,
                        "combat name",
                        ui,
                        [
                            MatchAspect::DamageOrHealName,
                            MatchAspect::IndirectSourceName,
                            MatchAspect::IndirectUniqueSourceName,
                            MatchAspect::SourceOrTargetName,
                            MatchAspect::SourceOrTargetUniqueName,
                        ],
                    );

                    ui.push_id("additional info rules", |ui| {
                        Self::show_group_rules_table(
                            &mut r.additional_info_rules,
                            "additional infos rules (e.g. difficulty)",
                            "Info",
                            ui,
                            200.0,
                            |r, ui| {
                                Self::show_rules_table(
                                    &mut r.rules,
                                    &r.name,
                                    ui,
                                    [
                                        MatchAspect::DamageOrHealName,
                                        MatchAspect::IndirectSourceName,
                                        MatchAspect::IndirectUniqueSourceName,
                                        MatchAspect::SourceOrTargetName,
                                        MatchAspect::SourceOrTargetUniqueName,
                                    ],
                                );
                            },
                        );
                    });
                },
            );

            self.show_occurred_names_window(selected_combat, ui);
        });
    }

    fn show_group_rules_table<T: BorrowMut<RulesGroup> + Default>(
        group_rules: &mut Vec<T>,
        title: &str,
        name_header: &str,
        ui: &mut Ui,
        popup_extra_space: f32,
        mut edit: impl FnMut(&mut T, &mut Ui),
    ) {
        ui.horizontal(|ui| {
            ui.label(title);
            if ui.button("Add ✚").clicked() {
                group_rules.push(Default::default());
            }
        });
        Table::new(ui)
            .min_scroll_height(200.0)
            .max_scroll_height(200.0)
            .header(HEADER_HEIGHT, |r| {
                r.cell(|ui| {
                    ui.label("On");
                });
                r.cell(|ui| {
                    ui.label("Edit");
                });
                r.cell(|ui| {
                    ui.label(name_header);
                });
            })
            .body(ROW_HEIGHT, |t| {
                let mut to_remove = Vec::new();
                for (id, rule) in group_rules.iter_mut().enumerate() {
                    t.row(|r| {
                        r.cell(|ui| {
                            ui.checkbox(&mut rule.borrow_mut().enabled, "");
                        });

                        r.cell(|ui| {
                            PopupButton::new("✏").show(ui, |ui| {
                                edit(rule, ui);
                                // HACK: so that the popup does not close when clicking the in one of the combo boxes
                                ui.add_space(popup_extra_space);
                            });
                        });

                        r.cell(|ui| {
                            TextEdit::singleline(&mut rule.borrow_mut().name)
                                .min_size(vec2(600.0, 0.0))
                                .show(ui);
                        });

                        r.cell(|ui| {
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
        match_aspect_set: impl IntoIterator<Item = MatchAspect> + Copy,
    ) {
        ui.horizontal(|ui| {
            ui.label(title);
            if ui.button("Add ✚").clicked() {
                rules.push(Default::default());
            }
        });
        Table::new(ui)
            .min_scroll_height(100.0)
            .max_scroll_height(200.0)
            .header(HEADER_HEIGHT, |r| {
                r.cell(|ui| {
                    ui.label("On");
                });
                r.cell(|ui| {
                    ui.label("Aspect to match");
                });
                r.cell(|ui| {
                    ui.label("Match Method");
                });
                r.cell(|ui| {
                    ui.label("Text to match");
                });
            })
            .body(ROW_HEIGHT, |t| {
                let mut to_remove = Vec::new();
                for (id, rule) in rules.iter_mut().enumerate() {
                    t.row(|r| {
                        r.cell(|ui| {
                            ui.checkbox(&mut rule.enabled, "");
                        });

                        r.cell(|ui| {
                            ComboBox::from_id_source(id + 9387465)
                                .selected_text(rule.aspect.display())
                                .width(150.0)
                                .show_ui(ui, |ui| {
                                    match_aspect_set.into_iter().for_each(|a| {
                                        ui.selectable_value(&mut rule.aspect, a, a.display());
                                    });
                                });
                        });

                        r.cell(|ui| {
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

                        r.cell(|ui| {
                            TextEdit::singleline(&mut rule.expression)
                                .min_size(vec2(400.0, 0.0))
                                .show(ui);
                        });

                        r.cell(|ui| {
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
    }

    fn show_occurred_names_window(&mut self, selected_combat: Option<&Combat>, ui: &mut Ui) {
        let combat = unwrap_or_return!(selected_combat);
        if !self.list_selected_combat_occurred_names {
            return;
        }

        Window::new("Selected Combat Occurred Names")
            .collapsible(false)
            .open(&mut self.list_selected_combat_occurred_names)
            .scroll2([true; 2])
            .constrain(true)
            .show(ui.ctx(), |ui| {
                const SPACE: f32 = 40.0;

                ui.label("This window is intended to help with creating combat naming rules.");

                ui.horizontal(|ui| {
                    ui.label("Search");
                    ui.text_edit_singleline(&mut self.occurred_combat_names_search_term);
                });

                ui.add_space(SPACE);

                Self::show_occurred_names_table(
                    ui,
                    "Source or Target Name",
                    &self.occurred_combat_names_search_term,
                    combat.name_occurrences.source_target_names.iter(),
                );

                ui.add_space(SPACE);

                Self::show_occurred_names_table(
                    ui,
                    "Source or Target Unique Name",
                    &self.occurred_combat_names_search_term,
                    combat.name_occurrences.source_target_unique_names.iter(),
                );

                ui.add_space(SPACE);

                Self::show_occurred_names_table(
                    ui,
                    "Indirect Source Name",
                    &self.occurred_combat_names_search_term,
                    combat.name_occurrences.indirect_source_names.iter(),
                );

                ui.add_space(SPACE);

                Self::show_occurred_names_table(
                    ui,
                    "Indirect Source Unique Name",
                    &self.occurred_combat_names_search_term,
                    combat.name_occurrences.indirect_source_unique_names.iter(),
                );

                ui.add_space(SPACE);

                Self::show_occurred_names_table(
                    ui,
                    "Damage / Heal Name",
                    &self.occurred_combat_names_search_term,
                    combat.name_occurrences.value_names.iter(),
                );
            });
    }

    fn show_occurred_names_table<'a>(
        ui: &mut Ui,
        title: &str,
        filter: &str,
        names: impl Iterator<Item = &'a String>,
    ) {
        ui.push_id(title, |ui| {
            Table::new(ui)
                .min_scroll_height(300.0)
                .max_scroll_height(300.0)
                .header(HEADER_HEIGHT, |r| {
                    r.cell(|ui| {
                        ui.label(title);
                    });
                })
                .body(ROW_HEIGHT, |b| {
                    for name in names.filter(|n| {
                        filter.len() == 0 || n.to_lowercase().contains(&filter.to_lowercase())
                    }) {
                        b.row(|r| {
                            r.cell(|ui| {
                                ui.label(name);
                            });
                            r.cell(|ui| {
                                if ui.button("🗐").on_hover_text("Copy").clicked() {
                                    ui.output_mut(|o| o.copied_text = name.clone());
                                }
                            });
                        });
                    }
                });
        });
    }
}
