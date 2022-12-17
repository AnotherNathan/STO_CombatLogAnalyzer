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

        let number = number.abs();

        write!(&mut result, "{:.*}", precision, number % 1000.0).unwrap();

        let mut number = number as u64 / 1000;

        while number > 0 {
            self.buffer.clear();
            write!(&mut self.buffer, "{}'", number % 1000).unwrap();
            result.insert_str(0, &self.buffer);
            number /= 1000;
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

        assert_eq!(formatter.format(-123456789.0, 2), "123'456'789.00");
    }
}
