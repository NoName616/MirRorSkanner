// Работа с угловыми форматами: градусы, минуты, секунды (d:m:s) ↔ float

use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AngleParseError {
    #[error("Invalid angle format: {0}")]
    InvalidFormat(String),
    #[error("Invalid number: {0}")]
    InvalidNumber(String),
}

/// Представляет угол в градусах, минутах и секундах
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AngleDMS {
    pub degrees: i32,
    pub minutes: u8,
    pub seconds: u8,
}

impl AngleDMS {
    /// Создает новый угол из градусов, минут, секунд
    pub fn new(degrees: i32, minutes: u8, seconds: u8) -> Self {
        Self {
            degrees,
            minutes: minutes.min(59),
            seconds: seconds.min(59),
        }
    }

    /// Преобразует угол в десятичные градусы (float)
    pub fn to_degrees(&self) -> f64 {
        let sign = if self.degrees < 0 { -1.0 } else { 1.0 };
        sign * (self.degrees.abs() as f64
            + self.minutes as f64 / 60.0
            + self.seconds as f64 / 3600.0)
    }

    /// Парсит строку формата "d:m:s" или "d:m:s" в AngleDMS
    /// Примеры: "12:30:45", "-5:15:30", "0:1:0"
    pub fn from_str(s: &str) -> Result<Self, AngleParseError> {
        let parts: Vec<&str> = s.split(':').collect();

        if parts.len() != 3 {
            return Err(AngleParseError::InvalidFormat(format!(
                "Expected format 'd:m:s', got '{}'",
                s
            )));
        }

        let degrees = parts[0]
            .parse::<i32>()
            .map_err(|_| AngleParseError::InvalidNumber(parts[0].to_string()))?;

        let minutes = parts[1]
            .parse::<u8>()
            .map_err(|_| AngleParseError::InvalidNumber(parts[1].to_string()))?
            .min(59);

        let seconds = parts[2]
            .parse::<u8>()
            .map_err(|_| AngleParseError::InvalidNumber(parts[2].to_string()))?
            .min(59);

        Ok(Self::new(degrees, minutes, seconds))
    }

    /// Парсит строку, которая может быть в формате d:m:s или просто числом
    pub fn from_str_flexible(s: &str) -> Result<Self, AngleParseError> {
        // Если строка содержит ':', парсим как d:m:s
        if s.contains(':') {
            Self::from_str(s)
        } else {
            // Иначе парсим как число градусов
            let degrees = s
                .parse::<f64>()
                .map_err(|_| AngleParseError::InvalidNumber(s.to_string()))?;
            Ok(Self::from_degrees(degrees))
        }
    }

    /// Создает угол из десятичных градусов
    pub fn from_degrees(degrees: f64) -> Self {
        let sign = if degrees < 0.0 { -1 } else { 1 };
        let abs_degrees = degrees.abs();

        let d = abs_degrees.floor() as i32 * sign;
        let remainder = abs_degrees - abs_degrees.floor();

        let total_seconds = (remainder * 3600.0).round() as u32;
        let m = (total_seconds / 60) as u8;
        let s = (total_seconds % 60) as u8;

        Self::new(d, m, s)
    }
}

impl fmt::Display for AngleDMS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.degrees, self.minutes, self.seconds)
    }
}

impl From<f64> for AngleDMS {
    fn from(degrees: f64) -> Self {
        Self::from_degrees(degrees)
    }
}

impl From<AngleDMS> for f64 {
    fn from(angle: AngleDMS) -> Self {
        angle.to_degrees()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dms() {
        let angle = AngleDMS::from_str("12:30:45").unwrap();
        assert_eq!(angle.degrees, 12);
        assert_eq!(angle.minutes, 30);
        assert_eq!(angle.seconds, 45);
    }

    #[test]
    fn test_parse_negative() {
        let angle = AngleDMS::from_str("-5:15:30").unwrap();
        assert_eq!(angle.degrees, -5);
        assert_eq!(angle.minutes, 15);
        assert_eq!(angle.seconds, 30);
    }

    #[test]
    fn test_to_degrees() {
        let angle = AngleDMS::new(12, 30, 45);
        let expected = 12.0 + 30.0 / 60.0 + 45.0 / 3600.0;
        assert!((angle.to_degrees() - expected).abs() < 0.0001);
    }

    #[test]
    fn test_from_degrees() {
        let degrees = 12.5125;
        let angle = AngleDMS::from_degrees(degrees);
        let back = angle.to_degrees();
        assert!((back - degrees).abs() < 0.01);
    }

    #[test]
    fn test_format_zero() {
        let angle = AngleDMS::from_str("0:1:0").unwrap();
        assert_eq!(angle.to_string(), "0:1:0");
        assert!((angle.to_degrees() - 1.0 / 60.0).abs() < 0.0001);
    }
}
