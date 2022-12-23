use std::{
    collections::HashMap,
    fmt::Write,
    ops::{Add, Range},
    path::{Path, PathBuf},
};

use arrayvec::ArrayVec;
use chrono::{Duration, NaiveDateTime};
use log::warn;
use rustc_hash::FxHashMap;

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
    pub identifier: String,
    pub time: Range<NaiveDateTime>,
    pub players: FxHashMap<String, Player>,
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
    pub max_one_hit: MaxOneHit,
    pub average_hit: f64,
    pub critical_chance: f64,
    pub flanking: f64,
    pub dps: f64,
    hits: Vec<Hit>,

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
    pub source: String,
    pub hit: Hit,
}

impl Analyzer {
    pub fn new(settings: AnalysisSettings) -> Option<Self> {
        Some(Self {
            parser: Parser::new(&PathBuf::from(&settings.combatlog_file))?,
            combat_separation_time: Duration::seconds(settings.combat_separation_time_seconds as _),
            settings,
            combats: Default::default(),
        })
    }

    pub fn update(&mut self) {
        let before_update_combat_count = self.combats.len();
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
                    self.combats.push(Combat::new(record.time));
                    self.combats.last_mut().unwrap()
                }
                Some(combat) => combat,
                None => {
                    self.combats.push(Combat::new(record.time));
                    self.combats.last_mut().unwrap()
                }
            };
            combat.update_time(record.time);

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
        }

        self.combats[before_update_combat_count..]
            .iter_mut()
            .for_each(|p| p.recalculate_values());
    }

    pub fn build_result(&self) -> Vec<Combat> {
        self.combats.clone()
    }
}

impl Combat {
    fn new(start_time: NaiveDateTime) -> Self {
        Self {
            time: start_time..start_time,
            identifier: String::new(),
            players: Default::default(),
        }
    }

    fn recalculate_values(&mut self) {
        self.identifier.clear();
        write!(
            &mut self.identifier,
            "{} - {}",
            self.time.start, self.time.end
        )
        .unwrap();
        self.players
            .values_mut()
            .for_each(|p| p.recalculate_values());
    }

    fn update_time(&mut self, end_time: NaiveDateTime) {
        self.time.end = end_time;
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
        match record.value {
            RecordValue::ShieldDamage(damage) | RecordValue::HullDamage(damage) => {
                match (&record.sub_source, &record.target) {
                    (Entity::None, _) | (_, Entity::None) => {
                        self.add_and_group_up_damage(
                            &[record.value_name],
                            record,
                            damage,
                            settings,
                        );
                    }
                    (Entity::NonPlayer { name, .. }, _) => {
                        self.add_and_group_up_pet_or_summon_damage(name, record, damage, settings);
                    }
                    _ => warn!(
                        "encountered unexpected sub source + target combo: {:?}; {:?}",
                        record.sub_source, record.target
                    ),
                }

                self.update_combat_time(record);
            }
            RecordValue::ShieldHeal(_) | RecordValue::HullHeal(_) => (),
        }
    }

    fn add_and_group_up_pet_or_summon_damage(
        &mut self,
        pet_or_summon_name: &str,
        record: &Record,
        damage: f64,
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
        damage: f64,
        settings: &AnalysisSettings,
    ) {
        if let Some(rule) = settings
            .custom_group_rules
            .iter()
            .find(|r| r.enabled && r.match_rule.matches(record))
        {
            let mut grouped_path = ArrayVec::<_, 3>::new();
            grouped_path.push(rule.group_name.as_str());
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

    fn recalculate_values(&mut self) {
        let combat_duration = self
            .combat_time
            .as_ref()
            .map(|t| t.end.signed_duration_since(t.start))
            .unwrap_or(Duration::max_value());
        let combat_duration = combat_duration.to_std().unwrap().as_secs_f64();
        self.damage_source.recalculate_metrics(combat_duration);
    }
}

impl DamageGroup {
    fn new(name: &str, is_pool: bool) -> Self {
        Self {
            name: name.to_string(),
            total_damage: 0.0,
            max_one_hit: MaxOneHit::default(),
            average_hit: 0.0,
            critical_chance: 0.0,
            flanking: 0.0,
            hits: Vec::new(),
            dps: 0.0,
            sub_groups: Default::default(),
            is_pool,
        }
    }

    fn recalculate_metrics(&mut self, combat_duration: f64) {
        self.max_one_hit.reset();

        self.total_damage = 0.0;
        let mut crits_count = 0;
        let mut flanks_count = 0;

        if self.sub_groups.len() > 0 {
            self.hits.clear();

            for sub_group in self.sub_groups.values_mut() {
                sub_group.recalculate_metrics(combat_duration);
                self.hits.extend_from_slice(&sub_group.hits);
                self.max_one_hit
                    .update(&sub_group.max_one_hit.source, &sub_group.max_one_hit.hit);
            }
        }

        for hit in self.hits.iter() {
            self.max_one_hit.update(&self.name, hit);
            self.total_damage += hit.damage as f64;

            if hit.flags.contains(ValueFlags::CRITICAL) {
                crits_count += 1;
            }

            if hit.flags.contains(ValueFlags::FLANK) {
                flanks_count += 1;
            }
        }

        let average_hit = self.total_damage / self.hits.len() as f64;
        let critical_chance = crits_count as f64 / self.hits.len() as f64;
        let flanking = flanks_count as f64 / self.hits.len() as f64;

        self.average_hit = average_hit;
        self.critical_chance = critical_chance * 100.0;
        self.flanking = flanking;
        self.dps = self.total_damage / combat_duration;
    }

    fn add_damage(&mut self, path: &[&str], damage: f64, flags: ValueFlags) {
        if path.len() == 1 {
            let sub_source = self.get_non_pool_sub_group(path[0]);
            sub_source.hits.push(Hit { damage, flags });
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
    fn update(&mut self, source: &str, hit: &Hit) {
        if self.hit.damage < hit.damage {
            self.hit = *hit;
            self.source.clear();
            self.source.write_str(source).unwrap();
        }
    }

    fn reset(&mut self) {
        self.source.clear();
        self.hit = Default::default();
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
        let result = analyzer.build_result();
        let combats: Vec<_> = result.iter().map(|c| &c.identifier).collect();
        println!("combats: {:?}", combats);
    }
}
