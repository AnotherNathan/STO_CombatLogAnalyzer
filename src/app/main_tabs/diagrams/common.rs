use std::{ops::RangeInclusive, sync::Arc};

use eframe::egui::plot::*;

use crate::{analyzer::Hit, helpers::number_formatting::NumberFormatter};

#[derive(Clone)]
pub struct PreparedDamageDataSet {
    pub name: String,
    pub dps: f64,
    pub total_damage: f64,
    pub hits: Arc<[Hit]>,
    pub start_time_s: f64,
    pub duration_s: f64,
}

impl PreparedDamageDataSet {
    pub fn new<'a>(
        name: &str,
        dps: f64,
        total_damage: f64,
        hits: impl Iterator<Item = &'a Hit>,
    ) -> PreparedDamageDataSet {
        let mut hits = Vec::from_iter(hits.copied());
        hits.sort_unstable_by_key(|h| h.time_millis);
        hits.dedup_by(|h1, h2| {
            if h1.time_millis != h2.time_millis {
                return false;
            }

            h2.damage += h1.damage;
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

pub fn format_label(name: &str, point: &PlotPoint) -> String {
    if point.x < 0.0 || point.y < 0.0 {
        return String::new();
    }

    let mut formatter = NumberFormatter::new();
    let x = formatter.format(point.x, 2);
    let y = formatter.format(point.y, 2);
    format!("{}\nDPS: {}\nTime: {}", name, y, x)
}

pub fn format_element(bar: &Bar, _: &BarChart) -> String {
    let mut formatter = NumberFormatter::new();
    format!("{}\n{}", bar.name, formatter.format(bar.value, 2))
}
