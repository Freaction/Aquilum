use std::cell::Cell;

const NANOS_PER_SECOND: i64 = 1_000_000_000;
const SECONDS_PER_DAY: i64 = 86_400;

thread_local! {
    static LOCAL_OFFSET: Cell<i64> = const { Cell::new(0) };
    static NOW_USED: Cell<bool> = const { Cell::new(false) };
}

pub fn set_local_offset(seconds: i64) {
    LOCAL_OFFSET.with(|offset| offset.set(seconds));
}

fn local_offset() -> i64 {
    LOCAL_OFFSET.with(Cell::get)
}

pub fn parse(value: &str) -> Option<i64> {
    parse_at(value, local_offset())
}

pub fn parse_at(value: &str, offset: i64) -> Option<i64> {
    let mut parts = value.split(['T', ' ', 't']).filter(|part| !part.is_empty());
    let day = civil_day(parts.next()?)?;
    let seconds = parts.next().and_then(time_of_day).unwrap_or(0);
    Some((day * SECONDS_PER_DAY + seconds - offset) * NANOS_PER_SECOND)
}

fn civil_day(text: &str) -> Option<i64> {
    let separator = text.chars().find(|symbol| *symbol == '-' || *symbol == '.')?;
    let mut fields = text.split(separator);
    let first = fields.next()?;
    let second = number(fields.next()?)?;
    let third = fields.next()?;
    if fields.next().is_some() {
        return None;
    }
    let (year, month, day) = if first.len() == 4 {
        (number(first)?, second, number(third)?)
    } else if third.len() == 4 {
        (number(third)?, second, number(first)?)
    } else {
        return None;
    };
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn time_of_day(text: &str) -> Option<i64> {
    let mut fields = text.split(':');
    let hour = number(fields.next()?)?;
    let minute = number(fields.next()?)?;
    let second = fields.next().map(number).unwrap_or(Some(0))?;
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    Some(hour * 3600 + minute * 60 + second)
}

fn number(text: &str) -> Option<i64> {
    let digits = text.trim();
    if digits.is_empty() || !digits.bytes().all(|symbol| symbol.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

pub fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if leap(year) => 29,
        _ => 28,
    }
}

fn leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub fn to_civil(nanos: i64) -> (i64, i64, i64, i64) {
    to_civil_at(nanos, local_offset())
}

pub fn to_civil_at(nanos: i64, offset: i64) -> (i64, i64, i64, i64) {
    let seconds = nanos.div_euclid(NANOS_PER_SECOND) + offset;
    let day = seconds.div_euclid(SECONDS_PER_DAY);
    let inside = seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, date) = civil_from_days(day);
    (year, month, date, inside)
}

pub fn from_civil(year: i64, month: i64, day: i64, seconds: i64) -> i64 {
    (days_from_civil(year, month, day) * SECONDS_PER_DAY + seconds - local_offset())
        * NANOS_PER_SECOND
}

#[cfg(test)]
fn start_of_day_at(nanos: i64, offset: i64) -> i64 {
    let (year, month, day, _) = to_civil_at(nanos, offset);
    (days_from_civil(year, month, day) * SECONDS_PER_DAY - offset) * NANOS_PER_SECOND
}

pub fn take_now_used() -> bool {
    NOW_USED.with(|used| used.replace(false))
}

pub fn now() -> i64 {
    NOW_USED.with(|used| used.set(true));
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as i64)
        .unwrap_or(0)
}

pub fn start_of_day(nanos: i64) -> i64 {
    let (year, month, day, _) = to_civil(nanos);
    from_civil(year, month, day, 0)
}

pub fn weekday(nanos: i64) -> i64 {
    let day = (nanos.div_euclid(NANOS_PER_SECOND) + local_offset()).div_euclid(SECONDS_PER_DAY);
    (day + 3).rem_euclid(7) + 1
}

pub fn format(nanos: i64) -> String {
    let (year, month, date, inside) = to_civil(nanos);
    let hour = inside / 3600;
    let minute = (inside % 3600) / 60;
    if hour == 0 && minute == 0 {
        format!("{year:04}-{month:02}-{date:02}")
    } else {
        format!("{year:04}-{month:02}-{date:02} {hour:02}:{minute:02}")
    }
}

fn civil_from_days(day: i64) -> (i64, i64, i64) {
    let shifted = day + 719_468;
    let era = if shifted >= 0 { shifted } else { shifted - 146_096 } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let date = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    (if month <= 2 { year + 1 } else { year }, month, date)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = if shifted >= 0 { shifted } else { shifted - 399 } / 400;
    let year_of_era = shifted - era * 400;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::{format, parse};

    const DAY: i64 = 86_400 * 1_000_000_000;

    #[test]
    fn a_time_zone_shifts_the_civil_day_but_not_the_moment() {
        let offset = 7 * 3600;
        let midnight = super::parse_at("2026-09-20", offset).expect("дата");
        let expected = (super::days_from_civil(2026, 9, 19) * 86_400 + 17 * 3600) * 1_000_000_000;
        assert_eq!(midnight, expected);

        let evening = super::parse_at("2026-09-20 22:35", offset).expect("дата со временем");
        let (year, month, day, inside) = super::to_civil_at(evening, offset);
        assert_eq!((year, month, day), (2026, 9, 20));
        assert_eq!(inside, 22 * 3600 + 35 * 60);
        assert_eq!(super::start_of_day_at(evening, offset), midnight);
    }

    #[test]
    fn the_epoch_itself_is_day_zero() {
        assert_eq!(parse("1970-01-01"), Some(0));
    }

    #[test]
    fn an_iso_date_counts_days_forward() {
        assert_eq!(parse("1970-01-02"), Some(DAY));
    }

    #[test]
    fn a_year_last_date_reads_the_day_first() {
        assert_eq!(parse("07-10-2024"), parse("2024-10-07"));
    }

    #[test]
    fn dots_separate_a_year_last_date_too() {
        assert_eq!(parse("07.10.2024"), parse("2024-10-07"));
    }

    #[test]
    fn a_time_adds_seconds_to_the_day() {
        let day = parse("2024-10-07").expect("date");
        assert_eq!(parse("2024-10-07 17:17"), Some(day + 17 * 3600 * 1_000_000_000 + 17 * 60 * 1_000_000_000));
        assert_eq!(parse("2024-10-07T17:17:05"), Some(day + (17 * 3600 + 17 * 60 + 5) * 1_000_000_000));
    }

    #[test]
    fn a_leap_day_is_a_real_day_and_a_missing_one_is_not() {
        assert!(parse("2024-02-29").is_some());
        assert!(parse("2023-02-29").is_none());
    }

    #[test]
    fn formatting_is_the_exact_inverse_of_parsing() {
        for text in ["1970-01-01", "2024-10-07", "2024-02-29", "1900-03-01"] {
            assert_eq!(format(parse(text).expect(text)), text);
        }
        assert_eq!(format(parse("2024-10-07 17:17").unwrap()), "2024-10-07 17:17");
    }

    #[test]
    fn an_ambiguous_or_broken_value_is_refused() {
        assert!(parse("10/07/2024").is_none());
        assert!(parse("2024-13-01").is_none());
        assert!(parse("07-10-24").is_none());
        assert!(parse("вчера").is_none());
        assert!(parse("").is_none());
    }

    #[test]
    fn a_broken_time_does_not_lose_the_date() {
        assert_eq!(parse("2024-10-07 99:99"), parse("2024-10-07"));
    }
}
