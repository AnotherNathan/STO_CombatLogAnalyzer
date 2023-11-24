use std::fmt::Write;

pub struct NumberFormatter {
    buffer: String,
}

impl NumberFormatter {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    pub fn format(&mut self, number: f64, precision: usize) -> String {
        let mut result = String::new();

        let is_negative = number.is_sign_negative();

        let mut number = number.abs();
        let fract = number.fract();

        if precision == 0 {
            number = number.round();
        }

        let mut number = number as u64;

        loop {
            self.buffer.clear();
            if number < 1000 {
                write!(&mut self.buffer, "{}", number).unwrap();
                result.insert_str(0, &self.buffer);
                break;
            }

            write!(&mut self.buffer, "'{:03}", number % 1000).unwrap();
            result.insert_str(0, &self.buffer);
            number /= 1000;
        }

        if precision == 0 {
            return Self::add_sign(result, is_negative);
        }

        self.buffer.clear();
        write!(&mut self.buffer, "{:.*}", precision, fract).unwrap();
        self.buffer.remove(0);
        result.push_str(&self.buffer);

        Self::add_sign(result, is_negative)
    }

    pub fn format_with_automated_suffixes(&mut self, number: f64) -> String {
        if number.abs() == 0.0 {
            return "0.0".to_string();
        }

        let is_negative = number.is_sign_negative();

        let number = number.abs();

        const THRESHOLD_AND_SUFFIX: &[(f64, &'static str)] = &[
            (1e-6, "n"),
            (1e-3, "u"),
            (0.0, "m"),
            (1.0e3, ""),
            (1.0e6, "k"),
            (1.0e9, "M"),
            (1.0e12, "G"),
            (1.0e15, "T"),
        ];

        const PRECISION_THRESHOLD: &[(f64, usize)] = &[(10.0, 2), (100.0, 1), (1000.0, 0)];

        for (threshold, suffix) in THRESHOLD_AND_SUFFIX.iter().copied() {
            if number < threshold {
                let normalized_number = number / (threshold / 1e3);
                let precision = PRECISION_THRESHOLD
                    .iter()
                    .copied()
                    .find_map(|(t, p)| if normalized_number < t { Some(p) } else { None })
                    .unwrap_or(0);
                return Self::add_sign(
                    format!("{}{}", self.format(normalized_number, precision), suffix),
                    is_negative,
                );
            }
        }

        "<too large>".to_string()
    }

    fn add_sign(mut result: String, is_negative: bool) -> String {
        if is_negative {
            result.insert(0, '-');
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_numbers() {
        let mut formatter = NumberFormatter::new();

        assert_eq!(formatter.format(123.1, 2), "123.10");
        assert_eq!(formatter.format(12345.1, 2), "12'345.10");
        assert_eq!(formatter.format(12345.123, 2), "12'345.12");
        assert_eq!(formatter.format(123456789.0, 2), "123'456'789.00");

        assert_eq!(formatter.format(12012.0, 2), "12'012.00");
        assert_eq!(formatter.format(12012012.0, 2), "12'012'012.00");

        assert_eq!(formatter.format(12012012.0, 0), "12'012'012");

        assert_eq!(formatter.format(1.567, 2), "1.57");
        assert_eq!(formatter.format(-1.567, 2), "-1.57");

        assert_eq!(formatter.format(-100.0, 0), "-100");
    }

    #[test]
    fn format_with_automated_suffixes() {
        let mut formatter = NumberFormatter::new();

        assert_eq!(formatter.format_with_automated_suffixes(123.1), "123");
        assert_eq!(formatter.format_with_automated_suffixes(12345.1), "12.3k");
        assert_eq!(formatter.format_with_automated_suffixes(12345.123), "12.3k");
        assert_eq!(
            formatter.format_with_automated_suffixes(123456789.0),
            "123M"
        );

        assert_eq!(formatter.format_with_automated_suffixes(12012.0), "12.0k");
        assert_eq!(
            formatter.format_with_automated_suffixes(12012012.0),
            "12.0M"
        );

        assert_eq!(formatter.format_with_automated_suffixes(1.567), "1.57");
        assert_eq!(formatter.format_with_automated_suffixes(-1.567), "-1.57");

        assert_eq!(formatter.format_with_automated_suffixes(0.0), "0.0");
        assert_eq!(formatter.format_with_automated_suffixes(-0.0), "0.0");
    }
}
