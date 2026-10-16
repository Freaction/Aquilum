use std::collections::HashMap;
use std::sync::OnceLock;

pub const SHY: char = '\u{ad}';
const MIN_WORD: usize = 5;

#[derive(Default)]
struct Node {
    next: Vec<(char, u32)>,
    weights: Option<Box<[u8]>>,
}

struct Language {
    nodes: Vec<Node>,
    exceptions: HashMap<String, Vec<usize>>,
    left: usize,
    right: usize,
}

impl Language {
    fn new(patterns: &str, exceptions: &str, left: usize, right: usize) -> Language {
        let mut nodes = vec![Node::default()];
        for pattern in patterns.split_whitespace() {
            let mut at = 0;
            let mut weights = vec![0u8];
            for ch in pattern.chars() {
                match ch.to_digit(10) {
                    Some(d) => *weights.last_mut().expect("вес") = d as u8,
                    None => {
                        at = match nodes[at].next.binary_search_by_key(&ch, |&(c, _)| c) {
                            Ok(i) => nodes[at].next[i].1 as usize,
                            Err(i) => {
                                let id = nodes.len();
                                nodes[at].next.insert(i, (ch, id as u32));
                                nodes.push(Node::default());
                                id
                            }
                        };
                        weights.push(0);
                    }
                }
            }
            nodes[at].weights = Some(weights.into_boxed_slice());
        }
        let exceptions = exceptions
            .split_whitespace()
            .map(|word| {
                let mut points = Vec::new();
                let mut count = 0;
                for ch in word.chars() {
                    if ch == '-' {
                        points.push(count);
                    } else {
                        count += 1;
                    }
                }
                (word.replace('-', ""), points)
            })
            .collect();
        Language { nodes, exceptions, left, right }
    }

    fn points(&self, word: &[char]) -> Vec<usize> {
        let lower: String = word.iter().flat_map(|c| c.to_lowercase()).collect();
        if let Some(points) = self.exceptions.get(&lower) {
            return points.clone();
        }
        let mut letters = Vec::with_capacity(word.len() + 2);
        letters.push('.');
        letters.extend(lower.chars());
        letters.push('.');
        if letters.len() != word.len() + 2 {
            return Vec::new();
        }
        let mut weights = vec![0u8; letters.len() + 1];
        for start in 0..letters.len() {
            let mut at = 0;
            for &ch in &letters[start..] {
                let Ok(i) = self.nodes[at].next.binary_search_by_key(&ch, |&(c, _)| c) else { break };
                at = self.nodes[at].next[i].1 as usize;
                for (i, &w) in self.nodes[at].weights.iter().flatten().enumerate() {
                    let slot = &mut weights[start + i];
                    *slot = (*slot).max(w);
                }
            }
        }
        (self.left..=word.len().saturating_sub(self.right)).filter(|&i| weights[i + 1] % 2 == 1).collect()
    }
}

fn russian() -> &'static Language {
    static LANGUAGE: OnceLock<Language> = OnceLock::new();
    LANGUAGE.get_or_init(|| Language::new(include_str!("../../assets/hyph/hyph-ru.pat.txt"), include_str!("../../assets/hyph/hyph-ru.hyp.txt"), 2, 2))
}

fn english() -> &'static Language {
    static LANGUAGE: OnceLock<Language> = OnceLock::new();
    LANGUAGE.get_or_init(|| Language::new(include_str!("../../assets/hyph/hyph-en-us.pat.txt"), include_str!("../../assets/hyph/hyph-en-us.hyp.txt"), 2, 3))
}

fn language(word: &[char]) -> Option<&'static Language> {
    if word.iter().all(|c| matches!(c, 'а'..='я' | 'А'..='Я' | 'ё' | 'Ё')) {
        Some(russian())
    } else if word.iter().all(char::is_ascii_alphabetic) {
        Some(english())
    } else {
        None
    }
}

pub fn breaks(text: &str) -> Vec<(usize, char)> {
    let mut out = Vec::new();
    let mut word: Vec<(usize, char)> = Vec::new();
    let mut flush = |word: &mut Vec<(usize, char)>| {
        if word.len() >= MIN_WORD {
            let letters: Vec<char> = word.iter().map(|&(_, c)| c).collect();
            if let Some(language) = language(&letters) {
                out.extend(language.points(&letters).into_iter().map(|p| (word[p].0, SHY)));
            }
        }
        word.clear();
    };
    for (at, ch) in text.char_indices() {
        if ch.is_alphabetic() {
            word.push((at, ch));
        } else {
            flush(&mut word);
        }
    }
    flush(&mut word);
    out
}

pub fn apply(text: &str, breaks: &[(usize, char)]) -> String {
    let mut out = String::with_capacity(text.len() + breaks.len() * 2);
    let mut from = 0;
    for &(at, ch) in breaks {
        out.push_str(&text[from..at]);
        out.push(ch);
        from = at;
    }
    out.push_str(&text[from..]);
    out
}

pub fn to_layout(breaks: &[(usize, char)], byte: usize) -> usize {
    byte + breaks.iter().take_while(|b| b.0 <= byte).map(|b| b.1.len_utf8()).sum::<usize>()
}

pub fn to_text(breaks: &[(usize, char)], byte: usize) -> usize {
    let mut shift = 0;
    for &(at, ch) in breaks {
        if at + shift + ch.len_utf8() > byte {
            break;
        }
        shift += ch.len_utf8();
    }
    byte - shift
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(text: &str) -> String {
        apply(text, &breaks(text)).replace(SHY, "-")
    }

    #[test]
    fn hyphenates_russian_and_english() {
        assert_eq!(show("государство"), "го-су-дар-ство");
        assert_eq!(show("Справедливостью,"), "Спра-вед-ли-во-стью,");
        assert_eq!(show("hyphenation"), "hy-phen-ation");
        assert_eq!(show("кот и pen"), "кот и pen");
    }

    #[test]
    fn maps_offsets_both_ways() {
        let text = "Это государство";
        let mut inserted = breaks(text);
        inserted[1].1 = '-';
        let out = apply(text, &inserted);
        for (byte, _) in text.char_indices() {
            let mapped = to_layout(&inserted, byte);
            assert!(out.is_char_boundary(mapped));
            assert_eq!(to_text(&inserted, mapped), byte);
        }
        assert_eq!(to_text(&inserted, out.len()), text.len());
    }
}
