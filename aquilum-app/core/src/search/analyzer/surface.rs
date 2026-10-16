use super::indexing_tokenizer;
use std::cmp::Ordering;
use std::collections::HashMap;
use tantivy::tokenizer::{TextAnalyzer, TokenStream};

pub fn stem_surface_labels(title: &str, body: &str) -> HashMap<String, String> {
    let mut surfaces = HashMap::<&str, u32>::new();
    for text in [title, body] {
        for surface in alphanumeric_words(text).filter(|surface| surface.chars().nth(1).is_some()) {
            *surfaces.entry(surface).or_insert(0) += 1;
        }
    }
    let mut analyzer = indexing_tokenizer();
    let mut best = HashMap::<String, (&str, u32)>::new();
    for (surface, count) in surfaces {
        let Some(stem) = stem_token(&mut analyzer, surface).filter(|stem| stem.chars().nth(1).is_some()) else {
            continue;
        };
        let chosen = best.entry(stem).or_insert((surface, count));
        if preference(&(surface, count), chosen) == Ordering::Greater {
            *chosen = (surface, count);
        }
    }
    best.into_iter().map(|(stem, (surface, _))| (stem, surface.to_owned())).collect()
}

fn preference((left, left_count): &(&str, u32), (right, right_count): &(&str, u32)) -> Ordering {
    left_count
        .cmp(right_count)
        .then_with(|| right.chars().count().cmp(&left.chars().count()))
        .then_with(|| right.cmp(left))
}

fn stem_token(analyzer: &mut TextAnalyzer, surface: &str) -> Option<String> {
    let mut stem = None;
    analyzer.token_stream(surface).process(&mut |token| {
        if stem.is_none() && !token.text.is_empty() {
            stem = Some(token.text.clone());
        }
    });
    stem
}

fn alphanumeric_words(text: &str) -> impl Iterator<Item = &str> {
    WordSpans { text, cursor: 0 }
}

struct WordSpans<'a> {
    text: &'a str,
    cursor: usize,
}

impl<'a> Iterator for WordSpans<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        while self.cursor < self.text.len() {
            let ch = self.text[self.cursor..].chars().next()?;
            if ch.is_alphanumeric() {
                break;
            }
            self.cursor += ch.len_utf8();
        }
        if self.cursor >= self.text.len() {
            return None;
        }
        let start = self.cursor;
        while self.cursor < self.text.len() {
            let ch = self.text[self.cursor..].chars().next()?;
            if !ch.is_alphanumeric() {
                break;
            }
            self.cursor += ch.len_utf8();
        }
        Some(&self.text[start..self.cursor])
    }
}

#[cfg(test)]
mod tests {
    use super::stem_surface_labels;

    #[test]
    fn maps_russian_stems_to_surface_words() {
        let labels = stem_surface_labels("новая книга", "про новую книгу");
        assert_eq!(labels.get("книг").map(String::as_str), Some("книга"));
        assert_eq!(labels.get("нов").map(String::as_str), Some("новая"));
    }

    #[test]
    fn prefers_the_most_frequent_surface_form() {
        let labels = stem_surface_labels("", "книгу книгу книгу книга");
        assert_eq!(labels.get("книг").map(String::as_str), Some("книгу"));
    }

    #[test]
    fn maps_english_stems_to_surface_words() {
        let labels = stem_surface_labels("machines", "about machines and machine");
        assert_eq!(labels.get("machin").map(String::as_str), Some("machines"));
    }
}
