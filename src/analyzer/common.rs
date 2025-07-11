use std::{
    iter::Sum,
    ops::{Add, AddAssign},
};

use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ValueFlags: u8{
        const NONE = 0;
        const CRITICAL = 1;
        const FLANK = 1 << 1;
        const KILL = 1 << 2;
        const IMMUNE = 1 << 3;
        const SHIELD_BREAK = 1 << 4;
        const MISS = 1 << 5;
    }
}

impl Default for ValueFlags {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ShieldHullValues {
    pub all: f64,
    pub shield: f64,
    pub hull: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ShieldHullOptionalValues {
    pub all: Option<f64>,
    pub shield: Option<f64>,
    pub hull: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ShieldHullCounts {
    pub all: u64,
    pub shield: u64,
    pub hull: u64,
}

impl ShieldHullValues {
    pub fn per_seconds(total: &Self, duration: f64) -> Self {
        // avoid absurd high numbers by having a combat duration of at least 1 sec
        Self {
            all: total.all / duration.max(1.0),
            shield: total.shield / duration.max(1.0),
            hull: total.hull / duration.max(1.0),
        }
    }
}

impl Sum for ShieldHullValues {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let (shield, hull, all) = iter.fold((0.0, 0.0, 0.0), |(s, h, a), v| {
            (v.shield + s, v.hull + h, v.all + a)
        });
        Self { all, shield, hull }
    }
}

impl Add for ShieldHullValues {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            all: self.all + rhs.all,
            shield: self.shield + rhs.shield,
            hull: self.hull + rhs.hull,
        }
    }
}

impl AddAssign for ShieldHullValues {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.all += rhs.all;
        self.shield += rhs.shield;
        self.hull += rhs.hull;
    }
}

impl ShieldHullOptionalValues {
    pub fn average(
        total: &ShieldHullValues,
        shield_count: u64,
        hull_count: u64,
        all_count: u64,
    ) -> Self {
        Self {
            all: if all_count == 0 {
                None
            } else {
                Some(total.all / all_count as f64)
            },
            shield: if shield_count == 0 {
                None
            } else {
                Some(total.shield / shield_count as f64)
            },
            hull: if hull_count == 0 {
                None
            } else {
                Some(total.hull / hull_count as f64)
            },
        }
    }

    pub fn percentage(amount: &ShieldHullValues, total: &ShieldHullValues) -> Self {
        Self {
            all: percentage_f64(amount.all, total.all),
            shield: percentage_f64(amount.shield, total.shield),
            hull: percentage_f64(amount.hull, total.hull),
        }
    }
}

impl ShieldHullCounts {
    pub fn to_values(&self) -> ShieldHullValues {
        ShieldHullValues {
            all: self.all as _,
            shield: self.shield as _,
            hull: self.hull as _,
        }
    }
}

impl Sum for ShieldHullCounts {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let (shield, hull, all) = iter.fold((0, 0, 0), |(s, h, a), v| {
            (v.shield + s, v.hull + h, v.all + a)
        });
        Self { all, shield, hull }
    }
}

impl Add for ShieldHullCounts {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            all: self.all + rhs.all,
            shield: self.shield + rhs.shield,
            hull: self.hull + rhs.hull,
        }
    }
}

impl AddAssign for ShieldHullCounts {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.all += rhs.all;
        self.shield += rhs.shield;
        self.hull += rhs.hull;
    }
}

impl ValueFlags {
    pub fn parse(input: &str) -> Self {
        let mut flags = ValueFlags::NONE;
        for flag in input.split('|') {
            flags |= match flag {
                "Critical" => ValueFlags::CRITICAL,
                "Flank" => ValueFlags::FLANK,
                "Kill" => ValueFlags::KILL,
                "Immune" => ValueFlags::IMMUNE,
                "ShieldBreak" => ValueFlags::SHIELD_BREAK,
                "Miss" => ValueFlags::MISS,
                _ => ValueFlags::NONE,
            };
        }

        flags
    }
}

pub fn percentage_f64(amount: f64, total: f64) -> Option<f64> {
    if total == 0.0 {
        None
    } else {
        Some((amount / total) * 100.0)
    }
}

pub fn average(total_value: f64, count: u64) -> Option<f64> {
    if count == 0 {
        None
    } else {
        Some(total_value / count as f64)
    }
}

pub fn percentage_u64(amount: u64, total_count: u64) -> Option<f64> {
    if total_count == 0 {
        return None;
    }

    Some((amount as f64 / total_count as f64) * 100.0)
}
