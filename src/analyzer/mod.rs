use std::{fmt::Write, ops::Range};

use chrono::{Duration, NaiveDateTime};
use itertools::Itertools;
use log::warn;
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

mod parser;
pub mod settings;

use self::{parser::*, settings::*};

pub struct Analyzer {
    parser: Parser,
    combat_separation_time: Duration,
    settings: AnalysisSettings,
    combats: Vec<Combat>,
}

type Players = FxHashMap<String, Player>;
type GroupingPath<'a> = SmallVec<[&'a str; 8]>;

#[derive(Clone, Debug)]
pub struct Combat {
    pub names: FxHashSet<String>,
    pub combat_time: Option<Range<NaiveDateTime>>,
    pub active_time: Range<NaiveDateTime>,
    pub total_damage_out: TotalDamage,
    pub total_damage_in: TotalDamage,
    pub players: Players,
    pub log_pos: Option<Range<u64>>,
    pub total_deaths: u64,
    pub total_kills: u64,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub combat_time: Option<Range<NaiveDateTime>>,
    pub active_time: Option<Range<NaiveDateTime>>,
    pub damage_out: DamageGroup,
    pub damage_in: DamageGroup,
    pub deaths: u64,
    pub kills: u64,
}

#[derive(Clone, Debug)]
pub struct DamageGroup {
    pub name: String,
    pub total_damage: TotalDamage,
    pub max_one_hit: MaxOneHit,
    pub average_hit: f64,
    pub critical_chance: f64,
    pub flanking: f64,
    pub dps: f64,
    pub shield_dps: f64,
    pub hull_dps: f64,
    pub damage_percentage: f64,
    pub hull_hits: Vec<Hit>,
    pub shield_hits: Vec<Hit>,

    is_pool: bool,

    pub sub_groups: FxHashMap<String, DamageGroup>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TotalDamage {
    pub all: f64,
    pub shield: f64,
    pub hull: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Hit {
    pub damage: f64,
    pub flags: ValueFlags,
    pub times_millis: u32, // offset to start of combat
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
            match self.process_next_record(&mut first_modified_combat) {
                Ok(_) => (),
                Err(RecordError::EndReached) => break,
                Err(RecordError::InvalidRecord(invalid_record)) => {
                    warn!("failed to parse record: {}", invalid_record);
                }
            }
        }

        if let Some(first_modified_combat) = first_modified_combat {
            self.combats[first_modified_combat..]
                .iter_mut()
                .for_each(|p| p.recalculate_metrics());
        }
    }

    fn process_next_record(
        &mut self,
        first_modified_combat: &mut Option<usize>,
    ) -> Result<(), RecordError> {
        let record = self.parser.parse_next()?;

        match self.combats.last_mut() {
            Some(combat)
                if record.time.signed_duration_since(combat.active_time.end)
                    > self.combat_separation_time =>
            {
                self.combats.push(Combat::new(&record));
            }
            None => {
                self.combats.push(Combat::new(&record));
            }
            _ => (),
        }
        first_modified_combat.get_or_insert(self.combats.len() - 1);
        let combat = self.combats.last_mut().unwrap();

        combat.update_meta_data(&record, &self.settings);

        if record.value.get() == 0.0 {
            return Ok(());
        }

        let combat_start_offset_millis = record
            .time
            .signed_duration_since(combat.active_time.start)
            .num_milliseconds() as u32;

        if let Entity::Player { full_name, .. } = &record.source {
            let player = combat.get_player(full_name);
            player.add_out_value(&record, combat_start_offset_millis, &self.settings);
        }

        if let Entity::Player { full_name, .. } = &record.target {
            let player = combat.get_player(full_name);
            player.add_in_value(&record, combat_start_offset_millis, &self.settings);
        }

        Ok(())
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
        let time = start_record.time..start_record.time;
        Self {
            combat_time: if start_record.is_player_out_damage() {
                Some(time.clone())
            } else {
                None
            },
            active_time: time,
            names: Default::default(),
            players: Default::default(),
            log_pos: start_record.log_pos.clone(),
            total_damage_out: Default::default(),
            total_damage_in: Default::default(),
            total_kills: 0,
            total_deaths: 0,
        }
    }

    fn get_player(&mut self, full_name: &str) -> &mut Player {
        if !self.players.contains_key(full_name) {
            let player = Player::new(full_name);
            self.players
                .insert(player.damage_out.name.to_string(), player);
        }
        self.players.get_mut(full_name).unwrap()
    }

    pub fn identifier(&self) -> String {
        let date_times = format!(
            "{} {} - {}",
            self.active_time.start.date(),
            self.active_time.start.time().format("%T"),
            self.active_time.end.time().format("%T")
        );

        let name = self.name();
        format!("{} | {}", name, date_times)
    }

    pub fn name(&self) -> String {
        if self.names.len() == 0 {
            return "Combat".to_string();
        }

        self.names.iter().join(", ")
    }

    fn recalculate_metrics(&mut self) {
        self.players
            .values_mut()
            .for_each(|p| p.recalculate_metrics());

        self.total_damage_out = self.recalculate_total_damage(|p| &p.damage_out);
        self.total_damage_in = self.recalculate_total_damage(|p| &p.damage_in);
        self.total_kills = self.players.values().map(|p| p.kills).sum();
        self.total_deaths = self.players.values().map(|p| p.deaths).sum();
        self.recalculate_damage_group_percentage(self.total_damage_out, |p| &mut p.damage_out);
        self.recalculate_damage_group_percentage(self.total_damage_in, |p| &mut p.damage_in);
    }

    fn recalculate_total_damage(
        &self,
        mut group: impl FnMut(&Player) -> &DamageGroup,
    ) -> TotalDamage {
        let (shield, hull, all) = self.players.values().fold((0.0, 0.0, 0.0), |(s, h, a), p| {
            let total_damage = &group(p).total_damage;
            (
                total_damage.shield + s,
                total_damage.hull + h,
                total_damage.all + a,
            )
        });
        TotalDamage { all, shield, hull }
    }

    fn recalculate_damage_group_percentage(
        &mut self,
        total_damage: TotalDamage,
        mut group: impl FnMut(&mut Player) -> &mut DamageGroup,
    ) {
        self.players
            .values_mut()
            .for_each(|p| group(p).recalculate_damage_percentage(total_damage.all));
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
        if record.is_player_out_damage() {
            let combat_time = self
                .combat_time
                .get_or_insert_with(|| record.time..record.time);
            combat_time.end = record.time;
        }
        self.active_time.end = record.time;
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
    fn new(full_name: &str) -> Self {
        Self {
            combat_time: None,
            active_time: None,
            damage_out: DamageGroup::new(full_name, true),
            damage_in: DamageGroup::new(full_name, true),
            deaths: 0,
            kills: 0,
        }
    }

    fn add_out_value(
        &mut self,
        record: &Record,
        combat_start_offset_millis: u32,
        settings: &AnalysisSettings,
    ) {
        match record.value {
            RecordValue::Damage(damage) => {
                let path = Self::build_grouping_path(record, settings);

                self.damage_out.add_damage(
                    &path,
                    damage,
                    record.value_flags,
                    combat_start_offset_millis,
                );

                self.update_combat_time(record);
                self.update_active_time(record);

                if record.value_flags.contains(ValueFlags::KILL) {
                    self.kills += 1;
                }
            }
            RecordValue::Heal(_) => (),
        }
    }

    fn add_in_value(
        &mut self,
        record: &Record,
        combat_start_offset_millis: u32,
        settings: &AnalysisSettings,
    ) {
        let source_name = record.source.name().unwrap_or("<unknown source>");
        match record.value {
            RecordValue::Damage(damage) => {
                let mut path = Self::build_grouping_path(record, settings);
                path.push(source_name);

                self.damage_in.add_damage(
                    &path,
                    damage,
                    record.value_flags,
                    combat_start_offset_millis,
                );
                self.update_active_time(record);
                if record.value_flags.contains(ValueFlags::KILL) {
                    self.deaths += 1;
                }
            }
            RecordValue::Heal(_) => (),
        }
    }

    fn build_grouping_path<'a>(
        record: &'a Record,
        settings: &'a AnalysisSettings,
    ) -> GroupingPath<'a> {
        let mut path = GroupingPath::new();

        match (&record.sub_source, &record.target) {
            (Entity::None, _) | (_, Entity::None) => {
                path.push(record.value_name);
            }

            (
                Entity::NonPlayer { name, .. }
                | Entity::Player {
                    full_name: name, ..
                },
                _,
            ) => {
                if settings
                    .summon_and_pet_grouping_revers_rules
                    .iter()
                    .any(|r| r.matches(record))
                {
                    path.extend_from_slice(&[name, record.value_name]);
                } else {
                    path.extend_from_slice(&[record.value_name, name]);
                }
            }
        }

        if let Some(rule) = settings
            .custom_group_rules
            .iter()
            .find(|r| r.matches(record))
        {
            path.push(rule.name.as_str());
        }

        path
    }

    fn update_combat_time(&mut self, record: &Record) {
        let combat_time = self
            .combat_time
            .get_or_insert_with(|| record.time..record.time);
        combat_time.end = record.time;
    }

    fn update_active_time(&mut self, record: &Record) {
        let active_time = self
            .active_time
            .get_or_insert_with(|| record.time..record.time);
        active_time.end = record.time;
    }

    fn recalculate_metrics(&mut self) {
        Self::recalculate_group_metrics(&mut self.damage_out, &self.combat_time);
        Self::recalculate_group_metrics(&mut self.damage_in, &self.active_time);
    }

    fn recalculate_group_metrics(
        group: &mut DamageGroup,
        combat_time: &Option<Range<NaiveDateTime>>,
    ) {
        let active_combat_duration = combat_time
            .as_ref()
            .map(|t| t.end.signed_duration_since(t.start))
            .unwrap_or(Duration::max_value());
        let combat_duration = active_combat_duration.to_std().unwrap().as_secs_f64();
        group.recalculate_metrics(combat_duration);
    }
}

impl DamageGroup {
    fn new(name: &str, is_pool: bool) -> Self {
        Self {
            name: name.to_string(),
            total_damage: Default::default(),
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
        self.total_damage.hull = 0.0;
        self.total_damage.shield = 0.0;

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
            self.total_damage.hull += hit.damage;

            if hit.flags.contains(ValueFlags::CRITICAL) {
                crits_count += 1;
            }

            if hit.flags.contains(ValueFlags::FLANK) {
                flanks_count += 1;
            }
        }

        for hit in self.shield_hits.iter().copied() {
            self.max_one_hit.update(&self.name, hit.damage);
            self.total_damage.shield += hit.damage;
        }

        self.total_damage.all = self.total_damage.hull + self.total_damage.shield;

        let average_hit = if self.hits() == 0 {
            0.0
        } else {
            self.total_damage.all / self.hits() as f64
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
        self.dps = self.total_damage.all / combat_duration.max(1.0); // avoid absurd high numbers
        self.shield_dps = self.total_damage.shield / combat_duration.max(1.0); // avoid absurd high numbers
        self.hull_dps = self.total_damage.hull / combat_duration.max(1.0); // avoid absurd high numbers
    }

    fn recalculate_damage_percentage(&mut self, parent_total_damage: f64) {
        self.damage_percentage = if self.total_damage.all == 0.0 {
            0.0
        } else {
            self.total_damage.all / parent_total_damage * 100.0
        };
        self.sub_groups
            .values_mut()
            .for_each(|s| s.recalculate_damage_percentage(self.total_damage.all));
    }

    fn add_damage(
        &mut self,
        path: &[&str],
        damage: Value,
        flags: ValueFlags,
        combat_start_offset_millis: u32,
    ) {
        if path.len() == 1 {
            let sub_source = self.get_non_pool_sub_group(path[0]);
            match damage {
                Value::Shield(shield_damage) => sub_source.shield_hits.push(Hit {
                    damage: shield_damage,
                    flags,
                    times_millis: combat_start_offset_millis,
                }),
                Value::Hull(hull_damage) => sub_source.hull_hits.push(Hit {
                    damage: hull_damage,
                    flags,
                    times_millis: combat_start_offset_millis,
                }),
            }

            return;
        }

        let sub_source = self.get_pool_sub_group(path.last().unwrap());
        sub_source.add_damage(
            &path[..path.len() - 1],
            damage,
            flags,
            combat_start_offset_millis,
        );
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
