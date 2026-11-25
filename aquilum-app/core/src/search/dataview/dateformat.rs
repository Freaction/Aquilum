use crate::search::note_date;
use crate::text::calendar::RU;

pub fn format(nanos: i64, pattern: &str) -> String {
    let (year, month, day, seconds) = note_date::to_civil(nanos);
    let weekday = note_date::weekday(nanos);
    let (hour, minute, second) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    let index = (month - 1).clamp(0, 11) as usize;
    let weekday_index = (weekday - 1).clamp(0, 6) as usize;

    let mut out = String::with_capacity(pattern.len() + 8);
    let symbols: Vec<char> = pattern.chars().collect();
    let mut cursor = 0usize;

    while cursor < symbols.len() {
        let symbol = symbols[cursor];
        if symbol == '\'' {
            cursor += 1;
            while cursor < symbols.len() && symbols[cursor] != '\'' {
                out.push(symbols[cursor]);
                cursor += 1;
            }
            cursor += 1;
            continue;
        }
        let run = symbols[cursor..]
            .iter()
            .take_while(|next| **next == symbol)
            .count();
        let token: String = std::iter::repeat_n(symbol, run).collect();
        match token.as_str() {
            "yyyy" | "yyy" => out.push_str(&format!("{year:04}")),
            "yy" => out.push_str(&format!("{:02}", year.rem_euclid(100))),
            "y" => out.push_str(&year.to_string()),
            "MMMM" => out.push_str(RU.months_of[index]),
            "MMM" => out.push_str(RU.months_short_of[index]),
            "MM" => out.push_str(&format!("{month:02}")),
            "M" => out.push_str(&month.to_string()),
            "LLLL" => out.push_str(RU.months[index]),
            "LLL" => out.push_str(RU.months_short[index]),
            "LL" => out.push_str(&format!("{month:02}")),
            "L" => out.push_str(&month.to_string()),
            "dd" => out.push_str(&format!("{day:02}")),
            "d" => out.push_str(&day.to_string()),
            "HH" => out.push_str(&format!("{hour:02}")),
            "H" => out.push_str(&hour.to_string()),
            "hh" => out.push_str(&format!("{:02}", hour_of_twelve(hour))),
            "h" => out.push_str(&hour_of_twelve(hour).to_string()),
            "mm" => out.push_str(&format!("{minute:02}")),
            "m" => out.push_str(&minute.to_string()),
            "ss" => out.push_str(&format!("{second:02}")),
            "s" => out.push_str(&second.to_string()),
            "EEEE" | "cccc" => out.push_str(RU.weekdays[weekday_index]),
            "EEE" | "ccc" => out.push_str(RU.weekdays_short[weekday_index]),
            "E" | "c" => out.push_str(&weekday.to_string()),
            _ => out.push_str(&token),
        }
        cursor += run;
    }
    out
}

fn hour_of_twelve(hour: i64) -> i64 {
    match hour % 12 {
        0 => 12,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::format;
    use crate::search::note_date;

    fn at(text: &str) -> i64 {
        note_date::parse(text).expect("дата разобрана")
    }

    #[test]
    fn a_day_with_a_short_month_reads_like_a_caption() {
        assert_eq!(format(at("2024-10-07"), "d MMM"), "7 окт.");
        assert_eq!(format(at("2024-10-07"), "d MMMM yyyy"), "7 октября 2024");
        assert_eq!(format(at("2024-10-07"), "LLLL"), "октябрь");
    }

    #[test]
    fn a_short_month_alone_takes_the_nominative_like_luxon() {
        assert_eq!(format(at("2024-05-07"), "d MMM"), "7 мая");
        assert_eq!(format(at("2024-05-07"), "LLL"), "май");
        assert_eq!(format(at("2024-07-07"), "LLL yyyy"), "июль 2024");
    }

    #[test]
    fn numbers_keep_their_width() {
        assert_eq!(format(at("2024-01-07 09:05"), "dd.MM.yyyy HH:mm"), "07.01.2024 09:05");
        assert_eq!(format(at("2024-01-07"), "d.M.yy"), "7.1.24");
    }

    #[test]
    fn the_weekday_is_known() {
        assert_eq!(format(at("2024-10-07"), "cccc"), "понедельник");
        assert_eq!(format(at("2024-10-13"), "EEE"), "вс");
    }

    #[test]
    fn quoted_text_passes_through_untouched() {
        assert_eq!(format(at("2024-10-07"), "d MMM 'года'"), "7 окт. года");
    }
}
