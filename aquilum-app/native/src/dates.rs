use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};

use crate::i18n;

const RU: [&str; 12] =
    ["января", "февраля", "марта", "апреля", "мая", "июня", "июля", "августа", "сентября", "октября", "ноября", "декабря"];
const EN: [&str; 12] =
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

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
        (true, false) => format!("{} {}", at.day(), RU[month]),
        (true, true) => format!("{} {} {} г.", at.day(), RU[month], at.year()),
        (false, false) => format!("{} {}", EN[month], at.day()),
        (false, true) => format!("{} {}, {}", EN[month], at.day(), at.year()),
    }
}

pub fn date_time(epoch_ms: u64) -> String {
    if local(epoch_ms).is_none() {
        return String::new();
    }
    let joint = if i18n::is_russian() { " в " } else { " at " };
    format!("{}{joint}{}", day(epoch_ms, false), time(epoch_ms))
}
