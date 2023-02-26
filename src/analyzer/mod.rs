use std::{fmt::Debug, ops::Range};

use chrono::{Duration, NaiveDateTime};
use educe::Educe;
use itertools::Itertools;
use log::warn;
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

mod common;
mod damage;
mod heal;
mod parser;
pub mod settings;
pub use common::*;
pub use damage::*;
pub use heal::*;

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
    pub total_damage_out: ShieldHullValues,
    pub total_damage_in: ShieldHullValues,
    pub total_heal_in: ShieldHullValues,
    pub total_heal_out: ShieldHullValues,
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
    pub heal_out: HealGroup,
    pub heal_in: HealGroup,
    pub deaths: u64,
    pub kills: u64,
}

pub type DamageGroup = AnalysisGroup<DamageData>;
pub type HealGroup = AnalysisGroup<HealData>;

#[derive(Clone, Debug, Educe)]
#[educe(Deref, DerefMut)]
pub struct AnalysisGroup<T: Clone + Sized + Debug> {
    pub name: String,
    #[educe(Deref, DerefMut)]
    pub data: T,
    pub sub_groups: FxHashMap<String, Self>,

    is_pool: bool,
}

#[derive(Clone, Debug, Educe, Default)]
#[educe(Deref, DerefMut)]
pub struct DamageData {
    #[educe(Deref, DerefMut)]
    pub damage_metrics: DamageMetrics,
    pub max_one_hit: MaxOneHit,
    pub damage_percentage: ShieldHullOptionalValues,
    pub hits_percentage: ShieldHullOptionalValues,
    pub hits: Vec<Hit>,
    pub damage_types: FxHashSet<String>,
}

#[derive(Clone, Debug, Educe, Default)]
#[educe(Deref, DerefMut)]
pub struct HealData {
    #[educe(Deref, DerefMut)]
    pub heal_metrics: HealMetrics,

    pub heal_percentage: ShieldHullOptionalValues,

    pub ticks: Vec<HealTick>,
}

impl<T: Clone + Sized + Debug + Default> AnalysisGroup<T> {
    fn new(name: &str, is_pool: bool) -> Self {
        Self {
            name: name.to_string(),
            data: Default::default(),
            sub_groups: Default::default(),
            is_pool,
        }
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

        if let (Entity::Player { full_name, .. }, Entity::NonPlayer { .. }) =
            (&record.sub_source, &record.source)
        {
            let player = combat.get_player(full_name);
            player.add_in_value(&record, combat_start_offset_millis, &self.settings);
        }

        if let (Entity::Player { full_name, .. }, Entity::None, Entity::None) =
            (&record.source, &record.sub_source, &record.target)
        {
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
            total_heal_in: Default::default(),
            total_heal_out: Default::default(),
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

    pub fn file_identifier(&self) -> String {
        let date_times = format!(
            "{} {} - {}",
            self.active_time.start.date(),
            self.active_time.start.time().format("%H-%M-%S"),
            self.active_time.end.time().format("%H-%M-%S")
        );
        let name = self.name();
        format!("{} {}", name, date_times)
    }

    fn recalculate_metrics(&mut self) {
        self.players
            .values_mut()
            .for_each(|p| p.recalculate_metrics());

        self.total_damage_out = self
            .players
            .values()
            .map(|p| p.damage_out.total_damage)
            .sum();
        self.total_damage_in = self
            .players
            .values()
            .map(|p| p.damage_in.total_damage)
            .sum();
        self.total_heal_out = self.players.values().map(|p| p.heal_out.total_heal).sum();
        self.total_heal_in = self.players.values().map(|p| p.heal_in.total_heal).sum();
        self.total_kills = self.players.values().map(|p| p.kills).sum();
        self.total_deaths = self.players.values().map(|p| p.deaths).sum();
        let total_hits_out: ShieldHullCounts = self
            .players
            .values()
            .map(|p| p.damage_out.damage_metrics.hits)
            .sum();
        let total_hits_in: ShieldHullCounts = self
            .players
            .values()
            .map(|p| p.damage_in.damage_metrics.hits)
            .sum();
        self.recalculate_damage_group_percentage(self.total_damage_out, total_hits_out, |p| {
            &mut p.damage_out
        });
        self.recalculate_damage_group_percentage(self.total_damage_in, total_hits_in, |p| {
            &mut p.damage_in
        });
        self.recalculate_heal_group_percentage(self.total_heal_out, |p| &mut p.heal_out);
        self.recalculate_heal_group_percentage(self.total_heal_in, |p| &mut p.heal_in);
    }

    fn recalculate_damage_group_percentage(
        &mut self,
        total_damage: ShieldHullValues,
        total_hits: ShieldHullCounts,
        mut group: impl FnMut(&mut Player) -> &mut DamageGroup,
    ) {
        self.players
            .values_mut()
            .for_each(|p| group(p).recalculate_percentages(&total_damage, &total_hits));
    }

    fn recalculate_heal_group_percentage(
        &mut self,
        total_heal: ShieldHullValues,
        mut group: impl FnMut(&mut Player) -> &mut HealGroup,
    ) {
        self.players
            .values_mut()
            .for_each(|p| group(p).recalculate_percentages(&total_heal));
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
        if record.is_player_out_damage() && !record.is_immune_or_zero() {
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
            heal_out: HealGroup::new(full_name, true),
            heal_in: HealGroup::new(full_name, true),
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
        self.update_active_time(record);
        let mut path = Self::build_grouping_path(record, settings);
        match record.value {
            RecordValue::Damage(damage) if !record.is_direct_self_damage() => {
                self.damage_out.add_damage(
                    &path,
                    damage,
                    record.value_flags,
                    record.value_type,
                    combat_start_offset_millis,
                );

                self.update_combat_time(record);

                if record.value_flags.contains(ValueFlags::KILL) {
                    self.kills += 1;
                }
            }
            RecordValue::Heal(heal) => {
                let target_name = if record.is_self_directed() {
                    record.source.name().unwrap_or("<unknown target>")
                } else {
                    record
                        .target
                        .name()
                        .or_else(|| record.sub_source.name())
                        .unwrap_or("<unknown target>")
                };
                path.push(target_name);
                self.heal_out
                    .add_heal(&path, heal, record.value_flags, combat_start_offset_millis);
            }
            _ => (),
        }
    }

    fn add_in_value(
        &mut self,
        record: &Record,
        combat_start_offset_millis: u32,
        settings: &AnalysisSettings,
    ) {
        let source_name = record.source.name().unwrap_or("<unknown source>");
        let mut path = Self::build_grouping_path(record, settings);
        path.push(source_name);
        match record.value {
            RecordValue::Damage(damage) => {
                self.damage_in.add_damage(
                    &path,
                    damage,
                    record.value_flags,
                    record.value_type,
                    combat_start_offset_millis,
                );
                self.update_active_time(record);
                if record.value_flags.contains(ValueFlags::KILL) {
                    self.deaths += 1;
                }
            }
            RecordValue::Heal(heal) => {
                self.heal_in
                    .add_heal(&path, heal, record.value_flags, combat_start_offset_millis);
            }
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
                }
                | Entity::NonPlayerCharacter { name, .. },
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
        if record.is_immune_or_zero() {
            return;
        }
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
        let combat_duration = Self::metrics_duration(&self.combat_time);
        let active_duration = Self::metrics_duration(&self.combat_time);
        self.damage_out.recalculate_metrics(combat_duration);
        self.damage_in.recalculate_metrics(active_duration);
        self.heal_out.recalculate_metrics(active_duration);
        self.heal_in.recalculate_metrics(active_duration);
    }

    fn metrics_duration(time: &Option<Range<NaiveDateTime>>) -> f64 {
        let duration = time
            .as_ref()
            .map(|t| t.end.signed_duration_since(t.start))
            .unwrap_or(Duration::max_value());
        let duration = duration.to_std().unwrap().as_secs_f64();
        duration
    }
}

impl DamageGroup {
    fn recalculate_metrics(&mut self, combat_duration: f64) {
        if self.sub_groups.len() > 0 {
            self.max_one_hit.reset();
            self.hits.clear();
            self.damage_types.clear();

            for sub_group in self.sub_groups.values_mut() {
                sub_group.recalculate_metrics(combat_duration);
                self.data.hits.extend_from_slice(&sub_group.hits);
                self.data
                    .max_one_hit
                    .update(&sub_group.max_one_hit.name, sub_group.max_one_hit.damage);
                for damage_type in sub_group.damage_types.iter() {
                    if !self.data.damage_types.contains(damage_type) {
                        self.data.damage_types.insert(damage_type.clone());
                    }
                }
            }
        } else {
            self.max_one_hit = MaxOneHit::from_hits(&self.name, &self.hits);
        }

        self.damage_metrics = DamageMetrics::calculate(&self.hits, combat_duration);
    }

    fn recalculate_percentages(
        &mut self,
        parent_total_damage: &ShieldHullValues,
        parent_hits: &ShieldHullCounts,
    ) {
        self.damage_percentage =
            ShieldHullOptionalValues::percentage(&self.total_damage, parent_total_damage);
        self.hits_percentage = ShieldHullOptionalValues::percentage(
            &self.damage_metrics.hits.to_values(),
            &parent_hits.to_values(),
        );
        self.sub_groups.values_mut().for_each(|s| {
            s.recalculate_percentages(
                &self.data.damage_metrics.total_damage,
                &self.data.damage_metrics.hits,
            )
        });
    }

    fn add_damage(
        &mut self,
        path: &[&str],
        hit: BaseHit,
        flags: ValueFlags,
        damage_type: &str,
        combat_start_offset_millis: u32,
    ) {
        if path.len() == 1 {
            let sub_source = self.get_non_pool_sub_group(path[0]);
            sub_source.hits.push(hit.to_hit(combat_start_offset_millis));
            sub_source.add_damage_type_non_pool(damage_type);

            return;
        }

        let sub_source = self.get_pool_sub_group(path.last().unwrap());
        sub_source.add_damage(
            &path[..path.len() - 1],
            hit,
            flags,
            damage_type,
            combat_start_offset_millis,
        );
    }

    fn add_damage_type_non_pool(&mut self, damage_type: &str) {
        if damage_type.is_empty() {
            return;
        }

        if self.damage_types.contains(damage_type) {
            return;
        }

        if self.damage_types.contains(damage_type) {
            return;
        }

        if damage_type == "Shield" && !self.damage_types.is_empty() {
            return;
        }

        if damage_type != "Shield" && self.damage_types.contains("Shield") {
            self.damage_types.remove("Shield");
        }

        self.damage_types.insert(damage_type.to_string());
    }
}

impl HealGroup {
    fn recalculate_metrics(&mut self, combat_duration: f64) {
        if self.sub_groups.len() > 0 {
            self.ticks.clear();

            for sub_group in self.sub_groups.values_mut() {
                sub_group.recalculate_metrics(combat_duration);
                self.data.ticks.extend_from_slice(&sub_group.ticks);
            }
        }

        self.heal_metrics = HealMetrics::calculate(&self.ticks, combat_duration);
    }

    fn recalculate_percentages(&mut self, parent_total_heal: &ShieldHullValues) {
        self.heal_percentage =
            ShieldHullOptionalValues::percentage(&self.total_heal, parent_total_heal);
        self.sub_groups
            .values_mut()
            .for_each(|s| s.recalculate_percentages(&self.data.heal_metrics.total_heal));
    }

    fn add_heal(
        &mut self,
        path: &[&str],
        tick: BaseHealTick,
        flags: ValueFlags,
        combat_start_offset_millis: u32,
    ) {
        if path.len() == 1 {
            let sub_source = self.get_non_pool_sub_group(path[0]);
            sub_source
                .ticks
                .push(tick.to_tick(combat_start_offset_millis));

            return;
        }

        let sub_source = self.get_pool_sub_group(path.last().unwrap());
        sub_source.add_heal(
            &path[..path.len() - 1],
            tick,
            flags,
            combat_start_offset_millis,
        );
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
