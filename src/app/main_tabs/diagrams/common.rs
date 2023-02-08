use std::{ops::RangeInclusive, sync::Arc};

use eframe::egui::plot::*;

use crate::{
    analyzer::{Hit, SpecificHit, ValueFlags},
    helpers::number_formatting::NumberFormatter,
};

#[derive(Clone)]
pub struct PreparedDamageDataSet {
    pub name: String,
    pub dps: f64,
    pub total_damage: f64,
    pub hits: Arc<[PreparedHit]>,
    pub start_time_s: f64,
    pub duration_s: f64,
}

pub struct PreparedHit {
    pub damage: f64,
    pub hull_damage: f64,
    pub shield_damage: f64,
    pub base_damage: f64,
    pub drain_damage: f64,
    pub time_millis: u32, // offset to start of combat
}

impl PreparedDamageDataSet {
    pub fn new<'a>(
        name: &str,
        dps: f64,
        total_damage: f64,
        hits: impl Iterator<Item = &'a Hit>,
    ) -> PreparedDamageDataSet {
        let mut hits = Vec::from_iter(
            hits.filter(|h| !h.flags.contains(ValueFlags::IMMUNE))
                .map(|h| PreparedHit::new(h)),
        );
        hits.sort_unstable_by_key(|h| h.time_millis);
        hits.dedup_by(|h1, h2| {
            if h1.time_millis != h2.time_millis {
                return false;
            }

            h2.merge(h1);
            true
        });

        let start_time_s = hits.iter().map(|h| h.time_millis).min().unwrap_or(0) as f64 / 1e3;
        let end_time_s = hits.iter().map(|h| h.time_millis).max().unwrap_or(0) as f64 / 1e3;

        let duration_s = end_time_s - start_time_s;

        Self {
            name: name.to_string(),
            dps,
            total_damage,
            hits: Arc::from(hits),
            start_time_s,
            duration_s,
        }
    }
}

impl PreparedHit {
    fn new(hit: &Hit) -> Self {
        match hit.specific {
            SpecificHit::Shield { .. } => Self {
                damage: hit.damage,
                shield_damage: hit.damage,
                hull_damage: 0.0,
                base_damage: 0.0,
                drain_damage: 0.0,
                time_millis: hit.time_millis,
            },
            SpecificHit::ShieldDrain => Self {
                damage: hit.damage,
                shield_damage: hit.damage,
                hull_damage: 0.0,
                base_damage: 0.0,
                drain_damage: hit.damage,
                time_millis: hit.time_millis,
            },
            SpecificHit::Hull { base_damage } => Self {
                damage: hit.damage,
                shield_damage: 0.0,
                hull_damage: hit.damage,
                base_damage,
                drain_damage: 0.0,
                time_millis: hit.time_millis,
            },
        }
    }

    fn merge(&mut self, other: &Self) {
        self.damage += other.damage;
        self.shield_damage += other.shield_damage;
        self.hull_damage += other.hull_damage;
        self.base_damage += other.base_damage;
        self.drain_damage += other.drain_damage;
    }
}

pub fn seconds_to_millis(seconds: f64) -> u32 {
    (seconds * 1e3).round() as _
}

pub fn millis_to_seconds(millis: u32) -> f64 {
    millis as f64 * (1.0 / 1e3)
}

pub fn format_axis(value: f64, _: &RangeInclusive<f64>) -> String {
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
