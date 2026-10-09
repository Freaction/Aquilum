use super::dates::LocalDateTime;
use regex::{Captures, Regex};
use std::sync::LazyLock;

pub fn format(pattern: &str, date: &LocalDateTime, locale: &str) -> String {
    static TOKENS: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r"\[([^\]]*)\]|YYYY|MMMM|DDDD|dddd|gggg|GGGG|MMM|DDD|ddd|YY|MM|DD|HH|mm|ss|dd|ww|WW|M|D|H|m|s|d|w|W"
    ).expect("токены moment"));
    let ru = locale == "ru";
    let months = if ru { ["январь","февраль","март","апрель","май","июнь","июль","август","сентябрь","октябрь","ноябрь","декабрь"] }
        else { ["January","February","March","April","May","June","July","August","September","October","November","December"] };
    let short_months = if ru { ["янв.","февр.","март","апр.","май","июнь","июль","авг.","сент.","окт.","нояб.","дек."] }
        else { ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"] };
    let weekdays = if ru { ["воскресенье","понедельник","вторник","среда","четверг","пятница","суббота"] }
        else { ["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"] };
    let short_days = if ru { ["вс","пн","вт","ср","чт","пт","сб"] }
        else { ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"] };
    let minimal_days = if ru { ["вс","пн","вт","ср","чт","пт","сб"] }
        else { ["Su","Mo","Tu","We","Th","Fr","Sa"] };
    let weekday = date.weekday() as usize;
    let (iso_year, iso_week) = date.iso_week();
    let (local_year, local_week) = date.local_week();
    TOKENS.replace_all(pattern, |found: &Captures<'_>| {
        if let Some(literal) = found.get(1) { return literal.as_str().to_owned(); }
        match &found[0] {
            "YYYY" => format!("{:04}", date.year), "YY" => format!("{:02}", date.year % 100),
            "MMMM" => months[(date.month - 1) as usize].to_owned(),
            "MMM" => short_months[(date.month - 1) as usize].to_owned(),
            "MM" => format!("{:02}", date.month), "M" => date.month.to_string(),
            "DDDD" | "dddd" => weekdays[weekday].to_owned(),
            "DDD" | "ddd" => short_days[weekday].to_owned(), "dd" => minimal_days[weekday].to_owned(),
            "d" => weekday.to_string(), "DD" => format!("{:02}", date.day), "D" => date.day.to_string(),
            "HH" => format!("{:02}", date.hour), "H" => date.hour.to_string(),
            "mm" => format!("{:02}", date.minute), "m" => date.minute.to_string(),
            "ss" => format!("{:02}", date.second), "s" => date.second.to_string(),
            "w" => local_week.to_string(), "ww" => format!("{local_week:02}"),
            "W" => iso_week.to_string(), "WW" => format!("{iso_week:02}"),
            "gggg" => format!("{local_year:04}"), "GGGG" => format!("{iso_year:04}"),
            _ => found[0].to_owned(),
        }
    }).into_owned()
}

pub fn apply_placeholders(text: &str, date: &LocalDateTime, now: &LocalDateTime, title: &str, locale: &str) -> String {
    static PLACEHOLDERS: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r"(?i)\{\{\s*(date|time|title)\s*(?::([^}]*))?\}\}"
    ).expect("плейсхолдеры шаблона"));
    PLACEHOLDERS.replace_all(text, |found: &Captures<'_>| {
        let kind = found[1].to_ascii_lowercase();
        if kind == "title" { return if found.get(2).is_none() { title.to_owned() } else { found[0].to_owned() }; }
        let pattern = found.get(2).map(|p| p.as_str().trim()).filter(|p| !p.is_empty())
            .unwrap_or(if kind == "date" { "YYYY-MM-DD" } else { "HH:mm" });
        format(pattern, if kind == "date" { date } else { now }, locale)
    }).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_literals_and_placeholders() {
        let date = LocalDateTime { hour: 9, minute: 5, second: 3, ..LocalDateTime::parse("2026-10-08").unwrap() };
        assert_eq!(format("YYYY YY M MM D DD H HH m mm s ss [YYYY] Q", &date, "ru"), "2026 26 10 10 8 08 9 09 5 05 3 03 YYYY Q");
        assert_eq!(format("DDDD DDD d dd ddd dddd", &date, "ru"), "четверг чт 4 чт чт четверг");
        assert_eq!(format("DDDD DDD d dd ddd dddd", &date, "en"), "Thursday Thu 4 Th Thu Thursday");
        let boundary = LocalDateTime::parse("2021-01-03").unwrap();
        assert_eq!(format("w ww W WW gggg GGGG", &boundary, "en"), "2 02 53 53 2021 2020");
        let now = LocalDateTime { hour: 12, minute: 34, ..date.add_days(1) };
        assert_eq!(apply_placeholders("{{date}} {{ DATE : DDDD }} {{time}} {{time:YYYY-MM-DD}} {{title}} {{other}}", &date, &now, "08", "ru"), "2026-10-08 четверг 12:34 2026-10-09 08 {{other}}");
        assert_eq!(format("[M] [неделя] [", &date, "ru"), "M неделя [");
    }
    #[test]
    fn node_month_names_ru_and_en() {
        let ru_long = ["январь","февраль","март","апрель","май","июнь","июль","август","сентябрь","октябрь","ноябрь","декабрь"];
        let ru_short = ["янв.","февр.","март","апр.","май","июнь","июль","авг.","сент.","окт.","нояб.","дек."];
        let en_long = ["January","February","March","April","May","June","July","August","September","October","November","December"];
        let en_short = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
        for i in 0..12 {
            let date = LocalDateTime { month: i as i64 + 1, ..LocalDateTime::parse("2026-10-08").unwrap() };
            assert_eq!(format("MMMM MMM", &date, "ru"), format!("{} {}", ru_long[i], ru_short[i]));
            assert_eq!(format("MMMM MMM", &date, "en"), format!("{} {}", en_long[i], en_short[i]));
        }
    }
}
