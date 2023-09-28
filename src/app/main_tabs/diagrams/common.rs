use std::{ops::RangeInclusive, sync::Arc};

use educe::Educe;
use egui_plot::*;

use crate::{
    analyzer::{HealTick, Hit, SpecificHit, ValueFlags},
    helpers::number_formatting::NumberFormatter,
};

#[derive(Clone)]
pub struct PreparedDataSet<T: PreparedValue> {
    pub name: String,
    pub all_per_second: f64,
    pub total_value: f64,
    pub values: Arc<[PreparedPoint<T>]>,
    pub start_time_s: f64,
    pub duration_s: f64,
}

pub type PreparedDamageDataSet = PreparedDataSet<PreparedHitValue>;
pub type PreparedHealDataSet = PreparedDataSet<PreparedHealValue>;

#[derive(Educe)]
#[educe(Deref, DerefMut)]
pub struct PreparedPoint<T: PreparedValue> {
    #[educe(Deref, DerefMut)]
    pub value: T,
    pub time_millis: u32, // offset to start of combat
}

pub type PreparedHit = PreparedPoint<PreparedHitValue>;
pub type PreparedHealTick = PreparedPoint<PreparedHealValue>;

#[derive(Clone, Copy)]
pub struct PreparedHitValue {
    pub damage: f64,
    pub hull_damage: f64,
    pub shield_damage: f64,
    pub base_damage: f64,
    pub drain_damage: f64,
}

#[derive(Clone, Copy)]
pub struct PreparedHealValue {
    pub heal: f64,
}

pub trait PreparedValue: Clone + 'static {
    fn value(&self) -> f64;
    fn merge(&mut self, other: &Self);
}

impl<T: PreparedValue> PreparedDataSet<T> {
    pub fn base_new(
        name: &str,
        all_per_second: f64,
        total_value: f64,
        values: impl Iterator<Item = impl Into<PreparedPoint<T>>>,
    ) -> Self {
        let mut values = Vec::from_iter(values.map(|h| h.into()));
        values.sort_unstable_by_key(|h| h.time_millis);
        values.dedup_by(|h1, h2| {
            if h1.time_millis != h2.time_millis {
                return false;
            }

            h2.merge(h1);
            true
        });

        let start_time_s = values.iter().map(|h| h.time_millis).min().unwrap_or(0) as f64 / 1e3;
        let end_time_s = values.iter().map(|h| h.time_millis).max().unwrap_or(0) as f64 / 1e3;

        let duration_s = end_time_s - start_time_s;

        Self {
            name: name.to_string(),
            all_per_second,
            total_value,
            values: Arc::from(values),
            start_time_s,
            duration_s,
        }
    }
}

impl PreparedDamageDataSet {
    pub fn new<'a>(
        name: &str,
        dps: f64,
        total_damage: f64,
        hits: impl Iterator<Item = &'a Hit>,
    ) -> Self {
        Self::base_new(
            name,
            dps,
            total_damage,
            hits.filter(|h| !h.flags.contains(ValueFlags::IMMUNE)),
        )
    }
}

impl PreparedHealDataSet {
    pub fn new<'a>(
        name: &str,
        hps: f64,
        total_heal: f64,
        ticks: impl Iterator<Item = &'a HealTick>,
    ) -> Self {
        Self::base_new(name, hps, total_heal, ticks)
    }
}

impl<'a> From<&'a Hit> for PreparedHit {
    fn from(hit: &'a Hit) -> Self {
        match hit.specific {
            SpecificHit::Shield { .. } => Self {
                value: PreparedHitValue {
                    damage: hit.damage,
                    shield_damage: hit.damage,
                    hull_damage: 0.0,
                    base_damage: 0.0,
                    drain_damage: 0.0,
                },
                time_millis: hit.time_millis,
            },
            SpecificHit::ShieldDrain => Self {
                value: PreparedHitValue {
                    damage: hit.damage,
                    shield_damage: hit.damage,
                    hull_damage: 0.0,
                    base_damage: 0.0,
                    drain_damage: hit.damage,
                },
                time_millis: hit.time_millis,
            },
            SpecificHit::Hull { base_damage } => Self {
                value: PreparedHitValue {
                    damage: hit.damage,
                    shield_damage: 0.0,
                    hull_damage: hit.damage,
                    base_damage,
                    drain_damage: 0.0,
                },
                time_millis: hit.time_millis,
            },
        }
    }
}

impl PreparedValue for PreparedHitValue {
    fn value(&self) -> f64 {
        self.damage
    }

    fn merge(&mut self, other: &Self) {
        self.damage += other.damage;
        self.shield_damage += other.shield_damage;
        self.hull_damage += other.hull_damage;
        self.base_damage += other.base_damage;
        self.drain_damage += other.drain_damage;
    }
}

impl<'a> From<&'a HealTick> for PreparedHealTick {
    fn from(tick: &'a HealTick) -> Self {
        Self {
            value: PreparedHealValue { heal: tick.amount },
            time_millis: tick.time_millis,
        }
    }
}

impl PreparedValue for PreparedHealValue {
    fn value(&self) -> f64 {
        self.heal
    }

    fn merge(&mut self, other: &Self) {
        self.heal += other.heal;
    }
}

pub fn seconds_to_millis(seconds: f64) -> u32 {
    (seconds * 1e3).round() as _
}

pub fn millis_to_seconds(millis: u32) -> f64 {
    millis as f64 * (1.0 / 1e3)
}

pub fn format_axis(value: f64, _: usize, _: &RangeInclusive<f64>) -> String {
    if value < 0.0 {
        return String::new();
    }
    let mut formatter = NumberFormatter::new();
    formatter.format(value, 0)
}

pub fn format_element(bar: &Bar, _: &BarChart) -> String {
    let mut formatter = NumberFormatter::new();
    format!("{}\n{}", bar.name, formatter.format(bar.value, 2))
}

pub fn time_slices<'a, T: PreparedValue>(
    data: &'a PreparedDataSet<T>,
    time_slice: f64,
) -> impl Iterator<Item = (f64, &'a [PreparedPoint<T>])> + 'a {
    let time_slice_m = seconds_to_millis(time_slice);
    let first_time_slice = seconds_to_millis(data.start_time_s) / time_slice_m;
    let mut time_slice_end = first_time_slice + time_slice_m;
    let mut values = &*data.values;
    let sliced_values = std::iter::from_fn(move || {
        if values.len() == 0 {
            return None;
        }
        let slice_end = values
            .iter()
            .take_while(|v| v.time_millis < time_slice_end)
            .count();
        let slice = &values[0..slice_end];
        let center = millis_to_seconds(time_slice_end - time_slice_m / 2);
        values = &values[slice_end..];
        time_slice_end += time_slice_m;
        Some((center, slice))
    });

    sliced_values
}
