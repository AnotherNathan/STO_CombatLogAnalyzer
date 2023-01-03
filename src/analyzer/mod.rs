use std::{
    collections::HashMap,
    fmt::Write,
    ops::{Add, Range},
    path::{Path, PathBuf},
};

use arrayvec::ArrayVec;
use chrono::{Duration, NaiveDateTime};
use eframe::epaint::ahash::HashSet;
use itertools::Itertools;
use log::{info, warn};
use rustc_hash::{FxHashMap, FxHashSet};

mod parser;
pub mod settings;

use self::{parser::*, settings::*};

pub struct Analyzer {
    parser: Parser,
    combat_separation_time: Duration,
    settings: AnalysisSettings,
    combats: Vec<Combat>,
}

#[derive(Clone, Debug)]
pub struct Combat {
    pub names: FxHashSet<String>,
    pub time: Range<NaiveDateTime>,
    pub players: FxHashMap<String, Player>,
    pub log_pos: Option<Range<u64>>,
}

#[derive(Clone, Debug)]
pub struct Player {
    combat_time: Option<Range<NaiveDateTime>>,
    pub damage_source: DamageGroup,
}

#[derive(Clone, Debug)]
pub struct DamageGroup {
    pub name: String,
    pub total_damage: f64,
    pub total_shield_damage: f64,
    pub total_hull_damage: f64,
    pub max_one_hit: MaxOneHit,
    pub average_hit: f64,
    pub critical_chance: f64,
    pub flanking: f64,
    pub dps: f64,
    pub shield_dps: f64,
    pub hull_dps: f64,
    pub damage_percentage: f64,
    hull_hits: Vec<Hit>,
    shield_hits: Vec<f64>,

    is_pool: bool,

    pub sub_groups: FxHashMap<String, DamageGroup>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Hit {
    pub damage: f64,
    pub flags: ValueFlags,
}

#[derive(Clone, Debug, Default)]
pub struct MaxOneHit {
    pub name: String,
    pub damage: f64,
}

impl Analyzer {
    pub fn new(settings: AnalysisSettings) -> Option<Self> {
        Some(Self {
            parser: Parser::new(settings.combatlog_file())?,
            combat_separation_time: Duration::seconds(settings.combat_separation_time_seconds as _),
            settings,
            combats: Default::default(),
        })
    }

    pub fn update(&mut self) {
        let mut first_modified_combat = None;
        loop {
            let record = match self.parser.parse_next() {
                Ok(record) => record,
                Err(RecordError::EndReached) => break,
                Err(RecordError::InvalidRecord(invalid_record)) => {
                    warn!("failed to parse record: {}", invalid_record);
                    continue;
                }
            };

            let Entity::Player { full_name, .. } = &record.source else{
                continue;
            };

            let combat = match self.combats.last_mut() {
                Some(combat)
                    if record.time.signed_duration_since(combat.time.end)
                        > self.combat_separation_time =>
                {
                    self.combats.push(Combat::new(&record));
                    self.combats.last_mut().unwrap()
                }
                Some(combat) => combat,
                None => {
                    self.combats.push(Combat::new(&record));
                    self.combats.last_mut().unwrap()
                }
            };

            combat.update_meta_data(&record, &self.settings);

            let player = match combat.players.get_mut(*full_name) {
                Some(player) => player,
                None => {
                    let player = Player::new(&record);
                    combat
                        .players
                        .insert(player.damage_source.name.to_string(), player);
                    combat.players.get_mut(*full_name).unwrap()
                }
            };

            player.add_value(&record, &self.settings);
            first_modified_combat.get_or_insert(self.combats.len() - 1);
        }

        if let Some(first_modified_combat) = first_modified_combat {
            self.combats[first_modified_combat..]
                .iter_mut()
                .for_each(|p| p.recalculate_metrics());
        }
    }

    pub fn result(&self) -> &Vec<Combat> {
        &self.combats
    }

    pub fn settings(&self) -> &AnalysisSettings {
        &self.settings
    }
}

impl Combat {
    fn new(start_record: &Record) -> Self {
        Self {
            time: start_record.time..start_record.time,
            names: Default::default(),
            players: Default::default(),
            log_pos: start_record.log_pos.clone(),
        }
    }

    pub fn identifier(&self) -> String {
        let date_times = format!(
            "{} {} - {}",
            self.time.start.date(),
            self.time.start.time().format("%T"),
            self.time.end.time().format("%T")
        );

        if self.names.len() == 0 {
            return format!("Combat | {}", date_times);
        }

        let name = self.names.iter().join(", ");
        format!("{} | {}", name, date_times)
    }

    fn recalculate_metrics(&mut self) {
        self.players
            .values_mut()
            .for_each(|p| p.recalculate_metrics());

        let total_damage = self
            .players
            .values()
            .map(|p| p.damage_source.total_damage)
            .sum();
        self.players
            .values_mut()
            .for_each(|p| p.recalculate_damage_percentage(total_damage));
    }

    fn update_meta_data(&mut self, record: &Record, settings: &AnalysisSettings) {
        self.update_names(record, settings);
        self.update_time(record);
        self.update_log_pos(record);
    }

    fn update_names(&mut self, record: &Record, settings: &AnalysisSettings) {
        settings
            .combat_name_rules
            .iter()
            .filter(|r| r.matches(record))
            .for_each(|r| {
                if !self.names.contains(&r.name) {
                    self.names.insert(r.name.clone());
                }
            });
    }

    fn update_time(&mut self, record: &Record) {
        self.time.end = record.time;
    }

    fn update_log_pos(&mut self, record: &Record) {
        if let (Some(log_pos), Some(record_log_pos)) =
            (self.log_pos.as_mut(), record.log_pos.as_ref())
        {
            log_pos.end = record_log_pos.end;
        }
    }
}

impl Player {
    fn new(record: &Record) -> Self {
        let Entity::Player{full_name,..} = record.source else{
            panic!("record source is not a player")
        };
        Self {
            combat_time: None,
            damage_source: DamageGroup::new(full_name, true),
        }
    }

    fn add_value(&mut self, record: &Record, settings: &AnalysisSettings) {
        if record.value.get() == 0.0 {
            return;
        }

        match record.value {
            RecordValue::Damage(damage) => {
                match (&record.sub_source, &record.target) {
                    (Entity::None, _) | (_, Entity::None) => {
                        self.add_and_group_up_damage(
                            &[record.value_name],
                            record,
                            damage,
                            settings,
                        );
                    }
                    (
                        Entity::NonPlayer { name, .. }
                        | Entity::Player {
                            full_name: name, ..
                        },
                        _,
                    ) => {
                        self.add_and_group_up_pet_or_summon_damage(name, record, damage, settings);
                    }
                    _ => warn!(
                        "encountered unexpected sub source + target combo: {}",
                        record.raw
                    ),
                }

                self.update_combat_time(record);
            }
            RecordValue::Heal(_) => (),
        }
    }

    fn add_and_group_up_pet_or_summon_damage(
        &mut self,
        pet_or_summon_name: &str,
        record: &Record,
        damage: Value,
        settings: &AnalysisSettings,
    ) {
        if settings
            .summon_and_pet_grouping_revers_rules
            .iter()
            .any(|r| r.enabled && r.matches(record))
        {
            self.add_and_group_up_damage(
                &[record.value_name, pet_or_summon_name],
                record,
                damage,
                settings,
            );
        } else {
            self.add_and_group_up_damage(
                &[pet_or_summon_name, record.value_name],
                record,
                damage,
                settings,
            );
        }
    }

    fn add_and_group_up_damage(
        &mut self,
        path: &[&str],
        record: &Record,
        damage: Value,
        settings: &AnalysisSettings,
    ) {
        if let Some(rule) = settings
            .custom_group_rules
            .iter()
            .find(|r| r.matches(record))
        {
            let mut grouped_path = ArrayVec::<_, 3>::new();
            grouped_path.push(rule.name.as_str());
            grouped_path.try_extend_from_slice(path).unwrap();
            self.damage_source
                .add_damage(&grouped_path, damage, record.value_flags);
        } else {
            self.damage_source
                .add_damage(path, damage, record.value_flags);
        }
    }

    fn update_combat_time(&mut self, record: &Record) {
        let combat_time = self
            .combat_time
            .get_or_insert_with(|| record.time..record.time);
        combat_time.end = record.time;
    }

    fn recalculate_metrics(&mut self) {
        let combat_duration = self
            .combat_time
            .as_ref()
            .map(|t| t.end.signed_duration_since(t.start))
            .unwrap_or(Duration::max_value());
        let combat_duration = combat_duration.to_std().unwrap().as_secs_f64();
        self.damage_source.recalculate_metrics(combat_duration);
    }

    fn recalculate_damage_percentage(&mut self, parent_total_damage: f64) {
        self.damage_source
            .recalculate_damage_percentage(parent_total_damage);
    }
}

impl DamageGroup {
    fn new(name: &str, is_pool: bool) -> Self {
        Self {
            name: name.to_string(),
            total_damage: 0.0,
            total_shield_damage: 0.0,
            total_hull_damage: 0.0,
            max_one_hit: MaxOneHit::default(),
            average_hit: 0.0,
            critical_chance: 0.0,
            flanking: 0.0,
            hull_hits: Vec::new(),
            shield_hits: Vec::new(),
            dps: 0.0,
            shield_dps: 0.0,
            hull_dps: 0.0,
            damage_percentage: 0.0,
            sub_groups: Default::default(),
            is_pool,
        }
    }

    pub fn shield_hits(&self) -> usize {
        self.shield_hits.len()
    }

    pub fn hull_hits(&self) -> usize {
        self.hull_hits.len()
    }

    pub fn hits(&self) -> usize {
        self.shield_hits() + self.hull_hits()
    }

    fn recalculate_metrics(&mut self, combat_duration: f64) {
        self.max_one_hit.reset();
        self.total_hull_damage = 0.0;
        self.total_shield_damage = 0.0;

        let mut crits_count = 0;
        let mut flanks_count = 0;

        if self.sub_groups.len() > 0 {
            self.shield_hits.clear();
            self.hull_hits.clear();

            for sub_group in self.sub_groups.values_mut() {
                sub_group.recalculate_metrics(combat_duration);
                self.shield_hits.extend_from_slice(&sub_group.shield_hits);
                self.hull_hits.extend_from_slice(&sub_group.hull_hits);
                self.max_one_hit
                    .update(&sub_group.max_one_hit.name, sub_group.max_one_hit.damage);
            }
        }

        for hit in self.hull_hits.iter() {
            self.max_one_hit.update(&self.name, hit.damage);
            self.total_hull_damage += hit.damage;

            if hit.flags.contains(ValueFlags::CRITICAL) {
                crits_count += 1;
            }

            if hit.flags.contains(ValueFlags::FLANK) {
                flanks_count += 1;
            }
        }

        for hit in self.shield_hits.iter().copied() {
            self.max_one_hit.update(&self.name, hit);
            self.total_shield_damage += hit;
        }

        self.total_damage = self.total_hull_damage + self.total_shield_damage;

        let average_hit = if self.hits() == 0 {
            0.0
        } else {
            self.total_damage / self.hits() as f64
        };
        let critical_chance = if crits_count == 0 {
            0.0
        } else {
            crits_count as f64 / self.hull_hits() as f64
        };
        let flanking = if flanks_count == 0 {
            0.0
        } else {
            flanks_count as f64 / self.hull_hits() as f64
        };

        self.average_hit = average_hit;
        self.critical_chance = critical_chance * 100.0;
        self.flanking = flanking * 100.0;
        self.dps = self.total_damage / combat_duration.max(1.0); // avoid absurd high numbers
        self.shield_dps = self.total_shield_damage / combat_duration.max(1.0); // avoid absurd high numbers
        self.hull_dps = self.total_hull_damage / combat_duration.max(1.0); // avoid absurd high numbers
    }

    fn recalculate_damage_percentage(&mut self, parent_total_damage: f64) {
        self.damage_percentage = if self.total_damage == 0.0 {
            0.0
        } else {
            self.total_damage / parent_total_damage * 100.0
        };
        self.sub_groups
            .values_mut()
            .for_each(|s| s.recalculate_damage_percentage(self.total_damage));
    }

    fn add_damage(&mut self, path: &[&str], damage: Value, flags: ValueFlags) {
        if path.len() == 1 {
            let sub_source = self.get_non_pool_sub_group(path[0]);
            match damage {
                Value::Shield(shield_damage) => sub_source.shield_hits.push(shield_damage),
                Value::Hull(hull_damage) => sub_source.hull_hits.push(Hit {
                    damage: hull_damage,
                    flags,
                }),
            }

            return;
        }

        let sub_source = self.get_pool_sub_group(path.first().unwrap());
        sub_source.add_damage(&path[1..], damage, flags);
    }

    fn get_non_pool_sub_group(&mut self, sub_group: &str) -> &mut Self {
        let candidate = self.get_sub_group_or_create_non_pool(sub_group);
        if !candidate.is_pool {
            return candidate;
        }

        candidate.get_non_pool_sub_group(sub_group)
    }

    fn get_pool_sub_group(&mut self, sub_group: &str) -> &mut Self {
        let candidate = self.sub_groups.get(sub_group);
        if candidate.map(|c| c.is_pool).unwrap_or(false) {
            return self.get_sub_group_or_create_non_pool(sub_group);
        }

        // make a new pool and move the non pool sub group on there
        let mut pool = Self::new(sub_group, true);
        if let Some(non_pool_sub_group) = self.sub_groups.remove(sub_group) {
            pool.sub_groups
                .insert(sub_group.to_string(), non_pool_sub_group);
        }
        self.sub_groups.insert(sub_group.to_string(), pool);
        self.get_pool_sub_group(sub_group)
    }

    fn get_sub_group_or_create_non_pool(&mut self, sub_group: &str) -> &mut Self {
        if !self.sub_groups.contains_key(sub_group) {
            self.sub_groups
                .insert(sub_group.to_string(), Self::new(sub_group, false));
        }

        self.sub_groups.get_mut(sub_group).unwrap()
    }
}

impl MaxOneHit {
    fn update(&mut self, name: &str, damage: f64) {
        if self.damage < damage {
            self.damage = damage;
            self.name.clear();
            self.name.write_str(name).unwrap();
        }
    }

    fn reset(&mut self) {
        self.name.clear();
        self.damage = Default::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "manual test"]
    fn analyze_log() {
        let mut analyzer = Analyzer::new(AnalysisSettings {
            combatlog_file:
                r"D:\Games\Star Trek Online_en\Star Trek Online\Live\logs\GameClient\combatlog.log"
                    .to_string(),
            ..Default::default()
        })
        .unwrap();

        analyzer.update();
        let result = analyzer.result();
        let combats: Vec<_> = result.iter().map(|c| c.identifier()).collect();
        println!("combats: {:?}", combats);
    }
}
