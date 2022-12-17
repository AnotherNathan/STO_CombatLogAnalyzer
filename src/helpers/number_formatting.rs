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
            return result;
        }

        self.buffer.clear();
        write!(&mut self.buffer, "{:.*}", precision, fract).unwrap();
        self.buffer.remove(0);
        result.push_str(&self.buffer);

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

        assert_eq!(formatter.format(12012.0, 2), "12'012.00");
        assert_eq!(formatter.format(12012012.0, 2), "12'012'012.00");

        assert_eq!(formatter.format(12012012.0, 0), "12'012'012");

        assert_eq!(formatter.format(1.567, 2), "1.57");
    }
}
