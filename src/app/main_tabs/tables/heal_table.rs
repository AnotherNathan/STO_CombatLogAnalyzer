use crate::{
    analyzer::*,
    app::{main_tabs::common::*, settings::Settings},
    col,
    helpers::number_formatting::NumberFormatter,
};

use super::metrics_table::*;

static COLUMNS: &[ColumnDescriptor<HealTablePartData>] = &[
    col!(
        "HPS",
        "Heals Per Second\nCalculated from the first action of the player to the last action in the log",
        |t| t.sort_by_option_f64_desc(|p| p.hps.all.value),
        |t, r| t.hps.show(r),
    ),
    col!(
        "Total Heal",
        |t| t.sort_by_option_f64_desc(|p| p.total_heal.all.value),
        |t, r| t.total_heal.show(r),
    ),
    col!(
        "Heal %",
        |t| t.sort_by_option_f64_desc(|p| p.heal_percentage.all.value),
        |t, r| {
            t.heal_percentage.show(r);
        },
    ),
    col!(
        "Average Heal",
        |t| t.sort_by_option_f64_desc(|p| p.average_heal.all.value),
        |t, r| t.average_heal.show(r),
    ),
    col!(
        "Critical %",
        |t| t.sort_by_option_f64_desc(|p| p.critical_percentage.value),
        |t, r| {
            t.critical_percentage.show(r);
        },
    ),
    col!(
        "Ticks",
        |t| t.sort_by_desc(|p| p.ticks.all.count),
        |t, r| {
            t.ticks.show(r);
        },
    ),
    col!(
        "Ticks / s",
        "Ticks Per Second\nCalculated from the first action of the player to the last action in the log",
        |t| t.sort_by_option_f64_desc(|p| p.ticks_per_second.all.value),
        |t, r| {
            t.ticks_per_second.show(r);
        },
    ),
    col!(
        "Ticks %",
        |t| t.sort_by_option_f64_desc(|p| p.ticks_percentage.all.value),
        |t, r| {
            t.ticks_percentage.show(r);
        },
    ),
];

pub struct HealTablePartData {
    total_heal: ShieldAndHullTextValue,
    hps: ShieldAndHullTextValue,
    heal_percentage: ShieldAndHullTextValue,
    average_heal: ShieldAndHullTextValue,
    critical_percentage: TextValue,
    ticks: ShieldAndHullTextCount,
    ticks_per_second: ShieldAndHullTextValue,
    ticks_percentage: ShieldAndHullTextValue,
    pub source_ticks: Vec<HealTick>,
}

pub type HealTable = MetricsTable<HealTablePartData>;
pub type HealTablePart = MetricsTablePart<HealTablePartData>;

impl HealTable {
    pub fn empty() -> Self {
        Self::empty_base(COLUMNS)
    }

    pub fn new(
        settings: &Settings,
        combat: &Combat,
        heal_group: impl FnMut(&Player) -> &HealGroup,
    ) -> Self {
        Self::new_base(
            settings,
            COLUMNS,
            combat,
            heal_group,
            HealTablePartData::new,
        )
    }
}

impl HealTablePart {
    pub fn total_heal(&self) -> f64 {
        self.total_heal.all.value.unwrap()
    }
}

impl HealTablePartData {
    fn new(
        settings: &Settings,
        group: &HealGroup,
        combat: &Combat,
        number_formatter: &mut NumberFormatter,
    ) -> Self {
        let more_decimals = settings.general.more_decimals;
        Self {
            total_heal: ShieldAndHullTextValue::new(
                &group.total_heal,
                if more_decimals { 2 } else { 0 },
                number_formatter,
            ),
            hps: ShieldAndHullTextValue::new(
                &group.hps,
                if more_decimals { 2 } else { 0 },
                number_formatter,
            ),
            heal_percentage: ShieldAndHullTextValue::option(
                &group.heal_percentage,
                if more_decimals { 3 } else { 2 },
                number_formatter,
            ),
            average_heal: ShieldAndHullTextValue::option(
                &group.average_heal,
                if more_decimals { 2 } else { 0 },
                number_formatter,
            ),
            critical_percentage: TextValue::option(
                group.critical_percentage,
                if more_decimals { 3 } else { 2 },
                number_formatter,
            ),
            ticks: ShieldAndHullTextCount::new(&group.heal_metrics.ticks),
            ticks_per_second: ShieldAndHullTextValue::new(
                &group.ticks_per_second,
                if more_decimals { 3 } else { 1 },
                number_formatter,
            ),
            ticks_percentage: ShieldAndHullTextValue::option(
                &group.ticks_percentage,
                if more_decimals { 3 } else { 2 },
                number_formatter,
            ),
            source_ticks: group.ticks.get(&combat.heal_ticks_manger).to_vec(),
        }
    }
}
