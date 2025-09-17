use std::{
    borrow::Cow,
    fmt::Debug,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    ops::Range,
    path::Path,
};

use chrono::{Duration, NaiveDateTime};
use educe::Educe;
use itertools::Itertools;
use log::warn;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

mod common;
mod damage;
mod groups;
mod heal;
mod name_manager;
mod parser;
pub mod settings;
mod values_manager;
pub use common::*;
pub use damage::*;
use groups::*;
pub use groups::{AnalysisGroup, DamageGroup, HealGroup};
pub use heal::*;
pub use name_manager::*;
pub use values_manager::*;

use self::{parser::*, settings::*};

pub struct Analyzer {
    parser: Parser,
    combat_separation_time: Duration,
    settings: AnalysisSettings,
    combats: Vec<Combat>,
}

type Players = NameMap<Player>;
type GroupingPath = SmallVec<[GroupPathSegment; 8]>;

#[derive(Clone, Debug)]
pub struct Combat {
    pub combat_names: FxHashMap<String, CombatName>,
    pub combat_time: Option<Range<NaiveDateTime>>,
    pub active_time: Range<NaiveDateTime>,
    pub total_damage_out: ShieldHullValues,
    pub total_damage_in: ShieldHullValues,
    pub total_heal_in: ShieldHullValues,
    pub total_heal_out: ShieldHullValues,
    pub players: Players,
    pub log_pos: Option<Range<u64>>,
    pub total_deaths: u32,
    pub total_kills: u32,
    pub name_manager: NameManager,
    pub hits_manger: HitsManager,
    pub heal_ticks_manger: HealTicksManager,
}

#[derive(Clone, Debug)]
pub struct CombatName {
    pub name: String,
    pub additional_infos: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub combat_time: Option<Range<NaiveDateTime>>,
    pub active_time: Option<Range<NaiveDateTime>>,
    pub damage_out: DamageGroup,
    pub damage_in: DamageGroup,
    pub heal_out: HealGroup,
    pub heal_in: HealGroup,
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
                .for_each(|p| p.update(&self.settings));
        }
    }

    fn process_next_record(
        &mut self,
        first_modified_combat: &mut Option<usize>,
    ) -> Result<(), RecordError<'_>> {
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

        combat.update_meta_data(&record);
        combat.update_names(&record);

        let combat_start_offset_millis = record
            .time
            .signed_duration_since(combat.active_time.start)
            .num_milliseconds() as u32;

        if let Entity::Player { full_name, .. } = &record.source {
            let player =
                Combat::get_player(&mut combat.players, combat.name_manager.handle(full_name));
            player.add_out_value(
                &record,
                combat_start_offset_millis,
                &self.settings,
                &mut combat.name_manager,
            );
        }

        if let Entity::Player { full_name, .. } = &record.target {
            let player =
                Combat::get_player(&mut combat.players, combat.name_manager.handle(full_name));
            player.add_in_value(
                &record,
                combat_start_offset_millis,
                &self.settings,
                &mut combat.name_manager,
            );
        }

        if let (Entity::Player { full_name, .. }, Entity::NonPlayer { .. }) =
            (&record.indirect_source, &record.source)
        {
            let player =
                Combat::get_player(&mut combat.players, combat.name_manager.handle(full_name));
            player.add_in_value(
                &record,
                combat_start_offset_millis,
                &self.settings,
                &mut combat.name_manager,
            );
        }

        if let (Entity::Player { full_name, .. }, Entity::None, Entity::None) =
            (&record.source, &record.indirect_source, &record.target)
        {
            let player =
                Combat::get_player(&mut combat.players, combat.name_manager.handle(full_name));
            player.add_in_value(
                &record,
                combat_start_offset_millis,
                &self.settings,
                &mut combat.name_manager,
            );
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
            combat_names: Default::default(),
            players: Default::default(),
            log_pos: start_record.log_pos.clone(),
            total_damage_out: Default::default(),
            total_damage_in: Default::default(),
            total_heal_in: Default::default(),
            total_heal_out: Default::default(),
            total_kills: 0,
            total_deaths: 0,
            name_manager: Default::default(),
            hits_manger: Default::default(),
            heal_ticks_manger: Default::default(),
        }
    }

    fn get_player(players: &mut NameMap<Player>, name: NameHandle) -> &mut Player {
        if !players.contains_key(&name) {
            let player = Player::new(name);
            players.insert(player.damage_out.name(), player);
        }
        players.get_mut(&name).unwrap()
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
        if self.combat_names.len() == 0 {
            return "Combat".to_string();
        }

        self.combat_names.values().map(|n| n.format()).join(", ")
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

    fn update(&mut self, settings: &AnalysisSettings) {
        self.update_combat_names(settings);

        self.hits_manger.clear();
        self.heal_ticks_manger.clear();
        self.players.values_mut().for_each(|p| {
            p.recalculate_metrics(&mut self.hits_manger, &mut self.heal_ticks_manger)
        });

        let players = self.players.values();

        self.total_damage_out = players.clone().map(|p| p.damage_out.total_damage).sum();
        self.total_damage_in = players.clone().map(|p| p.damage_in.total_damage).sum();
        self.total_heal_out = players.clone().map(|p| p.heal_out.total_heal).sum();
        self.total_heal_in = players.clone().map(|p| p.heal_in.total_heal).sum();
        self.total_kills = players
            .clone()
            .map(|p| p.damage_out.kills.values().copied().sum::<u32>())
            .sum();
        self.total_deaths = players
            .clone()
            .map(|p| p.damage_in.kills.values().copied().sum::<u32>())
            .sum();
        let total_hits_out: ShieldHullCounts = players
            .clone()
            .map(|p| p.damage_out.damage_metrics.hits)
            .sum();
        let total_hits_in: ShieldHullCounts = players
            .clone()
            .map(|p| p.damage_in.damage_metrics.hits)
            .sum();
        let total_heal_ticks_out = players.clone().map(|p| p.heal_out.heal_metrics.ticks).sum();
        let total_heal_ticks_in = players.clone().map(|p| p.heal_in.heal_metrics.ticks).sum();
        self.recalculate_damage_group_percentage(self.total_damage_out, total_hits_out, |p| {
            &mut p.damage_out
        });
        self.recalculate_damage_group_percentage(self.total_damage_in, total_hits_in, |p| {
            &mut p.damage_in
        });
        self.recalculate_heal_group_percentage(self.total_heal_out, total_heal_ticks_out, |p| {
            &mut p.heal_out
        });
        self.recalculate_heal_group_percentage(self.total_heal_in, total_heal_ticks_in, |p| {
            &mut p.heal_in
        });
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
        parent_ticks: ShieldHullCounts,
        mut group: impl FnMut(&mut Player) -> &mut HealGroup,
    ) {
        self.players
            .values_mut()
            .for_each(|p| group(p).recalculate_percentages(&total_heal, &parent_ticks));
    }

    fn update_meta_data(&mut self, record: &Record) {
        self.update_time(record);
        self.update_log_pos(record);
    }

    fn update_names(&mut self, record: &Record) {
        self.name_manager.insert_some(
            record.source.name(),
            NameFlags::SOURCE.set_if(NameFlags::PLAYER, record.source.is_player()),
        );
        self.name_manager.insert_some(
            record.source.unique_name(),
            NameFlags::SOURCE_UNIQUE.set_if(NameFlags::PLAYER, record.source.is_player()),
        );
        self.name_manager.insert_some(
            record.target.name(),
            NameFlags::TARGET.set_if(NameFlags::PLAYER, record.target.is_player()),
        );
        self.name_manager.insert_some(
            record.target.unique_name(),
            NameFlags::TARGET_UNIQUE.set_if(NameFlags::PLAYER, record.target.is_player()),
        );
        self.name_manager.insert_some(
            record.indirect_source.name(),
            NameFlags::INDIRECT_SOURCE
                .set_if(NameFlags::PLAYER, record.indirect_source.is_player()),
        );
        self.name_manager.insert_some(
            record.indirect_source.unique_name(),
            NameFlags::INDIRECT_SOURCE_UNIQUE
                .set_if(NameFlags::PLAYER, record.indirect_source.is_player()),
        );
        self.name_manager
            .insert(record.value_name, NameFlags::VALUE);
        self.name_manager.insert(record.value_type, NameFlags::NONE);
    }

    fn update_combat_names(&mut self, settings: &AnalysisSettings) {
        self.combat_names.clear();

        settings
            .combat_name_rules
            .iter()
            .filter(|r| self.name_manager.matches(&r.name_rule))
            .for_each(|r| {
                self.combat_names.insert(
                    r.name_rule.name.clone(),
                    CombatName::new(r, &self.name_manager),
                );
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
    fn new(full_name: NameHandle) -> Self {
        Self {
            combat_time: None,
            active_time: None,
            damage_out: DamageGroup::new_branch(GroupPathSegment::Group(full_name)),
            damage_in: DamageGroup::new_branch(GroupPathSegment::Group(full_name)),
            heal_out: HealGroup::new_branch(GroupPathSegment::Group(full_name)),
            heal_in: HealGroup::new_branch(GroupPathSegment::Group(full_name)),
        }
    }

    fn add_out_value(
        &mut self,
        record: &Record,
        combat_start_offset_millis: u32,
        settings: &AnalysisSettings,
        name_manager: &mut NameManager,
    ) {
        if settings
            .damage_out_exclusion_rules
            .iter()
            .any(|r| r.matches_record(record))
        {
            return;
        }
        self.update_active_time(record);
        let mut path = Self::build_grouping_path(record, settings, name_manager);
        let target_name = if record.is_self_directed() {
            record.source.name()
        } else {
            record
                .target
                .name()
                .or_else(|| record.indirect_source.name())
        };
        let target_name = target_name
            .map(|n| name_manager.handle(n))
            .unwrap_or_default();
        match record.value {
            RecordValue::Damage(damage) if !record.is_direct_self_damage() => {
                path.insert(0, GroupPathSegment::Group(target_name));
                self.damage_out.add_damage(
                    &path,
                    damage,
                    record.value_flags,
                    name_manager.handle(record.value_type),
                    combat_start_offset_millis,
                    name_manager,
                );

                self.update_combat_time(record);
            }
            RecordValue::Heal(heal) => {
                path.push(GroupPathSegment::Group(target_name));
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
        name_manager: &mut NameManager,
    ) {
        let source_name = record
            .source
            .name()
            .map(|n| name_manager.handle(n))
            .unwrap_or_default();
        let mut path = Self::build_grouping_path(record, settings, name_manager);
        path.push(GroupPathSegment::Group(source_name));
        match record.value {
            RecordValue::Damage(damage) => {
                self.damage_in.add_damage(
                    &path,
                    damage,
                    record.value_flags,
                    name_manager.handle(record.value_type),
                    combat_start_offset_millis,
                    name_manager,
                );
                self.update_active_time(record);
            }
            RecordValue::Heal(heal) => {
                self.heal_in
                    .add_heal(&path, heal, record.value_flags, combat_start_offset_millis);
            }
        }
    }

    fn build_grouping_path(
        record: &Record,
        settings: &AnalysisSettings,
        name_manager: &mut NameManager,
    ) -> GroupingPath {
        let mut path = GroupingPath::new();

        match (&record.indirect_source, &record.target) {
            (Entity::None, _) | (_, Entity::None) => {
                path.push(GroupPathSegment::Value(
                    name_manager.handle(record.value_name),
                ));
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
                    .indirect_source_grouping_revers_rules
                    .iter()
                    .any(|r| r.matches_record(record))
                {
                    path.extend_from_slice(&[
                        GroupPathSegment::Value(name_manager.handle(name)),
                        GroupPathSegment::Group(name_manager.handle(record.value_name)),
                    ]);
                } else {
                    path.extend_from_slice(&[
                        GroupPathSegment::Value(name_manager.handle(record.value_name)),
                        GroupPathSegment::Group(name_manager.handle(name)),
                    ]);
                }
            }
        }

        if let Some(rule) = settings
            .custom_group_rules
            .iter()
            .find(|r| r.matches_record(record))
        {
            path.push(GroupPathSegment::Group(
                name_manager.insert(rule.name.as_str(), NameFlags::NONE),
            ));
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

    fn recalculate_metrics(
        &mut self,
        hits_manager: &mut HitsManager,
        heal_ticks_manager: &mut HealTicksManager,
    ) {
        let combat_duration = Self::metrics_duration(&self.combat_time);
        let active_duration = Self::metrics_duration(&self.active_time);
        self.damage_out
            .recalculate_metrics(combat_duration, hits_manager, &mut |_, _| {});
        self.damage_in
            .recalculate_metrics(active_duration, hits_manager, &mut |_, _| {});
        self.heal_out
            .recalculate_metrics(active_duration, heal_ticks_manager, &mut |_| {});
        self.heal_in
            .recalculate_metrics(active_duration, heal_ticks_manager, &mut |_| {});
    }

    fn metrics_duration(time: &Option<Range<NaiveDateTime>>) -> f64 {
        let duration = time
            .as_ref()
            .map(|t| t.end.signed_duration_since(t.start))
            .unwrap_or(Duration::MAX);
        let duration = duration.to_std().unwrap().as_secs_f64();
        duration
    }
}

impl Combat {
    pub fn read_log_combat_data(&self, file_path: &Path) -> Option<Vec<u8>> {
        let pos = match self.log_pos.clone() {
            Some(p) => p,
            None => return None,
        };

        let file = match File::options().create(false).read(true).open(file_path) {
            Ok(f) => f,
            Err(_) => return None,
        };

        let mut combat_data = Vec::new();
        combat_data.resize((pos.end - pos.start) as _, 0);
        let mut reader = BufReader::with_capacity(1 << 20, file);
        reader.seek(SeekFrom::Start(pos.start)).ok()?;

        reader.read_exact(&mut combat_data).ok()?;

        Some(combat_data)
    }
}

impl CombatName {
    fn new(rule: &CombatNameRule, name_manager: &NameManager) -> Self {
        let additional_infos: Vec<_> = rule
            .additional_info_rules
            .iter()
            .filter(|r| name_manager.matches(r))
            .map(|r| r.name.clone())
            .collect();
        Self {
            name: rule.name_rule.name.clone(),
            additional_infos,
        }
    }

    fn format(&self) -> Cow<'_, String> {
        if self.additional_infos.len() == 0 {
            return Cow::Borrowed(&self.name);
        }

        let name = format!(
            "{} ({})",
            self.name,
            self.additional_infos.iter().join(", ")
        );
        Cow::Owned(name)
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
