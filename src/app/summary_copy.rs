use eframe::egui::*;
use itertools::Itertools;

use crate::{
    analyzer::*,
    custom_widgets::popup_button::PopupButton,
    helpers::{
        format_duration, number_formatting::NumberFormatter, time_range_to_duration_or_zero,
    },
};

pub struct SummaryCopy {
    aspects: Vec<Aspect>,
}

struct Aspect {
    name: &'static str,
    header: &'static str,
    include: bool,
    get: fn(&Player) -> f64,
    format: fn(f64, &mut NumberFormatter) -> String,
    reverse_sort: bool,
}

impl SummaryCopy {
    pub fn show(&mut self, combat: Option<&Combat>, ui: &mut Ui) {
        if ui
            .add_enabled(combat.is_some(), Button::new("Copy Combat Summary"))
            .clicked()
        {
            ui.output_mut(|o| o.copied_text = self.build_summary(combat.unwrap()));
        }

        ui.add_enabled(combat.is_some(), |ui: &mut Ui| {
            PopupButton::new("⛭")
                .show(ui, |ui| {
                    ui.label("Configure copy elements");
                    for aspect in self.aspects.iter_mut() {
                        ui.checkbox(&mut aspect.include, aspect.name);
                    }

                    ui.label("Limit the number of elements,\nif you wish to paste the summary into the game chat.\nSo that it will not be truncated by the game.");
                })
                .response
        });
    }

    fn build_summary(&self, combat: &Combat) -> String {
        let mut number_formatter = NumberFormatter::new();
        let aspects = self.aspects.iter().filter(|a| a.include);
        let first_aspect = aspects.clone().nth(0).unwrap_or(&self.aspects[0]);
        let players = combat
            .players
            .values()
            .sorted_by(|p1, p2| {
                let cmp = (first_aspect.get)(p1).total_cmp(&(first_aspect.get)(p2));
                if first_aspect.reverse_sort {
                    return cmp.reverse();
                }
                cmp
            })
            .map(|p| {
                let aspects = aspects
                    .clone()
                    .map(|a| {
                        let value = (a.get)(p);
                        (a.format)(value, &mut number_formatter)
                    })
                    .join("|");

                format!(
                    "{} {}",
                    String::from_iter(
                        p.damage_in
                            .name()
                            .get(&combat.name_manager)
                            .chars()
                            .skip_while(|c| *c != '@')
                    ),
                    aspects
                )
            });

        let aspects = aspects.clone().map(|a| a.header).join("|");
        let aspects_header = format!("Name {}", aspects);

        let header_and_players = std::iter::once(aspects_header).chain(players).join(" / ");

        let duration = format_duration(time_range_to_duration_or_zero(&combat.combat_time));

        format!(
            "CLA - {} ({}): {}",
            combat.name(),
            duration,
            header_and_players
        )
    }
}

impl Default for SummaryCopy {
    fn default() -> Self {
        Self {
            aspects: vec![
                aspect(
                    "DPS",
                    "DPS",
                    true,
                    |p| p.damage_out.dps.all,
                    |v, f| f.format_with_automated_suffixes(v),
                    true,
                ),
                aspect(
                    "Damage",
                    "Dmg",
                    false,
                    |p| p.damage_out.total_damage.all,
                    |v, f| f.format_with_automated_suffixes(v),
                    true,
                ),
                aspect(
                    "Damage %",
                    "Dmg%",
                    false,
                    |p| p.damage_out.damage_percentage.all.unwrap_or(0.0),
                    |v, f| f.format(v, 1),
                    true,
                ),
                aspect(
                    "Critical %",
                    "Crit%",
                    false,
                    |p| p.damage_out.critical_percentage.unwrap_or(0.0),
                    |v, f| f.format(v, 1),
                    true,
                ),
                aspect(
                    "Damage Resistance %",
                    "DmgRes%",
                    false,
                    |p| p.damage_out.damage_resistance_percentage.unwrap_or(0.0),
                    |v, f| f.format(v, 1),
                    false,
                ),
                aspect(
                    "Damage In",
                    "DmgIn",
                    false,
                    |p| p.damage_in.total_damage.all,
                    |v, f| f.format_with_automated_suffixes(v),
                    true,
                ),
                aspect(
                    "Damage In %",
                    "DmgIn%",
                    false,
                    |p| p.damage_in.damage_percentage.all.unwrap_or(0.0),
                    |v, f| f.format(v, 1),
                    true,
                ),
                aspect(
                    "Damage Resistance In %",
                    "DmgIn%",
                    false,
                    |p| p.damage_in.damage_resistance_percentage.unwrap_or(0.0),
                    |v, f| f.format(v, 1),
                    true,
                ),
            ],
        }
    }
}

fn aspect(
    name: &'static str,
    header: &'static str,
    include: bool,
    get: fn(&Player) -> f64,
    format: fn(f64, &mut NumberFormatter) -> String,
    reverse_sort: bool,
) -> Aspect {
    Aspect {
        name,
        header,
        include,
        get,
        format,
        reverse_sort,
    }
}
