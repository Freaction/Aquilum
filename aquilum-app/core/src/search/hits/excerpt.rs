pub const CHARS_BEFORE_MATCH: usize = 72;
pub const CHARS_AFTER_MATCH: usize = 188;

pub fn excerpt(text: &str, at: usize, ended: bool) -> String {
    let start = text[..at]
        .char_indices()
        .rev()
        .nth(CHARS_BEFORE_MATCH - 1)
        .map_or(0, |(index, _)| index);
    let end = text[at..]
        .char_indices()
        .nth(CHARS_AFTER_MATCH)
        .map_or(text.len(), |(index, _)| at + index);
    let shown = text[start..end].split_whitespace().collect::<Vec<_>>().join(" ");
    let before = if start > 0 { "..." } else { "" };
    let after = if end < text.len() || !ended { "..." } else { "" };
    format!("{before}{shown}{after}")
}

#[cfg(test)]
mod tests {
    use super::excerpt;

    #[test]
    fn keeps_the_match_visible_and_marks_both_cuts() {
        let body = format!("{}needle {}", "word ".repeat(100), "tail ".repeat(100));
        let shown = excerpt(&body, body.find("needle").unwrap(), true);
        assert!(shown.contains("needle") && shown.starts_with("...") && shown.ends_with("..."));
    }

    #[test]
    fn a_short_text_is_shown_whole_without_cuts() {
        assert_eq!(excerpt("  одна\nстрока  ", 0, true), "одна строка");
        assert_eq!(excerpt("", 0, true), "");
        assert_eq!(excerpt("начало", 0, false), "начало...");
    }

    #[test]
    fn exactly_the_allowed_number_of_characters_before_the_match_needs_no_cut() {
        let text = format!("{}ц", "ж".repeat(72));
        assert!(!excerpt(&text, 144, true).starts_with("..."));
        let text = format!("{}ц", "ж".repeat(73));
        assert!(excerpt(&text, 146, true).starts_with("..."));
    }
}
