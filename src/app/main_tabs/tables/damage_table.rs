use crate::{
    analyzer::*, app::main_tabs::common::*, col, custom_widgets::table::*,
    helpers::number_formatting::NumberFormatter,
};

use super::metrics_table::*;

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
        |t| t.sort_by_option_f64_desc(|p| p.damage_percentage.value),
        |t, r| {
            t.damage_percentage.show(r);
        },
    ),
    col!(
        "Damage Resistance %",
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
    col!("Hits", |t| t.sort_by_desc(|p| p.hits.all.count), |t, r| {
            t.hits.show(r);
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
];

pub struct DamageTablePartData {
    total_damage: ShieldAndHullTextValue,
    dps: ShieldAndHullTextValue,
    damage_percentage: TextValue,
    max_one_hit: MaxOneHit,
    average_hit: ShieldAndHullTextValue,
    critical_percentage: TextValue,
    flanking: TextValue,
    damage_resistance_percentage: TextValue,
    base_damage: TextValue,
    base_dps: TextValue,
    hits: ShieldAndHullTextCount,
    damage_types: DamageTypes,
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
    fn new(source: &DamageGroup, number_formatter: &mut NumberFormatter) -> Self {
        Self {
            total_damage: ShieldAndHullTextValue::new(&source.total_damage, 2, number_formatter),
            dps: ShieldAndHullTextValue::new(&source.dps, 2, number_formatter),
            damage_percentage: TextValue::new(source.damage_percentage, 3, number_formatter),
            average_hit: ShieldAndHullTextValue::option(&source.average_hit, 2, number_formatter),
            critical_percentage: TextValue::option(source.critical_percentage, 3, number_formatter),
            flanking: TextValue::option(source.flanking, 3, number_formatter),
            max_one_hit: MaxOneHit::new(source, number_formatter),
            damage_resistance_percentage: TextValue::option(
                source.damage_resistance_percentage,
                3,
                number_formatter,
            ),
            base_damage: TextValue::new(source.total_base_damage, 2, number_formatter),
            base_dps: TextValue::new(source.base_dps, 2, number_formatter),
            damage_types: DamageTypes::new(source),
            hits: ShieldAndHullTextCount::new(&source.damage_metrics.hits),
            source_hits: source
                .hits
                .iter()
                .chain(source.hits.iter())
                .copied()
                .collect(),
        }
    }
}

impl DamageTablePart {
    pub fn dps(&self) -> f64 {
        self.dps.all.value.unwrap()
    }

    pub fn total_damage(&self) -> f64 {
        self.total_damage.all.value.unwrap()
    }
}

impl MaxOneHit {
    fn new(source: &DamageGroup, number_formatter: &mut NumberFormatter) -> Self {
        Self {
            damage: TextValue::new(source.max_one_hit.damage, 2, number_formatter),
            name: source.max_one_hit.name.clone(),
        }
    }

    fn show(&self, row: &mut TableRow) {
        if let Some(response) = self.damage.show(row) {
            response.on_hover_text(&self.name);
        }
    }
}

impl DamageTypes {
    fn new(source: &DamageGroup) -> Self {
        match source.damage_types.len() {
            0 => Self::Unknown,
            1 => Self::Single(source.damage_types.iter().nth(0).unwrap().clone()),
            _ => Self::Mixed(source.damage_types.iter().cloned().collect()),
        }
    }

    fn show(&self, row: &mut TableRow) {
        row.cell(|ui| match self {
            DamageTypes::Unknown => (),
            DamageTypes::Single(damage_type) => {
                ui.label(damage_type);
            }
            DamageTypes::Mixed(damage_types) => {
                ui.label("<mixed>").on_hover_ui(|ui| {
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
