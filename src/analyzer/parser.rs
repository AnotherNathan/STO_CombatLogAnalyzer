use std::{
    fmt::Write,
    fs::File,
    io::{BufRead, BufReader, Seek},
    ops::Range,
    path::Path,
};

use chrono::NaiveDateTime;
use lazy_static::lazy_static;
use regex::Regex;

use super::*;

#[derive(Debug)]
pub struct Record<'a> {
    pub time: NaiveDateTime,
    pub source: Entity<'a>,
    pub target: Entity<'a>,
    pub indirect_source: Entity<'a>, // e.g. a pet
    pub value_name: &'a str,
    pub value_type: &'a str,
    pub value_flags: ValueFlags,
    pub value: RecordValue,
    pub raw: &'a str,
    pub log_pos: Option<Range<u64>>,
}

#[derive(Debug)]
pub enum Entity<'a> {
    None,
    Player {
        full_name: &'a str, // -> name@handle
        id: (u64, u64),
    },
    NonPlayer {
        name: &'a str,
        id: u64,
        unique_name: &'a str,
    },
    NonPlayerCharacter {
        id: u64,
        name: &'a str,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum RecordValue {
    Damage(BaseHit),
    Heal(BaseHealTick),
}

pub struct Parser {
    file: BufReader<File>,
    buffer: String,
    scratch_pad: String,
}

pub enum RecordError<'a> {
    EndReached,
    InvalidRecord(&'a str),
}

impl Parser {
    pub fn new(file_name: &Path) -> Option<Self> {
        let file = File::options()
            .read(true)
            .write(false)
            .open(file_name)
            .ok()?;

        Some(Self {
            file: BufReader::with_capacity(1 << 20, file), // 1MB
            buffer: String::new(),
            scratch_pad: String::new(),
        })
    }

    pub fn pos(&mut self) -> Option<u64> {
        self.file.stream_position().ok()
    }

    pub fn parse_next(&mut self) -> Result<Record, RecordError> {
        self.buffer.clear();
        let start_pos = self.pos();
        let count = self.file.read_line(&mut self.buffer)?;
        let end_pos = self.pos();
        if count == 0 {
            return Err(RecordError::EndReached);
        }

        let log_pos = match (start_pos, end_pos) {
            (Some(s), Some(e)) => Some(s..e),
            _ => None,
        };
        Self::parse_from_line(&self.buffer, &mut self.scratch_pad, log_pos)
            .ok_or_else(|| RecordError::InvalidRecord(&self.buffer))
    }

    fn parse_from_line<'a>(
        line: &'a str,
        scratch_pad: &mut String,
        log_pos: Option<Range<u64>>,
    ) -> Option<Record<'a>> {
        let mut parts = line.split(',');

        let time_and_source_name = parts.next()?.trim();
        let (time, source_name) =
            Self::parse_time_and_source_name(time_and_source_name, scratch_pad)?;

        let source_id_and_unique_name = parts.next()?.trim();
        let source = Entity::parse(source_name, source_id_and_unique_name)?;

        let indirect_source_name = parts.next()?.trim();
        let indirect_source_id_and_unique_name = parts.next()?.trim();
        let indirect_source =
            Entity::parse(indirect_source_name, indirect_source_id_and_unique_name)?;

        let target_name = parts.next()?.trim();
        let target_id_and_unique_name = parts.next()?.trim();
        let target = Entity::parse(target_name, target_id_and_unique_name)?;

        let value_name = parts.next()?.trim();

        // don't know what these are (e.g. Pn.Rfd0cd)
        parts.next()?;

        let value_type = parts.next()?.trim();
        let value_flags = parts.next()?.trim();
        let value_flags = ValueFlags::parse(value_flags);
        let value1 = parts.next()?.trim();
        let value2 = parts.next()?.trim();

        let value = RecordValue::new(value_type, value1, value2, value_flags)?;

        let record = Record {
            time,
            source,
            target,
            indirect_source,
            value_name,
            value_type,
            value_flags,
            value,
            raw: line,
            log_pos,
        };
        Some(record)
    }

    fn parse_time_and_source_name<'b>(
        time_and_source_name: &'b str,
        scratch_pad: &mut String,
    ) -> Option<(NaiveDateTime, &'b str)> {
        let mut time_and_source_name = time_and_source_name.split("::");
        let time = time_and_source_name.next()?;

        scratch_pad.clear();
        write!(scratch_pad, "{}00", time).ok()?;
        let time = NaiveDateTime::parse_from_str(&scratch_pad, "%y:%m:%d:%H:%M:%S%.3f").ok()?;
        let name = time_and_source_name.next()?;

        Some((time, name))
    }
}

impl<'a> Record<'a> {
    pub fn is_player_out_damage(&self) -> bool {
        self.source.is_player() && self.value.is_damage()
    }

    pub fn is_immune_or_zero(&self) -> bool {
        self.value.is_all_zero() || self.value_flags.contains(ValueFlags::IMMUNE)
    }

    pub fn is_self_directed(&self) -> bool {
        self.target.is_none() && self.indirect_source.is_none()
    }

    pub fn is_direct_self_damage(&self) -> bool {
        self.is_self_directed() && self.value.is_damage()
    }
}

lazy_static! {
    static ref ID_AND_UNIQUE_NAME_REGEX: Regex = Regex::new(
        r"(?P<type>P|C|S)\[(?P<id>\d+)(@(?P<player_id>\d+))?(\s+(?P<unique_name>[^\]]+))?\]"
    )
    .unwrap();
}
impl<'a> Entity<'a> {
    fn parse(name: &'a str, id_and_unique_name: &'a str) -> Option<Self> {
        if name.is_empty() && (id_and_unique_name.is_empty() || id_and_unique_name == "*") {
            return Some(Self::None);
        }

        let captures = ID_AND_UNIQUE_NAME_REGEX.captures(id_and_unique_name)?;
        let entity_type = captures.name("type")?.as_str();
        let id = captures.name("id")?.as_str();
        let id = str::parse::<u64>(id).ok()?;

        match entity_type {
            "P" => {
                let player_id = captures.name("player_id")?.as_str();
                let player_id = str::parse::<u64>(player_id).ok()?;
                let unique_name = captures.name("unique_name")?.as_str();

                Some(Self::Player {
                    full_name: unique_name,
                    id: (id, player_id),
                })
            }
            "C" => {
                let unique_name = captures.name("unique_name")?.as_str();
                Some(Self::NonPlayer {
                    name,
                    id,
                    unique_name,
                })
            }
            "S" => Some(Self::NonPlayerCharacter { id, name }),
            _ => None,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Entity::None => None,
            Entity::Player { full_name, .. } => Some(full_name),
            Entity::NonPlayer { name, .. } => Some(name),
            Entity::NonPlayerCharacter { name, .. } => Some(name),
        }
    }

    pub fn unique_name(&self) -> Option<&str> {
        match self {
            Entity::None => None,
            Entity::Player { full_name, .. } => Some(full_name),
            Entity::NonPlayer { unique_name, .. } => Some(unique_name),
            Entity::NonPlayerCharacter { .. } => None,
        }
    }

    pub fn is_player(&self) -> bool {
        match self {
            Entity::Player { .. } => true,
            _ => false,
        }
    }

    pub fn is_none(&self) -> bool {
        match self {
            Entity::None { .. } => true,
            _ => false,
        }
    }
}

impl RecordValue {
    pub fn new(value_type: &str, value1: &str, value2: &str, flags: ValueFlags) -> Option<Self> {
        let value1 = str::parse::<f64>(value1).ok()?;
        let value2 = str::parse::<f64>(value2).ok()?;

        if value1 < 0.0 && value_type == "HitPoints" {
            if value1 < 0.0 {
                return Some(Self::Heal(BaseHealTick::hull(value1, flags)));
            }
            return Some(Self::Damage(BaseHit::hull(value1, flags, value2)));
        }

        if value_type == "Shield" {
            if value2 == 0.0 && !flags.contains(ValueFlags::SHIELD_BREAK) {
                if value1 < 0.0 {
                    return Some(Self::Heal(BaseHealTick::shield(value1, flags)));
                }

                if value1 > 0.0 {
                    return Some(Self::Damage(BaseHit::shield_drain(value1, flags)));
                }
            }
            return Some(Self::Damage(BaseHit::shield(value1, flags, value2)));
        }

        if value2 == 0.0 {
            return Some(Self::Damage(BaseHit::hull(value1, flags, value1)));
        }
        return Some(Self::Damage(BaseHit::hull(value1, flags, value2)));
    }

    pub fn is_all_zero(&self) -> bool {
        match self {
            RecordValue::Damage(v) => {
                v.damage == 0.0
                    && match v.specific {
                        SpecificHit::Shield {
                            damage_prevented_to_hull,
                        } => damage_prevented_to_hull == 0.0,
                        SpecificHit::ShieldDrain => true,
                        SpecificHit::Hull { base_damage } => base_damage == 0.0,
                    }
            }
            RecordValue::Heal(v) => v.amount == 0.0,
        }
    }

    pub fn is_damage(&self) -> bool {
        match self {
            RecordValue::Damage(_) => true,
            RecordValue::Heal(_) => false,
        }
    }
}

impl<'a> From<std::io::Error> for RecordError<'a> {
    fn from(_: std::io::Error) -> Self {
        RecordError::EndReached
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rustc_hash::FxHashSet;

    use super::*;

    #[ignore = "manual test"]
    #[test]
    fn read_log() {
        let mut parser = Parser::new(&PathBuf::from(
            r"D:\Games\Star Trek Online_en\Star Trek Online\Live\logs\GameClient\saved_combats\Combat 2023-02-10 20-36-00 - 20-37-05.log",
        ))
        .unwrap();

        let mut record_data = Vec::new();
        loop {
            match parser.parse_next() {
                Ok(record) => record_data.push(record.time),
                Err(RecordError::InvalidRecord(invalid_record)) => {
                    panic!("{}", invalid_record);
                }
                Err(RecordError::EndReached) => break,
            };
        }

        // println!("{:?}", record_data);
    }

    #[ignore = "manual test"]
    #[test]
    fn single_record() {
        let record = Parser::parse_from_line(
            "23:01:07:10:12:56.3::Borg Queen Octahedron,C[25 Mission_Space_Borg_Queen_Diamond],Ayel,P[12793028@5473940 Ayel@greyblizzard],,*,Plasma Fire,Pn.Wujkxq,Plasma,Kill,2086.87,5300.66",
            &mut String::new(),
            None)
            .unwrap();

        println!("{:?}", record)
    }

    #[ignore = "helper to find a way to detect a combats name"]
    #[test]
    fn list_all_names() {
        let mut parser = Parser::new(&PathBuf::from(
            r"D:\Games\Star Trek Online_en\Star Trek Online\Live\logs\GameClient\saved_combats\upload_Infected_Space_26-11-2022_07-40.log",
        ))
        .unwrap();

        let mut names = FxHashSet::default();
        while let Ok(record) = parser.parse_next() {
            if let Some(name) = record.source.name() {
                names.insert(name.to_string());
            }
            if let Some(name) = record.target.name() {
                names.insert(name.to_string());
            }
            if let Some(name) = record.indirect_source.name() {
                names.insert(name.to_string());
            }

            if let Some(unique_name) = record.source.unique_name() {
                names.insert(unique_name.to_string());
            }
            if let Some(unique_name) = record.target.unique_name() {
                names.insert(unique_name.to_string());
            }
            if let Some(unique_name) = record.indirect_source.unique_name() {
                names.insert(unique_name.to_string());
            }
        }

        for name in names.iter() {
            println!("{}", name);
        }
    }
}
