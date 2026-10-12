use std::ops::Range;

use super::{Lang, Token};

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

pub fn tokens(line: &str, lang: Lang) -> Vec<(Range<usize>, Token)> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut prev = ' ';
    let mut in_tag = false;
    while i < line.len() {
        let rest = &line[i..];
        let c = rest.chars().next().unwrap_or(' ');
        if let Some(open) = lang.comment().iter().find(|m| rest.starts_with(**m)) {
            let close = match *open {
                "/*" => rest[2..].find("*/").map(|j| i + j + 4),
                "<!--" => rest[4..].find("-->").map(|j| i + j + 7),
                _ => None,
            };
            let end = close.unwrap_or(line.len());
            out.push((i..end, Token::Comment));
            i = end;
            continue;
        }
        if lang == Lang::Markup && c == '<' {
            let name = rest[1..].trim_start_matches(['/', '!']);
            let start = line.len() - name.len();
            let len = name.find(|c: char| !(c.is_alphanumeric() || c == '-')).unwrap_or(name.len());
            if len > 0 {
                out.push((start..start + len, Token::Keyword));
                in_tag = true;
                i = start + len;
                prev = 'a';
                continue;
            }
        }
        if lang == Lang::Markup && c == '>' {
            in_tag = false;
        }
        if matches!(c, '"' | '\'' | '`') && !(c == '\'' && lang == Lang::Rust && prev.is_alphanumeric()) {
            let mut j = i + 1;
            while j < line.len() {
                if bytes[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if bytes[j] == c as u8 {
                    j += 1;
                    break;
                }
                j += 1;
            }
            let end = j.min(line.len());
            let key = matches!(lang, Lang::Json) && line[end..].trim_start().starts_with(':');
            out.push((i..end, if key { Token::Property } else { Token::String }));
            i = end;
            prev = c;
            continue;
        }
        if c.is_ascii_digit() && !word_char(prev) {
            let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_')).map_or(line.len(), |j| i + j);
            out.push((i..end, Token::Number));
            i = end;
            prev = '0';
            continue;
        }
        if word_char(c) && !c.is_ascii_digit() {
            let end = rest.find(|c: char| !word_char(c) && c != '-' || (c == '-' && !matches!(lang, Lang::Css | Lang::Markup))).map_or(line.len(), |j| i + j);
            let word = &line[i..end];
            let after = line[end..].trim_start();
            let keywords = lang.keywords();
            let token = if (matches!(lang, Lang::Sql | Lang::Dataview) && keywords.contains(&word.to_ascii_lowercase().as_str())) || keywords.contains(&word) {
                Some(Token::Keyword)
            } else if in_tag || (lang == Lang::Css && after.starts_with(':')) || (lang == Lang::Yaml && after.starts_with(':') && line[..i].trim().is_empty()) {
                Some(Token::Property)
            } else if after.starts_with('(') || (lang == Lang::Rust && after.starts_with('!')) {
                Some(Token::Function)
            } else if prev == '.' {
                Some(Token::Property)
            } else if word.chars().next().is_some_and(|c| c.is_ascii_uppercase()) && !matches!(lang, Lang::Yaml | Lang::Shell | Lang::Sql | Lang::Dataview) {
                Some(Token::Type)
            } else {
                None
            };
            if let Some(token) = token {
                out.push((i..end, token));
            }
            i = end;
            prev = 'a';
            continue;
        }
        prev = c;
        i += c.len_utf8();
    }
    out
}
