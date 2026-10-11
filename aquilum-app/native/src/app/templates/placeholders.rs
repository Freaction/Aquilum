use chrono::{Datelike, Timelike};

const DEFAULT_DATE: &str = "YYYY-MM-DD";
const DEFAULT_TIME: &str = "HH:mm";
const MONTHS: [&str; 12] = ["январь", "февраль", "март", "апрель", "май", "июнь", "июль", "август", "сентябрь", "октябрь", "ноябрь", "декабрь"];
const MONTHS_SHORT: [&str; 12] = ["янв.", "февр.", "март", "апр.", "май", "июнь", "июль", "авг.", "сент.", "окт.", "нояб.", "дек."];
const DAYS: [&str; 7] = ["понедельник", "вторник", "среда", "четверг", "пятница", "суббота", "воскресенье"];
const DAYS_SHORT: [&str; 7] = ["пн", "вт", "ср", "чт", "пт", "сб", "вс"];
const TOKENS: [&str; 16] = ["YYYY", "MMMM", "DDDD", "MMM", "DDD", "YY", "MM", "DD", "HH", "mm", "ss", "M", "D", "H", "m", "s"];

pub fn apply_placeholders(text: &str) -> String {
    apply_at(text, &chrono::Local::now())
}

fn apply_at<T: Datelike + Timelike>(text: &str, now: &T) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str("{{");
            rest = after;
            continue;
        };
        let inner = &after[..end];
        let (kind, pattern) = inner.split_once(':').map_or((inner, None), |(k, p)| (k, Some(p)));
        let default = match kind.trim().to_lowercase().as_str() {
            "date" => Some(DEFAULT_DATE),
            "time" => Some(DEFAULT_TIME),
            _ => None,
        };
        match default {
            Some(default) if !pattern.is_some_and(|p| p.contains('}')) => {
                let pattern = pattern.map(str::trim).filter(|p| !p.is_empty()).unwrap_or(default);
                out.push_str(&format_moment(pattern, now));
            }
            _ => out.push_str(&rest[start..start + 2 + end + 2]),
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

fn part<T: Datelike + Timelike>(token: &str, now: &T) -> String {
    match token {
        "YYYY" => format!("{:04}", now.year()),
        "YY" => format!("{:02}", now.year().rem_euclid(100)),
        "MMMM" => MONTHS[now.month0() as usize].to_owned(),
        "MMM" => MONTHS_SHORT[now.month0() as usize].to_owned(),
        "MM" => format!("{:02}", now.month()),
        "M" => now.month().to_string(),
        "DDDD" => DAYS[now.weekday().num_days_from_monday() as usize].to_owned(),
        "DDD" => DAYS_SHORT[now.weekday().num_days_from_monday() as usize].to_owned(),
        "DD" => format!("{:02}", now.day()),
        "D" => now.day().to_string(),
        "HH" => format!("{:02}", now.hour()),
        "H" => now.hour().to_string(),
        "mm" => format!("{:02}", now.minute()),
        "m" => now.minute().to_string(),
        "ss" => format!("{:02}", now.second()),
        _ => now.second().to_string(),
    }
}

fn format_moment<T: Datelike + Timelike>(pattern: &str, now: &T) -> String {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(c) = rest.chars().next() {
        if c == '['
            && let Some(close) = rest.find(']')
        {
            out.push_str(&rest[1..close]);
            rest = &rest[close + 1..];
        } else if let Some(token) = TOKENS.iter().find(|t| rest.starts_with(**t)) {
            out.push_str(&part(token, now));
            rest = &rest[token.len()..];
        } else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moment() -> chrono::NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 11).unwrap().and_hms_opt(9, 5, 7).unwrap()
    }

    #[test]
    fn formats_like_the_tauri_version() {
        let now = moment();
        assert_eq!(apply_at("Сегодня {{date}}, время {{time}}", &now), "Сегодня 2026-10-11, время 09:05");
        assert_eq!(apply_at("{{ Date : DDDD, D MMMM YYYY [в] H:mm:ss }}", &now), "воскресенье, 11 октябрь 2026 в 9:05:07");
        assert_eq!(apply_at("{{date:100%}} {{title}} {{date:}}", &now), "100% {{title}} 2026-10-11");
        assert_eq!(apply_at("{{date:YY.M.D}} {{time:HH-mm}}", &now), "26.10.11 09-05");
    }
}
