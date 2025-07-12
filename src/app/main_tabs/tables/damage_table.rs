use crate::{
    analyzer::*, app::main_tabs::common::*, col, custom_widgets::table::*,
    helpers::number_formatting::NumberFormatter,
};

use super::{common::Kills, metrics_table::*};

static COLUMNS: &[ColumnDescriptor<DamageTablePartData>] = &[
    col!(
        "DPS",
        "Damage Per Second\nCalculated from the first damage of the player to the last damage in the log",
        |t| t.sort_by_option_f64_desc(|p| p.dps.all.value),
        |t, r| t.dps.show(r),
    ),
    col!(
        "Total Damage",
        |t| t.sort_by_option_f64_desc(|p| p.total_damage.all.value),
        |t, r| t.total_damage.show(r),
    ),
    col!(
        "Damage %",
        |t| t.sort_by_option_f64_desc(|p| p.damage_percentage.all.value),
        |t, r| {
            t.damage_percentage.show(r);
        },
    ),
    col!(
        "Resistance %",
        "Damage Resistance % excluding any drain damage",
        |t| t.sort_by_option_f64_asc(|p| p.damage_resistance_percentage.value),
        |t, r| {
            t.damage_resistance_percentage.show(r);
        },
    ),
    col!(
        "Max One-Hit",
        |t| t.sort_by_option_f64_desc(|p| p.max_one_hit.damage.value),
        |t, r| t.max_one_hit.show(r),
    ),
    col!(
        "Average Hit",
        |t| t.sort_by_option_f64_desc(|p| p.average_hit.all.value),
        |t, r| t.average_hit.show(r),
    ),
    col!(
        "Critical %",
        |t| t.sort_by_option_f64_desc(|p| p.critical_percentage.value),
        |t, r| {
            t.critical_percentage.show(r);
        },
    ),
    col!(
        "Flanking %",
        |t| t.sort_by_option_f64_desc(|p| p.flanking.value),
        |t, r| {
            t.flanking.show(r);
        },
    ),
    col!("Hits",
        "Every damage number that shows up, counts as one hit.\nThis means for an attack, that hits the shields of an enemy, 2 Hits will be counted. One for the shield Hit and one for the hull Hit.",
        |t| t.sort_by_desc(|p| p.hits.all.count), |t, r| {
            t.hits.show(r);
        },
    ),
    col!("Hits / s",
        "Hits Per Second\nCalculated from the first damage of the player to the last damage in the log",
        |t| t.sort_by_option_f64_desc(|p| p.hits_per_second.all.value),
        |t, r| {
            t.hits_per_second.show(r);
        },
    ),
    col!("Hits %", |t| t.sort_by_option_f64_desc(|p| p.hits_percentage.all.value), |t, r| {
            t.hits_percentage.show(r);
        },
    ),
    col!("Misses", |t| t.sort_by_asc(|p| p.misses.count), |t, r| {
            t.misses.show(r);
        },
    ),
    col!("Accuracy %", |t| t.sort_by_option_f64_desc(|p| p.accuracy_percentage.value), |t, r| {
            t.accuracy_percentage.show(r);
        },
    ),
    col!("Kills", |t| t.sort_by_asc(|p| p.kills.total_count), |t, r| {
            t.kills.show(r);
        },
    ),
    col!("Damage Types", |t| t.sort_by_desc(|p| p.damage_types.clone()), |t, r| {
            t.damage_types.show(r);
        },
    ),
    col!(
        "Base DPS",
        "Damage Per Second If there were no shields and no damage resistances\nThis excludes any drain damage",
        |t| t.sort_by_option_f64_desc(|p| p.base_dps.value),
        |t, r| {
            t.base_dps.show(r);
        },
    ),
    col!(
        "Base Damage",
        "Damage If there were no shields and no damage resistances\nThis excludes any drain damage",
        |t| t.sort_by_option_f64_desc(|p| p.base_damage.value),
        |t, r| {
            t.base_damage.show(r);
        },
    ),
    col!(
        "Total Crit Damage",
        "Note that this only applies to hull hits, since shield hits do not crit",
        |t| t.sort_by_option_f64_desc(|p| p.total_crit_damage.value),
        |t, r| {
            t.total_crit_damage.show(r);
        },
    ),
     col!(
        "Total Non-Crit Hull Damage",
        |t| t.sort_by_option_f64_desc(|p| p.total_non_crit_hull_damage.value),
        |t, r| {
            t.total_non_crit_hull_damage.show(r);
        },
    ),
    col!(
        "Average Crit Hit",
        "Note that this only applies to hull hits, since shield hits do not crit",
        |t| t.sort_by_option_f64_desc(|p| p.average_crit_hit.value),
        |t, r| {
            t.average_crit_hit.show(r);
        },
    ),
    col!(
        "Average Non-Crit Hull Hit",
        |t| t.sort_by_option_f64_desc(|p| p.average_non_crit_hull_hit.value),
        |t, r| {
            t.average_non_crit_hull_hit.show(r);
        },
    ),
];

pub struct DamageTablePartData {
    total_damage: ShieldAndHullTextValue,
    dps: ShieldAndHullTextValue,
    damage_percentage: ShieldAndHullTextValue,
    max_one_hit: MaxOneHit,
    average_hit: ShieldAndHullTextValue,
    critical_percentage: TextValue,
    flanking: TextValue,
    damage_resistance_percentage: TextValue,
    base_damage: TextValue,
    base_dps: TextValue,
    hits: ShieldAndHullTextCount,
    hits_per_second: ShieldAndHullTextValue,
    hits_percentage: ShieldAndHullTextValue,
    misses: TextCount,
    accuracy_percentage: TextValue,
    kills: Kills,
    damage_types: DamageTypes,
    total_crit_damage: TextValue,
    total_non_crit_hull_damage: TextValue,
    average_crit_hit: TextValue,
    average_non_crit_hull_hit: TextValue,
    pub source_hits: Vec<Hit>,
}

pub type DamageTable = MetricsTable<DamageTablePartData>;
pub type DamageTablePart = MetricsTablePart<DamageTablePartData>;

struct MaxOneHit {
    damage: TextValue,
    name: String,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
enum DamageTypes {
    Unknown,
    Mixed(Vec<String>),
    Single(String),
}

impl DamageTable {
    pub fn empty() -> Self {
        Self::empty_base(COLUMNS)
    }

    pub fn new(combat: &Combat, damage_group: impl FnMut(&Player) -> &DamageGroup) -> Self {
        Self::new_base(COLUMNS, combat, damage_group, DamageTablePartData::new)
    }
}

impl DamageTablePartData {
    fn new(source: &DamageGroup, combat: &Combat, number_formatter: &mut NumberFormatter) -> Self {
        Self {
            total_damage: ShieldAndHullTextValue::new(&source.total_damage, 2, number_formatter),
            dps: ShieldAndHullTextValue::new(&source.dps, 2, number_formatter),
            damage_percentage: ShieldAndHullTextValue::option(
                &source.damage_percentage,
                3,
                number_formatter,
            ),
            average_hit: ShieldAndHullTextValue::option(&source.average_hit, 2, number_formatter),
            critical_percentage: TextValue::option(source.critical_percentage, 3, number_formatter),
            flanking: TextValue::option(source.flanking, 3, number_formatter),
            max_one_hit: MaxOneHit::new(source, number_formatter, &combat.name_manager),
            damage_resistance_percentage: TextValue::option(
                source.damage_resistance_percentage,
                3,
                number_formatter,
            ),
            base_damage: TextValue::new(source.total_base_damage, 2, number_formatter),
            base_dps: TextValue::new(source.base_dps, 2, number_formatter),
            kills: Kills::new(source, &combat.name_manager),
            damage_types: DamageTypes::new(source, &combat.name_manager),
            hits: ShieldAndHullTextCount::new(&source.damage_metrics.hits),
            hits_per_second: ShieldAndHullTextValue::new(
                &source.hits_per_second,
                3,
                number_formatter,
            ),
            hits_percentage: ShieldAndHullTextValue::option(
                &source.hits_percentage,
                3,
                number_formatter,
            ),
            misses: TextCount::new(source.misses),
            accuracy_percentage: TextValue::option(source.accuracy_percentage, 3, number_formatter),
            total_crit_damage: TextValue::new(source.total_crit_damage, 2, number_formatter),
            total_non_crit_hull_damage: TextValue::new(
                source.total_non_crit_hull_damage,
                2,
                number_formatter,
            ),
            average_crit_hit: TextValue::option(source.average_crit_hit, 2, number_formatter),
            average_non_crit_hull_hit: TextValue::option(
                source.average_non_crit_hull_hit,
                2,
                number_formatter,
            ),
            source_hits: source.hits.get(&combat.hits_manger).to_vec(),
        }
    }
}

impl DamageTablePart {
    pub fn total_damage(&self) -> f64 {
        self.total_damage.all.value.unwrap()
    }
}

impl MaxOneHit {
    fn new(
        source: &DamageGroup,
        number_formatter: &mut NumberFormatter,
        name_manager: &NameManager,
    ) -> Self {
        Self {
            damage: TextValue::new(source.max_one_hit.damage, 2, number_formatter),
            name: source.max_one_hit.name.get(name_manager).to_string(),
        }
    }

    fn show(&self, row: &mut TableRow) {
        if let Some(response) = self.damage.show(row) {
            response.on_hover_text(&self.name);
        }
    }
}

impl DamageTypes {
    fn new(source: &DamageGroup, name_manager: &NameManager) -> Self {
        match source.damage_types.len() {
            0 => Self::Unknown,
            1 => Self::Single(
                source
                    .damage_types
                    .iter()
                    .nth(0)
                    .unwrap()
                    .get(name_manager)
                    .to_string(),
            ),
            _ => Self::Mixed(
                source
                    .damage_types
                    .iter()
                    .map(|d| d.get(name_manager).to_string())
                    .collect(),
            ),
        }
    }

    fn show(&self, row: &mut TableRow) {
        row.cell(|ui| match self {
            DamageTypes::Unknown => (),
            DamageTypes::Single(damage_type) => {
                ui.label(damage_type);
            }
            DamageTypes::Mixed(damage_types) => {
                details_tooltip(ui.label("<mixed>"), |ui| {
                    Table::new(ui).body(ROW_HEIGHT, |b| {
                        for damage_type in damage_types.iter() {
                            b.row(|r| {
                                r.cell(|ui| {
                                    ui.label(damage_type);
                                });
                            });
                        }
                    });
                });
            }
        });
    }
}
