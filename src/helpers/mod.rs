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
