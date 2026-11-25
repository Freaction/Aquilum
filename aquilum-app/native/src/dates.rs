use aquilum_core::text::calendar::{EN, RU};
use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};

use crate::i18n;

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or_default()
}

pub fn offset_seconds() -> i64 {
    i64::from(Local::now().offset().local_minus_utc())
}

pub fn local(epoch_ms: u64) -> Option<DateTime<Local>> {
    Local.timestamp_millis_opt(epoch_ms as i64).single()
}

pub fn time(epoch_ms: u64) -> String {
    let Some(at) = local(epoch_ms) else { return String::new() };
    if i18n::is_russian() {
        format!("{:02}:{:02}", at.hour(), at.minute())
    } else {
        let (pm, hour) = at.hour12();
        format!("{:02}:{:02} {}", hour, at.minute(), if pm { "PM" } else { "AM" })
    }
}

pub fn day(epoch_ms: u64, with_year: bool) -> String {
    let Some(at) = local(epoch_ms) else { return String::new() };
    let month = at.month0() as usize;
    match (i18n::is_russian(), with_year) {
        (true, false) => format!("{} {}", at.day(), RU.months_of[month]),
        (true, true) => format!("{} {} {} г.", at.day(), RU.months_of[month], at.year()),
        (false, false) => format!("{} {}", EN.months[month], at.day()),
        (false, true) => format!("{} {}, {}", EN.months[month], at.day(), at.year()),
    }
}

pub fn date_time(epoch_ms: u64) -> String {
    if local(epoch_ms).is_none() {
        return String::new();
    }
    let joint = if i18n::is_russian() { " в " } else { " at " };
    format!("{}{joint}{}", day(epoch_ms, false), time(epoch_ms))
}
