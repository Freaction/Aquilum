use aquilum_core::text::calendar::{EN, Names, RU};
use chrono::{Datelike, Timelike};

use crate::i18n;

const DEFAULT_DATE: &str = "YYYY-MM-DD";
const DEFAULT_TIME: &str = "HH:mm";
const LONGEST_TOKEN: usize = 4;

struct Locale {
    names: &'static Names,
    russian: bool,
    genitive: bool,
}

pub fn apply_placeholders(text: &str) -> String {
    apply_at(text, &chrono::Local::now(), i18n::is_russian())
}

fn apply_at<T: Datelike + Timelike>(text: &str, now: &T, russian: bool) -> String {
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
                out.push_str(&format_moment(pattern, now, russian));
            }
            _ => out.push_str(&rest[start..start + 2 + end + 2]),
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

fn month_follows_day(pattern: &str) -> bool {
    pattern.match_indices('D').any(|(at, _)| {
        let mut rest = &pattern[at + 1..];
        rest = rest.strip_prefix(['o', 'D']).unwrap_or(rest);
        let mut gaps = 0;
        loop {
            if let Some(next) = rest.strip_prefix(char::is_whitespace) {
                rest = next;
            } else if let Some(literal) = rest.strip_prefix('[').and_then(|inside| inside.split_once(']')).filter(|(text, _)| !text.contains('[')) {
                rest = literal.1;
            } else {
                break;
            }
            gaps += 1;
        }
        gaps > 0 && rest.starts_with("MMM")
    })
}

fn ordinal(day: u32, russian: bool) -> String {
    let suffix = match (russian, day % 10, day % 100) {
        (true, ..) => "-го",
        (false, _, 11..=13) => "th",
        (false, 1, _) => "st",
        (false, 2, _) => "nd",
        (false, 3, _) => "rd",
        (false, ..) => "th",
    };
    format!("{day}{suffix}")
}

fn meridiem(hour: u32, upper: bool, russian: bool) -> &'static str {
    match (russian, hour, upper) {
        (true, 0..=3, _) => "ночи",
        (true, 4..=11, _) => "утра",
        (true, 12..=16, _) => "дня",
        (true, ..) => "вечера",
        (false, 12.., true) => "PM",
        (false, _, true) => "AM",
        (false, 12.., false) => "pm",
        (false, ..) => "am",
    }
}

fn part<T: Datelike + Timelike>(token: &str, now: &T, locale: &Locale) -> Option<String> {
    let names = locale.names;
    let (month, weekday, hour12) = (now.month0() as usize, now.weekday().num_days_from_monday() as usize, (now.hour() + 11) % 12 + 1);
    let week = now.iso_week().week();
    Some(match token {
        "YYYY" => format!("{:04}", now.year()),
        "YY" => format!("{:02}", now.year().rem_euclid(100)),
        "Q" => (now.month0() / 3 + 1).to_string(),
        "MMMM" => (if locale.genitive { names.months_of } else { names.months })[month].to_owned(),
        "MMM" => (if locale.genitive { names.months_short_of } else { names.months_short })[month].to_owned(),
        "MM" => format!("{:02}", now.month()),
        "M" => now.month().to_string(),
        "Do" => ordinal(now.day(), locale.russian),
        "DDDD" => format!("{:03}", now.ordinal()),
        "DDD" => now.ordinal().to_string(),
        "DD" => format!("{:02}", now.day()),
        "D" => now.day().to_string(),
        "dddd" => names.weekdays[weekday].to_owned(),
        "ddd" => names.weekdays_short[weekday].to_owned(),
        "dd" => names.weekdays_min[weekday].to_owned(),
        "d" => now.weekday().num_days_from_sunday().to_string(),
        "E" => now.weekday().number_from_monday().to_string(),
        "WW" => format!("{week:02}"),
        "W" => week.to_string(),
        "HH" => format!("{:02}", now.hour()),
        "H" => now.hour().to_string(),
        "hh" => format!("{hour12:02}"),
        "h" => hour12.to_string(),
        "mm" => format!("{:02}", now.minute()),
        "m" => now.minute().to_string(),
        "ss" => format!("{:02}", now.second()),
        "s" => now.second().to_string(),
        "A" | "a" => meridiem(now.hour(), token == "A", locale.russian).to_owned(),
        _ => return None,
    })
}

fn format_moment<T: Datelike + Timelike>(pattern: &str, now: &T, russian: bool) -> String {
    let locale = Locale { names: if russian { &RU } else { &EN }, russian, genitive: russian && month_follows_day(pattern) };
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(c) = rest.chars().next() {
        if c == '['
            && let Some(close) = rest.find(']')
        {
            out.push_str(&rest[1..close]);
            rest = &rest[close + 1..];
            continue;
        }
        let token = (1..=LONGEST_TOKEN).rev().find_map(|len| {
            let token = rest.get(..len)?;
            Some((token, part(token, now, &locale)?))
        });
        match token {
            Some((token, text)) => {
                out.push_str(&text);
                rest = &rest[token.len()..];
            }
            None => {
                out.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> chrono::NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap().and_hms_opt(hour, minute, second).unwrap()
    }

    fn sunday() -> chrono::NaiveDateTime {
        at(2026, 10, 11, 9, 5, 7)
    }

    #[test]
    fn default_and_custom_formats_in_russian() {
        let now = sunday();
        assert_eq!(apply_at("Сегодня {{date}}, время {{time}}", &now, true), "Сегодня 2026-10-11, время 09:05");
        assert_eq!(apply_at("{{ Date : dddd, D MMMM YYYY [в] H:mm:ss }}", &now, true), "воскресенье, 11 октября 2026 в 9:05:07");
        assert_eq!(apply_at("{{date:100%}} {{title}} {{date:}}", &now, true), "100% {{title}} 2026-10-11");
        assert_eq!(apply_at("{{date:YY.M.D}} {{time:HH-mm}}", &now, true), "26.10.11 09-05");
    }

    #[test]
    fn day_of_year_weekday_and_week_tokens_follow_moment() {
        let now = sunday();
        assert_eq!(apply_at("{{date:dd ddd dddd E d}}", &now, true), "вс вс воскресенье 7 0");
        assert_eq!(apply_at("{{date:W WW Q DDD DDDD}}", &now, true), "41 41 4 284 284");
        assert_eq!(apply_at("{{date:DDD DDDD W WW}}", &at(2026, 1, 5, 0, 0, 0), true), "5 005 2 02");
    }

    #[test]
    fn twelve_hour_clock_and_ordinals() {
        assert_eq!(apply_at("{{time:h:mm A}} {{time:hh a}}", &sunday(), true), "9:05 утра 09 утра");
        assert_eq!(apply_at("{{time:h A}}", &at(2026, 1, 1, 0, 0, 0), true), "12 ночи");
        assert_eq!(apply_at("{{time:h A}}", &at(2026, 1, 1, 13, 0, 0), true), "1 дня");
        assert_eq!(apply_at("{{time:h A}}", &at(2026, 1, 1, 20, 0, 0), true), "8 вечера");
        assert_eq!(apply_at("{{date:Do}}", &sunday(), true), "11-го");
        assert_eq!(apply_at("{{time:h:mm A}} {{time:hh a}}", &sunday(), false), "9:05 AM 09 am");
        assert_eq!(apply_at("{{time:h a}} {{time:H A}}", &at(2026, 1, 1, 0, 0, 0), false), "12 am 0 AM");
        for (day, expected) in [(1, "1st"), (2, "2nd"), (3, "3rd"), (4, "4th"), (11, "11th"), (12, "12th"), (13, "13th"), (21, "21st"), (22, "22nd"), (23, "23rd")] {
            assert_eq!(apply_at("{{date:Do}}", &at(2026, 10, day, 0, 0, 0), false), expected);
        }
    }

    #[test]
    fn names_follow_the_language() {
        let now = sunday();
        assert_eq!(apply_at("{{date:dddd, MMMM Do YYYY h:mm a}}", &now, false), "Sunday, October 11th 2026 9:05 am");
        assert_eq!(apply_at("{{date:ddd MMM D dd}}", &now, false), "Sun Oct 11 Su");
        assert_eq!(apply_at("{{date:ddd MMM D dd}}", &now, true), "вс окт. 11 вс");
    }

    #[test]
    fn a_russian_month_is_genitive_only_in_a_pattern_with_a_day_before_it() {
        let may = at(2026, 5, 7, 0, 0, 0);
        for (pattern, expected) in [
            ("D MMMM", "7 мая"),
            ("DD MMMM", "07 мая"),
            ("Do MMM", "7-го мая"),
            ("D [of] MMMM", "7 of мая"),
            ("D[ ]MMMM", "7 мая"),
            ("MMMM D", "май 7"),
            ("MMMM", "май"),
            ("MMM", "май"),
            ("DD.MM MMMM", "07.05 май"),
            ("D.MMMM", "7.май"),
        ] {
            assert_eq!(apply_at(&format!("{{{{date:{pattern}}}}}"), &may, true), expected, "{pattern}");
        }
        assert_eq!(apply_at("{{date:D MMMM}}", &at(2026, 3, 7, 0, 0, 0), true), "7 марта");
        assert_eq!(apply_at("{{date:MMMM}}", &at(2026, 3, 7, 0, 0, 0), true), "март");
    }

    #[test]
    fn brackets_and_unknown_letters_pass_through() {
        let now = sunday();
        assert_eq!(apply_at("{{date:[Дата:] D}}", &now, true), "Дата: 11");
        assert_eq!(apply_at("{{date:[YYYY] YYYY}}", &now, true), "YYYY 2026");
        assert_eq!(apply_at("{{date:Q-я четверть}}", &now, true), "4-я четверть");
        assert_eq!(apply_at("{{date:YYYY-MM-DDTHH:mm:ss}}", &now, true), "2026-10-11T09:05:07");
    }
}
