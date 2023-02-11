use std::ops::Range;

use chrono::*;

pub mod number_formatting;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F64TotalOrd(pub f64);

impl PartialOrd for F64TotalOrd {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.total_cmp(&other.0))
    }
}

impl Eq for F64TotalOrd {}

impl Ord for F64TotalOrd {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

pub fn time_range_to_duration(time_range: &Range<NaiveDateTime>) -> Duration {
    time_range.end.signed_duration_since(time_range.start)
}

pub fn time_range_to_duration_or_zero(time_range: &Option<Range<NaiveDateTime>>) -> Duration {
    time_range
        .as_ref()
        .map(time_range_to_duration)
        .unwrap_or(Duration::zero())
}

pub fn format_duration(duration: Duration) -> String {
    let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap() + duration;
    if duration >= Duration::hours(1) {
        return format!("{}", time.format("%T%.3f"));
    }
    format!("{}", time.format("%M:%S%.3f"))
}

#[macro_export]
macro_rules! unwrap_or_continue {
    ($expression:expr) => {
        match $expression {
            Some(thing) => thing,
            None => continue,
        }
    };
}

#[macro_export]
macro_rules! unwrap_or_break {
    ($expression:expr) => {
        match $expression {
            Some(thing) => thing,
            None => break,
        }
    };

    ($expression:expr, $label:lifetime) => {
        match $expression {
            Some(thing) => thing,
            None => break $label,
        }
    };
}

#[macro_export]
macro_rules! unwrap_or_return {
    ($expression:expr) => {
        match $expression {
            Some(thing) => thing,
            None => return,
        }
    };

    ($expression:expr, $ret:expr) => {
        match $expression {
            Some(thing) => thing,
            None => return $ret,
        }
    };
}
