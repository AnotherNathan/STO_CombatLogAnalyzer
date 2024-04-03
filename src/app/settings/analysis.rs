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
    indirect_source_reversal_rules: IndirectSourceReversalRules,
    custom_grouping_rules: CustomGroupingRules,
    damage_out_exclusion_rules: DamageOutExclusionRules,
    combat_names_rules: CombatNameRules,
}

#[derive(Default)]
struct IndirectSourceReversalRules {
    selected: Option<usize>,
}

#[derive(Default)]
struct CustomGroupingRules {
    selected_group: Option<usize>,
    selected_rule: Option<usize>,
}

#[derive(Default)]
struct DamageOutExclusionRules {
    selected: Option<usize>,
}

#[derive(Default)]
struct CombatNameRules {
    selected_group: Option<usize>,
    selected_rule: Option<usize>,
    selected_additional_info_group: Option<usize>,
    selected_additional_info_rule: Option<usize>,
}

struct GroupRulesTable<'a, T: BorrowMut<RulesGroup> + Default> {
    group_rules: &'a mut Vec<T>,
    title: &'a str,
    name_header: &'a str,
    selected_group: &'a mut Option<usize>,
    popup_extra_space: f32,
}

struct RulesTable<'a> {
    rules: &'a mut Vec<MatchRule>,
    title: &'a str,
    match_aspect_set: &'a [MatchAspect],
    selected_rule: &'a mut Option<usize>,
}

impl AnalysisTab {
    pub fn show(
        &mut self,
        modified_settings: &mut Settings,
        selected_combat: Option<&Combat>,
        ui: &mut Ui,
    ) {
        if ui
            .add_enabled(
                selected_combat.is_some(),
                Button::new("List Selected Combat Occurred Names"),
            )
            .clicked()
        {
            self.list_selected_combat_occurred_names = true;
        }

        self.indirect_source_reversal_rules
            .show(&mut modified_settings.analysis, ui);
        ui.add_space(20.0);

        ui.separator();
        ui.push_id(line!(), |ui| {
            self.custom_grouping_rules
                .show(&mut modified_settings.analysis, ui);
        });
        ui.add_space(20.0);

        ui.separator();
        self.damage_out_exclusion_rules
            .show(&mut modified_settings.analysis, ui);
        ui.add_space(20.0);

        ui.separator();
        self.combat_names_rules
            .show(&mut modified_settings.analysis, ui);

        self.show_occurred_names_window(selected_combat, ui);
    }

    fn show_occurred_names_window(&mut self, selected_combat: Option<&Combat>, ui: &mut Ui) {
        let combat = unwrap_or_return!(selected_combat);
        if !self.list_selected_combat_occurred_names {
            return;
        }

        Window::new("Selected Combat Occurred Names")
            .collapsible(false)
            .open(&mut self.list_selected_combat_occurred_names)
            .scroll2(true)
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
                    combat.name_manager.source_targets(),
                );

                ui.add_space(SPACE);

                Self::show_occurred_names_table(
                    ui,
                    "Source or Target Unique Name",
                    &self.occurred_combat_names_search_term,
                    combat.name_manager.source_targets_unique(),
                );

                ui.add_space(SPACE);

                Self::show_occurred_names_table(
                    ui,
                    "Indirect Source Name",
                    &self.occurred_combat_names_search_term,
                    combat.name_manager.indirect_sources(),
                );

                ui.add_space(SPACE);

                Self::show_occurred_names_table(
                    ui,
                    "Indirect Source Unique Name",
                    &self.occurred_combat_names_search_term,
                    combat.name_manager.source_targets_unique(),
                );

                ui.add_space(SPACE);

                Self::show_occurred_names_table(
                    ui,
                    "Damage / Heal Name",
                    &self.occurred_combat_names_search_term,
                    combat.name_manager.values(),
                );
            });
    }

    fn show_occurred_names_table<'a>(
        ui: &mut Ui,
        title: &str,
        filter: &str,
        names: impl Iterator<Item = &'a str>,
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
                                    ui.output_mut(|o| o.copied_text = name.to_string());
                                }
                            });
                        });
                    }
                });
        });
    }
}

impl IndirectSourceReversalRules {
    fn show(&mut self, modified_settings: &mut AnalysisSettings, ui: &mut Ui) {
        RulesTable::new(
            &mut modified_settings.indirect_source_grouping_revers_rules,
            "Indirect Source Grouping Reversal Rules\n(e.g. pets, anomalies, certain traits etc.)",
            &[
                MatchAspect::DamageOrHealName,
                MatchAspect::IndirectSourceName,
                MatchAspect::IndirectUniqueSourceName,
            ],
            &mut self.selected,
        )
        .show(ui);
    }
}

impl DamageOutExclusionRules {
    fn show(&mut self, modified_settings: &mut AnalysisSettings, ui: &mut Ui) {
        RulesTable::new(
            &mut modified_settings.damage_out_exclusion_rules,
            "Damage Out Exclusion Rules",
            &[
                MatchAspect::DamageOrHealName,
                MatchAspect::IndirectSourceName,
                MatchAspect::IndirectUniqueSourceName,
                MatchAspect::SourceOrTargetName,
                MatchAspect::SourceOrTargetUniqueName,
            ],
            &mut self.selected,
        )
        .show(ui);
    }
}

impl CustomGroupingRules {
    fn show(&mut self, modified_settings: &mut AnalysisSettings, ui: &mut Ui) {
        GroupRulesTable::new(
            &mut modified_settings.custom_group_rules,
            "Custom Grouping Rules",
            "Group Name",
            &mut self.selected_group,
            100.0,
        )
        .show(ui, |r, ui| {
            RulesTable::new(
                &mut r.rules,
                &r.name,
                &[
                    MatchAspect::DamageOrHealName,
                    MatchAspect::IndirectSourceName,
                    MatchAspect::IndirectUniqueSourceName,
                ],
                &mut self.selected_rule,
            )
            .show(ui);
        });
    }
}

impl CombatNameRules {
    fn show(&mut self, modified_settings: &mut AnalysisSettings, ui: &mut Ui) {
        CollapsingHeader::new("Combat Name Detection Rules").show_unindented(ui, |ui| {
            GroupRulesTable::new(
                &mut modified_settings.combat_name_rules,
                "",
                "Combat Name",
                &mut self.selected_group,
                200.0,
            )
            .show(ui, |r, ui| {
                RulesTable::new(
                    &mut r.name_rule.rules,
                    "combat name",
                    &[
                        MatchAspect::DamageOrHealName,
                        MatchAspect::IndirectSourceName,
                        MatchAspect::IndirectUniqueSourceName,
                        MatchAspect::SourceOrTargetName,
                        MatchAspect::SourceOrTargetUniqueName,
                    ],
                    &mut self.selected_rule,
                )
                .show(ui);

                ui.push_id("additional info rules", |ui| {
                    GroupRulesTable::new(
                        &mut r.additional_info_rules,
                        "additional infos rules (e.g. difficulty)",
                        "Info",
                        &mut self.selected_additional_info_group,
                        200.0,
                    )
                    .show(ui, |r, ui| {
                        RulesTable::new(
                            &mut r.rules,
                            &r.name,
                            &[
                                MatchAspect::DamageOrHealName,
                                MatchAspect::IndirectSourceName,
                                MatchAspect::IndirectUniqueSourceName,
                                MatchAspect::SourceOrTargetName,
                                MatchAspect::SourceOrTargetUniqueName,
                            ],
                            &mut self.selected_additional_info_rule,
                        )
                        .show(ui);
                    });
                });
            });
        });
    }
}

impl<'a, T: BorrowMut<RulesGroup> + Default> GroupRulesTable<'a, T> {
    fn new(
        group_rules: &'a mut Vec<T>,
        title: &'a str,
        name_header: &'a str,
        selected_group: &'a mut Option<usize>,
        popup_extra_space: f32,
    ) -> Self {
        Self {
            group_rules,
            title,
            name_header,
            selected_group,
            popup_extra_space,
        }
    }

    fn show(&mut self, ui: &mut Ui, mut edit: impl FnMut(&mut T, &mut Ui)) {
        ui.horizontal(|ui| {
            ui.label(self.title);
            if ui.button("Add ✚").clicked() {
                self.group_rules.push(Default::default());
            }

            show_move_up_down(self.selected_group, self.group_rules, ui);
        });
        Table::new(ui)
            .min_scroll_height(200.0)
            .max_scroll_height(200.0)
            .cell_spacing(10.0)
            .header(HEADER_HEIGHT, |r| {
                r.cell(|ui| {
                    ui.label("On");
                });
                r.cell(|ui| {
                    ui.label("Edit");
                });
                r.cell(|ui| {
                    ui.label(self.name_header);
                });
            })
            .body(ROW_HEIGHT, |t| {
                let mut to_remove = Vec::new();
                for (id, rule) in self.group_rules.iter_mut().enumerate() {
                    let row_response = t.selectable_row(*self.selected_group == Some(id), |r| {
                        r.cell(|ui| {
                            ui.checkbox(&mut rule.borrow_mut().enabled, "");
                        });

                        r.cell(|ui| {
                            PopupButton::new("✏").show(ui, |ui| {
                                edit(rule, ui);
                                // HACK: so that the popup does not close when clicking the in one of the combo boxes
                                ui.add_space(self.popup_extra_space);
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

                    if row_response.clicked() {
                        *self.selected_group = Some(id);
                    }
                }

                to_remove.into_iter().rev().for_each(|i| {
                    self.group_rules.remove(i);
                });
            });
    }
}

impl<'a> RulesTable<'a> {
    fn new(
        rules: &'a mut Vec<MatchRule>,
        title: &'a str,
        match_aspect_set: &'a [MatchAspect],
        selected_rule: &'a mut Option<usize>,
    ) -> Self {
        Self {
            rules,
            title,
            match_aspect_set,
            selected_rule,
        }
    }

    fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(self.title);
            if ui.button("Add ✚").clicked() {
                self.rules.push(Default::default());
            }

            show_move_up_down(self.selected_rule, self.rules, ui);
        });
        ui.push_id(self.title, |ui| {
            Table::new(ui)
                .min_scroll_height(100.0)
                .max_scroll_height(200.0)
                .cell_spacing(10.0)
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
                    for (id, rule) in self.rules.iter_mut().enumerate() {
                        let row_response = t.selectable_row(*self.selected_rule == Some(id), |r| {
                            r.cell(|ui| {
                                ui.checkbox(&mut rule.enabled, "");
                            });

                            r.cell(|ui| {
                                ComboBox::from_id_source(id + 9387465)
                                    .selected_text(rule.aspect.display())
                                    .width(150.0)
                                    .show_ui(ui, |ui| {
                                        self.match_aspect_set.into_iter().for_each(|a| {
                                            ui.selectable_value(&mut rule.aspect, *a, a.display());
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

                        if row_response.clicked() {
                            *self.selected_rule = Some(id);
                        }
                    }

                    to_remove.into_iter().rev().for_each(|i| {
                        self.rules.remove(i);
                    });
                });
        });
    }
}

fn show_move_up_down<T>(selected: &mut Option<usize>, items: &mut Vec<T>, ui: &mut Ui) {
    if ui
        .add_enabled(
            selected.map(|s| s > 0 && s < items.len()).unwrap_or(false),
            Button::new("⬆"),
        )
        .clicked()
    {
        let index = selected.unwrap();
        items.swap(index, index - 1);
        *selected = Some(index - 1);
    }

    if ui
        .add_enabled(
            selected.map(|s| s < items.len() - 1).unwrap_or(false),
            Button::new("⬇"),
        )
        .clicked()
    {
        let index = selected.unwrap();
        items.swap(index, index + 1);
        *selected = Some(index + 1);
    }
}
