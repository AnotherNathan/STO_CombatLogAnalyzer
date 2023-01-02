use std::{
    fmt::Write,
    fs::File,
    io::{BufRead, BufReader, Seek},
    path::Path,
};

use bitflags::bitflags;
use chrono::NaiveDateTime;
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Debug)]
pub struct Record<'a> {
    pub time: NaiveDateTime,
    pub source: Entity<'a>,
    pub target: Entity<'a>,
    pub sub_source: Entity<'a>, // e.g. a pet
    pub value_name: &'a str,
    pub value_type: &'a str,
    pub value_flags: ValueFlags,
    pub value: RecordValue,
    pub raw: &'a str,
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
}

#[derive(Debug, Clone, Copy)]
pub enum RecordValue {
    Damage(Value),
    Heal(Value),
}

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Shield(f64),
    Hull(f64),
}

bitflags! {
    pub struct ValueFlags: u8{
        const NONE = 0;
        const CRITICAL = 1;
        const FLANK = 1 << 1;
        const KILL = 1 << 2;
    }
}

impl Default for ValueFlags {
    fn default() -> Self {
        Self::NONE
    }
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
        let mut file = File::options()
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
        let count = self.file.read_line(&mut self.buffer)?;
        if count == 0 {
            return Err(RecordError::EndReached);
        }

        Self::parse_from_line(&self.buffer, &mut self.scratch_pad)
            .ok_or_else(|| RecordError::InvalidRecord(&self.buffer))
    }

    fn parse_from_line<'a>(line: &'a str, scratch_pad: &mut String) -> Option<Record<'a>> {
        let mut parts = line.split(',');

        let time_and_source_name = parts.next()?.trim();
        let (time, source_name) =
            Self::parse_time_and_source_name(time_and_source_name, scratch_pad)?;

        let source_id_and_unique_name = parts.next()?.trim();
        let source = Entity::parse(source_name, source_id_and_unique_name)?;

        let sub_source_name = parts.next()?.trim();
        let sub_source_id_and_unique_name = parts.next()?.trim();
        let sub_source = Entity::parse(sub_source_name, sub_source_id_and_unique_name)?;

        let target_name = parts.next()?.trim();
        let target_id_and_unique_name = parts.next()?.trim();
        let target = Entity::parse(target_name, target_id_and_unique_name)?;

        let value_name = parts.next()?.trim();

        // don't know what these are (e.g. Pn.Rfd0cd)
        parts.next()?;

        let value_type = parts.next()?.trim();
        let value_flags = parts.next()?.trim();
        let value_flags = ValueFlags::parse(value_flags);
        let damage_or_heal = parts.next()?.trim();
        let damage_or_heal_pre_modifiers = parts.next()?.trim();

        let value = RecordValue::new(value_type, damage_or_heal, damage_or_heal_pre_modifiers)?;

        let record = Record {
            time,
            source,
            target,
            sub_source,
            value_name,
            value_type,
            value_flags,
            value,
            raw: line,
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

impl ValueFlags {
    fn parse(input: &str) -> Self {
        let mut flags = ValueFlags::NONE;
        for flag in input.split('|') {
            flags |= match flag {
                "Critical" => ValueFlags::CRITICAL,
                "Flank" => ValueFlags::FLANK,
                "Kill" => ValueFlags::KILL,
                _ => ValueFlags::NONE,
            };
        }

        flags
    }
}

lazy_static! {
    static ref ID_AND_UNIQUE_NAME_REGEX: Regex =
        Regex::new(r"(?P<type>P|C)\[(?P<id>\d+)(@(?P<player_id>\d+))? (?P<unique_name>[^\]]+)\]")
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
        let unique_name = captures.name("unique_name")?.as_str();

        match entity_type {
            "P" => {
                let player_id = captures.name("player_id")?.as_str();
                let player_id = str::parse::<u64>(player_id).ok()?;

                Some(Self::Player {
                    full_name: unique_name,
                    id: (id, player_id),
                })
            }
            "C" => Some(Self::NonPlayer {
                name,
                id,
                unique_name,
            }),
            _ => None,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Entity::None => None,
            Entity::Player { full_name, .. } => Some(full_name),
            Entity::NonPlayer { name, .. } => Some(name),
        }
    }

    pub fn unique_name(&self) -> Option<&str> {
        match self {
            Entity::None => None,
            Entity::Player { full_name, .. } => Some(full_name),
            Entity::NonPlayer { unique_name, .. } => Some(unique_name),
        }
    }
}

impl RecordValue {
    pub fn new(
        value_type: &str,
        damage_or_heal: &str,
        damage_or_heal_pre_modifiers: &str,
    ) -> Option<Self> {
        let damage_or_heal = str::parse::<f64>(damage_or_heal).ok()?;

        if damage_or_heal < 0.0 && value_type == "HitPoints" {
            return Some(Self::Heal(Value::Hull(damage_or_heal.abs())));
        }

        if value_type == "Shield" {
            if damage_or_heal < 0.0 && damage_or_heal_pre_modifiers == "0" {
                return Some(Self::Heal(Value::Shield(damage_or_heal.abs())));
            }

            return Some(Self::Damage(Value::Shield(damage_or_heal.abs())));
        }

        return Some(Self::Damage(Value::Hull(damage_or_heal.abs())));
    }

    pub fn get(&self) -> f64 {
        match self {
            RecordValue::Damage(v) | RecordValue::Heal(v) => v.get(),
        }
    }
}

impl Value {
    pub fn get(&self) -> f64 {
        match self {
            Value::Shield(v) | Value::Hull(v) => *v,
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

    use super::*;

    #[ignore = "manual test"]
    #[test]
    fn read_log() {
        let mut parser = Parser::new(&PathBuf::from(
            r"D:\Games\Star Trek Online_en\Star Trek Online\Live\logs\GameClient\combatlog.log",
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
}
