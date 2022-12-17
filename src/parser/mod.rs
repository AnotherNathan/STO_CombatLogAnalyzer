use std::{
    fmt::Write,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    time::SystemTime,
};

use bitflags::bitflags;
use chrono::{DateTime, NaiveDateTime};
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Debug)]
pub struct Record<'a> {
    pub time: NaiveDateTime,
    pub entity: &'a str,
    pub player_handle: Option<&'a str>,
    pub pet_name: Option<&'a str>,
    pub damage_source: &'a str,
    pub damage_type: &'a str,
    pub hit_flags: HitFlags,
    pub damage: f32,
}

bitflags! {
    pub struct HitFlags: u8{
        const NONE = 0;
        const CRITICAL = 1;
        const FLANK = 1 << 1;
        const KILL = 1 << 2;
    }
}

impl Default for HitFlags {
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

lazy_static! {
    static ref PLAYER_NAME_REGEX: Regex = Regex::new(r"P\[\d+@\d+ ([^\]]+)\]").unwrap();
}

// TODO remove once the iterator method advance_by is stabilized
trait AdvanceBy {
    fn advance_by_n(&mut self, n: usize) -> Result<(), ()>;
}

impl<T: Iterator> AdvanceBy for T {
    fn advance_by_n(&mut self, n: usize) -> Result<(), ()> {
        for _ in 0..n {
            self.next().ok_or(())?;
        }

        Ok(())
    }
}

impl Parser {
    pub fn new(file_name: &Path) -> Option<Self> {
        let file = File::options()
            .read(true)
            .write(false)
            .open(file_name)
            .ok()?;

        Some(Self {
            file: BufReader::new(file),
            buffer: String::new(),
            scratch_pad: String::new(),
        })
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

        let time_and_entity = parts.next()?;
        let mut time_and_entity = time_and_entity.split("::");
        let time = time_and_entity.next()?;
        scratch_pad.clear();
        write!(scratch_pad, "{}00", time).ok()?;
        let time = NaiveDateTime::parse_from_str(&scratch_pad, "%y:%m:%d:%H:%M:%S%.3f").ok()?;
        let entity = time_and_entity.next()?;

        let entity_type = parts.next()?;
        let player_handle = if entity_type.starts_with('P') {
            PLAYER_NAME_REGEX
                .captures(entity_type)
                .map(|c| c.get(1))
                .flatten()
                .map(|m| m.as_str())
        } else {
            None
        };

        let pet_name = parts.next()?;
        let pet_name = if pet_name.is_empty() {
            None
        } else {
            Some(pet_name)
        };

        // pet type, target, target type
        parts.advance_by_n(3).ok()?;

        let damage_source = parts.next()?;

        // don't know what these are (e.g. Pn.Rfd0cd)
        parts.advance_by_n(1).ok()?;

        let damage_type = parts.next()?;
        let hit_flags = Self::parse_flags(parts.next()?);
        let damage = parts.next()?;
        let damage = str::parse::<f32>(damage).ok()?.abs();

        let record = Record {
            damage,
            hit_flags,
            damage_source,
            damage_type,
            entity,
            time,
            pet_name,
            player_handle,
        };
        Some(record)
    }

    fn parse_flags(flags: &str) -> HitFlags {
        let mut result = HitFlags::NONE;
        for flag in flags.split('|') {
            result |= match flag {
                "Critical" => HitFlags::CRITICAL,
                "Flank" => HitFlags::FLANK,
                "Kill" => HitFlags::KILL,
                _ => HitFlags::NONE,
            };
        }

        result
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
            r"D:\Games\Star Trek Online_en\Star Trek Online\Playtest\logs\GameClient\combatlog.log",
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

        println!("{:?}", record_data);
    }
}
