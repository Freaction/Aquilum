use crate::search::analyzer::stem_surface_labels;
use crate::search::schema;
use super::stopwords::is_stop_term;
use std::path::Path;

const MIN_SIGNIFICANT_WORDS: usize = 15;
const MIN_BODY_TERM_CHARS: usize = 3;
const MIN_FILENAME_CHARS: usize = 2;
const MAX_BODY_TERMS: usize = 3;

#[derive(Debug, Clone)]
pub struct ExtractedQueries {
    pub title: Option<String>,
    pub embedded_titles: Vec<String>,
    pub body_terms: Vec<String>,
    pub word_count: usize,
}

pub fn extract_queries(text: &str, document_path: Option<&str>) -> ExtractedQueries {
    let title = document_path
        .and_then(title_from_path)
        .map(|value| sanitize_query(&value))
        .filter(|value| is_usable_filename(value));
    let embedded_titles = embedded_titles(text, title.as_deref());
    let body = strip_leading_heading(text);
    let labels = stem_surface_labels(title.as_deref().unwrap_or(""), &body);

    let mut ranked: Vec<(String, u32)> = schema::token_statistics(&body)
        .frequencies
        .into_iter()
        .map(|(stem, frequency)| {
            let label = labels.get(&stem).cloned().unwrap_or_else(|| stem.clone());
            (label, frequency)
        })
        .filter(|(label, _)| is_usable_body_term(label))
        .collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut body_terms = Vec::new();
    for (label, _) in ranked {
        if body_terms.len() >= MAX_BODY_TERMS {
            break;
        }
        if title
            .as_deref()
            .is_some_and(|title| title.eq_ignore_ascii_case(&label))
        {
            continue;
        }
        if title.as_deref().is_some_and(|title| {
            title.to_lowercase().contains(&label.to_lowercase())
        }) {
            continue;
        }
        if body_terms.iter().any(|existing: &String| {
            existing.eq_ignore_ascii_case(&label)
        }) {
            continue;
        }
        body_terms.push(label);
    }

    ExtractedQueries {
        title,
        embedded_titles,
        body_terms,
        word_count: significant_word_count(text),
    }
}

fn embedded_titles(text: &str, filename: Option<&str>) -> Vec<String> {
    let mut titles = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("]]" ) else { break };
        let raw = &after_start[..end];
        let raw = raw.split('|').next().unwrap_or_default().split('#').next().unwrap_or_default();
        let title = sanitize_query(raw);
        if is_usable_filename(&title)
            && !filename.is_some_and(|value| value.eq_ignore_ascii_case(&title))
            && !titles.iter().any(|value: &String| value.eq_ignore_ascii_case(&title))
        {
            titles.push(title);
        }
        rest = &after_start[end + 2..];
    }
    titles
}

pub fn has_enough_text(word_count: usize) -> bool {
    word_count >= MIN_SIGNIFICANT_WORDS
}

fn title_from_path(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    let stem = Path::new(&normalized)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if stem.starts_with("__empty_tab__") {
        return None;
    }
    Some(stem.to_owned())
}

fn is_usable_filename(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.chars().count() < MIN_FILENAME_CHARS {
        return false;
    }
    if !trimmed.chars().any(|ch| ch.is_alphanumeric()) {
        return false;
    }
    !(trimmed.contains(':') && trimmed.chars().any(|ch| ch.is_ascii_digit()))
}

fn sanitize_query(value: &str) -> String {
    let mut cleaned = String::with_capacity(value.len());
    let mut digits = String::new();

    let flush_digits = |cleaned: &mut String, digits: &mut String| {
        if digits.chars().count() > 1 {
            cleaned.push_str(digits);
        }
        digits.clear();
    };

    for ch in value.chars() {
        if ch == '#' {
            flush_digits(&mut cleaned, &mut digits);
            continue;
        }
        if ch.is_ascii_digit() {
            digits.push(ch);
            continue;
        }
        flush_digits(&mut cleaned, &mut digits);
        if ch.is_alphabetic() || matches!(ch, ' ' | '-' | '\'' | '(' | ')') {
            cleaned.push(ch);
        } else if !cleaned.ends_with(' ') {
            cleaned.push(' ');
        }
    }
    flush_digits(&mut cleaned, &mut digits);

    cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|ch: char| matches!(ch, '-' | '\'' | '(' | ')'))
        .trim()
        .to_owned()
}

fn strip_leading_heading(text: &str) -> String {
    let trimmed = text.trim();
    for line in trimmed.lines() {
        let stripped = line.trim();
        if stripped
            .strip_prefix('#')
            .map(str::trim)
            .is_some_and(|heading| !heading.is_empty() && !heading.starts_with('#'))
        {
            return trimmed.replacen(line, "", 1);
        }
    }
    trimmed.to_owned()
}

fn is_usable_body_term(label: &str) -> bool {
    let trimmed = label.trim();
    let lower = trimmed.to_lowercase();
    if trimmed.chars().count() < MIN_BODY_TERM_CHARS {
        return false;
    }
    if !trimmed.chars().any(|ch| ch.is_alphabetic()) {
        return false;
    }
    if trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }
    if looks_like_date_or_time(trimmed) {
        return false;
    }
    !is_stop_term(&lower)
}

fn looks_like_date_or_time(value: &str) -> bool {
    let mut has_digit = false;
    let mut has_alpha = false;
    let mut separators = 0usize;
    for ch in value.chars() {
        if ch.is_ascii_digit() {
            has_digit = true;
        } else if ch.is_alphabetic() {
            has_alpha = true;
        } else if matches!(ch, ':' | '.' | '/' | '-') {
            separators += 1;
        }
    }
    (has_digit && !has_alpha && separators > 0) || (value.contains(':') && has_digit)
}

fn significant_word_count(text: &str) -> usize {
    schema::token_statistics(text)
        .frequencies
        .values()
        .map(|&count| count as usize)
        .sum()
}

pub fn is_cyrillic(ch: char) -> bool {
    matches!(ch, '\u{0400}'..='\u{04FF}' | '\u{0500}'..='\u{052F}')
}

pub fn body_term_lang(term: &str) -> &'static str {
    if term.chars().any(is_cyrillic) {
        "ru"
    } else {
        "en"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_first_with_digits_allowed() {
        assert!(is_usable_filename("20030907"));
        assert!(is_usable_filename("Accordion2"));
        assert!(is_usable_filename("UI-2024"));
        assert!(!is_usable_filename("12:30"));
    }

    #[test]
    fn filename_plus_body_terms() {
        let text = "*20030907*\nТип: #учебник\n\n- Single Expand — секция\n- Panel — контент\n".repeat(4);
        let extracted = extract_queries(&text, Some(r"D:\notes\Accordion.md"));
        assert_eq!(extracted.title.as_deref(), Some("Accordion"));
        assert!(extracted.body_terms.len() <= MAX_BODY_TERMS);
        assert!(!extracted
            .body_terms
            .iter()
            .any(|q| matches!(q.to_lowercase().as_str(), "управляет" | "доступен" | "несколько")));
    }

    #[test]
    fn numeric_filename_is_title() {
        let text = "квантовая механика ".repeat(8);
        let extracted = extract_queries(&text, Some(r"notes\20030907.md"));
        assert_eq!(extracted.title.as_deref(), Some("20030907"));
    }

    #[test]
    fn title_from_path_strips_extension() {
        assert_eq!(title_from_path(r"C:\vault\Accordion.md").as_deref(), Some("Accordion"));
        assert_eq!(title_from_path(r"notes\Accordion.md").as_deref(), Some("Accordion"));
        assert_eq!(title_from_path("__empty_tab__1"), None);
    }

    #[test]
    fn filename_query_removes_formatting_and_noise() {
        assert_eq!(sanitize_query("✨ **Макар Чудра**, #литература 1"), "Макар Чудра литература");
        assert_eq!(sanitize_query("1984 — роман"), "1984 роман");
        assert_eq!(sanitize_query("Accordion2"), "Accordion");
    }

    #[test]
    fn filename_allows_wiki_lookup_for_a_short_note() {
        let extracted = extract_queries("# Макар Чудра", Some(r"D:\notes\Макар Чудра.md"));
        assert_eq!(extracted.title.as_deref(), Some("Макар Чудра"));
        assert!(!has_enough_text(extracted.word_count));
    }

    #[test]
    fn extracts_titles_from_wikilinks_in_any_markdown_construct() {
        let extracted = extract_queries(
            "> [!book] [[ Победители недр]]\n\n- [[Другой роман|подпись]]\n[[Тема#раздел]]",
            Some(r"D:\notes\Заметка.md"),
        );
        assert_eq!(
            extracted.embedded_titles,
            ["Победители недр", "Другой роман", "Тема"]
        );
    }

    #[test]
    fn ignores_malformed_and_duplicate_wikilinks() {
        let extracted = extract_queries(
            "[[Макар Чудра]] [[Макар Чудра|рассказ]] [[незакрытая ссылка",
            None,
        );
        assert_eq!(extracted.embedded_titles, ["Макар Чудра"]);
    }
}
