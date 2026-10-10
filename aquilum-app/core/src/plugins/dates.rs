use serde::{Deserialize, Serialize};
use crate::search::note_date;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalDateTime {
    pub year: i64,
    pub month: i64,
    pub day: i64,
    pub hour: i64,
    pub minute: i64,
    pub second: i64,
}

impl LocalDateTime {
    pub fn valid(&self) -> bool {
        (1..=9999).contains(&self.year) && (1..=12).contains(&self.month)
            && (1..=note_date::days_in_month(self.year, self.month)).contains(&self.day)
            && (0..24).contains(&self.hour) && (0..60).contains(&self.minute)
            && (0..60).contains(&self.second)
    }

    pub fn parse(value: &str) -> Option<Self> {
        let bytes = value.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-'
            || !bytes.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) { return None; }
        let date = Self { year: value[..4].parse().ok()?, month: value[5..7].parse().ok()?,
            day: value[8..].parse().ok()?, hour: 0, minute: 0, second: 0 };
        date.valid().then_some(date)
    }

    pub fn date(&self) -> String { format!("{:04}-{:02}-{:02}", self.year, self.month, self.day) }

    // Цикл 400 лет сохраняет календарь и укладывается в наносекунды i64.
    fn nanos(&self) -> (i64, i64) {
        let year = 1800 + (self.year - 1800).rem_euclid(400);
        (note_date::from_civil(year, self.month, self.day, 0), self.year - year)
    }

    pub fn add_days(&self, days: i64) -> Self {
        let (nanos, era) = self.nanos();
        let cycle = 146097_i128 * 86_400_000_000_000;
        let base = note_date::from_civil(1800, 1, 1, 0) as i128;
        let shifted = nanos as i128 + days as i128 * 86_400_000_000_000 - base;
        let cycles = shifted.div_euclid(cycle) as i64;
        let normalized = (base + shifted.rem_euclid(cycle)) as i64;
        let (year, month, day, _) = note_date::to_civil(normalized);
        Self { year: year + era + cycles * 400, month, day, ..*self }
    }

    pub fn weekday(&self) -> i64 { note_date::weekday(self.nanos().0) % 7 }

    pub fn start_of_week(&self, start: i64) -> Self {
        let date = self.add_days(-(self.weekday() - start).rem_euclid(7));
        Self { hour: 0, minute: 0, second: 0, ..date }
    }

    pub fn iso_week(&self) -> (i64, i64) {
        let thursday = self.add_days(3 - (self.weekday() + 6) % 7);
        let first = Self { month: 1, day: 4, ..thursday }.start_of_week(1);
        let start = self.start_of_week(1);
        let (a, era_a) = start.nanos();
        let (b, era_b) = first.nanos();
        let days = a.div_euclid(86_400_000_000_000) - b.div_euclid(86_400_000_000_000) + (era_a - era_b) / 400 * 146097;
        (thursday.year, days / 7 + 1)
    }

    pub fn local_week(&self) -> (i64, i64) {
        let year = self.add_days(6 - self.weekday()).year;
        let first = Self { year, month: 1, day: 1, ..*self }.start_of_week(0);
        let start = self.start_of_week(0);
        let (a, era_a) = start.nanos();
        let (b, era_b) = first.nanos();
        let days = a.div_euclid(86_400_000_000_000) - b.div_euclid(86_400_000_000_000) + (era_a - era_b) / 400 * 146097;
        (year, days / 7 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundaries_and_calendar_arithmetic() {
        for (date, expected) in [("2020-12-31", (2020,53)), ("2021-01-03", (2020,53)), ("2024-12-30", (2025,1)), ("1800-01-01", (1800,1)), ("2200-01-01", (2200,1))] {
            let date = LocalDateTime::parse(date).unwrap();
            assert_eq!(date.iso_week(), expected);
        }
        let date = LocalDateTime::parse("2024-02-28").unwrap();
        assert_eq!(date.add_days(1).date(), "2024-02-29");
        assert_eq!(date.add_days(2).date(), "2024-03-01");
        assert_eq!(date.add_days(146097).date(), "2424-02-28");
        assert_eq!(date.add_days(-146097).date(), "1624-02-28");
        assert_eq!(LocalDateTime::parse("2021-12-31").unwrap().local_week(), (2022,1));
        assert_eq!(LocalDateTime::parse("2021-01-03").unwrap().start_of_week(1).date(), "2020-12-28");
        for value in ["2023-02-29", "2024-13-01", "2024-1-01", "../x"] { assert!(LocalDateTime::parse(value).is_none()); }
        assert_eq!(LocalDateTime::parse("1600-02-28").unwrap().add_days(1).date(), "1600-02-29");
    }
}
