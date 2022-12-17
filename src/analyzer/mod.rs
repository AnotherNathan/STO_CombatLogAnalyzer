use std::{collections::HashMap, fmt::Write, path::Path};

use chrono::{Duration, NaiveDateTime};
use log::warn;
use rustc_hash::FxHashMap;

use crate::parser::*;

pub struct Analyzer {
    parser: Parser,
    combat_separation_time: Duration,
    combats: Vec<Combat>,
}

#[derive(Clone, Debug)]
pub struct Combat {
    pub identifier: String,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub players: FxHashMap<String, Player>,
}

#[derive(Clone, Debug)]
pub struct Player {
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
    pub damage_source: DamageSource,
}

#[derive(Clone, Debug)]
pub struct DamageSource {
    pub name: String,
    pub total_damage: f64,
    pub max_one_hit: MaxOneHit,
    pub average_hit: f64,
    pub critical_chance: f64,
    pub flanking: f64,
    pub dps: f64,
    hits: Vec<Hit>,
    pub sub_sources: FxHashMap<String, DamageSource>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Hit {
    pub damage: f64,
    pub flags: HitFlags,
}

#[derive(Clone, Debug, Default)]
pub struct MaxOneHit {
    pub source: String,
    pub hit: Hit,
}

impl Analyzer {
    pub fn new(file_name: &Path, combat_separation_time: Duration) -> Option<Self> {
        Some(Self {
            parser: Parser::new(file_name)?,
            combat_separation_time,
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

            if record.player_handle.is_none() {
                continue;
            }

            let combat = match self.combats.last_mut() {
                Some(combat)
                    if record.time.signed_duration_since(combat.end_time)
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
            combat.update(record.time);

            let player = match combat.players.get_mut(record.entity) {
                Some(player) => player,
                None => {
                    let player = Player::new(&record);
                    combat
                        .players
                        .insert(player.damage_source.name.to_string(), player);
                    combat.players.get_mut(record.entity).unwrap()
                }
            };

            match record.pet_name {
                Some(_) => {
                    // todo pet damage
                }
                None => player.add_hit(&record),
            }
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
            start_time,
            end_time: start_time,
            identifier: String::new(),
            players: Default::default(),
        }
    }

    fn recalculate_values(&mut self) {
        self.identifier.clear();
        write!(
            &mut self.identifier,
            "{} - {}",
            self.start_time, self.end_time
        )
        .unwrap();
        self.players
            .values_mut()
            .for_each(|p| p.recalculate_values());
    }

    fn update(&mut self, end_time: NaiveDateTime) {
        self.end_time = end_time;
    }
}

impl Player {
    fn new(record: &Record) -> Self {
        Self {
            start_time: record.time,
            end_time: record.time,
            damage_source: DamageSource::new(record.entity),
        }
    }

    fn add_hit(&mut self, record: &Record) {
        self.end_time = record.time;
        self.damage_source.add_sub_source_source_hit(
            record.damage_source,
            record.damage,
            record.hit_flags,
        );
    }

    fn recalculate_values(&mut self) {
        let combat_duration = self.end_time.signed_duration_since(self.start_time);
        let combat_duration = combat_duration.to_std().unwrap().as_secs_f64();
        self.damage_source.recalculate_values(combat_duration);
    }
}

impl DamageSource {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            total_damage: 0.0,
            max_one_hit: MaxOneHit::default(),
            average_hit: 0.0,
            critical_chance: 0.0,
            flanking: 0.0,
            hits: Vec::new(),
            dps: 0.0,
            sub_sources: Default::default(),
        }
    }

    fn recalculate_values(&mut self, combat_duration: f64) {
        self.max_one_hit.reset();

        self.total_damage = 0.0;
        let mut crits_count = 0;
        let mut flanks_count = 0;

        if self.sub_sources.len() > 0 {
            self.hits.clear();

            for sub_source in self.sub_sources.values_mut() {
                sub_source.recalculate_values(combat_duration);
                self.hits.extend_from_slice(&sub_source.hits);
                self.max_one_hit
                    .update(&sub_source.max_one_hit.source, &sub_source.max_one_hit.hit);
            }
        }

        for hit in self.hits.iter() {
            self.max_one_hit.update(&self.name, hit);
            self.total_damage += hit.damage as f64;

            if hit.flags.contains(HitFlags::CRITICAL) {
                crits_count += 1;
            }

            if hit.flags.contains(HitFlags::FLANK) {
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

    fn add_sub_source_source_hit(&mut self, sub_source: &str, damage: f32, flags: HitFlags) {
        let sub_source = self.get_or_create_sub_source(sub_source);
        sub_source.hits.push(Hit {
            damage: damage as _,
            flags,
        });
    }

    fn get_or_create_sub_source(&mut self, sub_source: &str) -> &mut Self {
        if !self.sub_sources.contains_key(sub_source) {
            self.sub_sources
                .insert(sub_source.to_string(), Self::new(sub_source));
        }

        self.sub_sources.get_mut(sub_source).unwrap()
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

    pub fn to_string(&self) -> String {
        format!("{} ({})", self.hit.damage, self.source)
    }
}

mod tests {
    use std::{ops::Add, path::PathBuf};

    use super::*;

    #[test]
    #[ignore = "manual test"]
    fn analyze_log() {
        let mut analyzer = Analyzer::new(
            &PathBuf::from(
                r"D:\Games\Star Trek Online_en\Star Trek Online\Live\logs\GameClient\combatlog.log",
            ),
            Duration::minutes(1).add(Duration::seconds(30)),
        )
        .unwrap();

        analyzer.update();
        let result = analyzer.build_result();
        let combats: Vec<_> = result.iter().map(|c| &c.identifier).collect();
        println!("combats: {:?}", combats);
    }
}
