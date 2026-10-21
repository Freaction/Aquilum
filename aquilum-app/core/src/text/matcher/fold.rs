use std::collections::HashMap;
use std::sync::OnceLock;
use tantivy::tokenizer::{AsciiFoldingFilter, RawTokenizer, TextAnalyzer, TokenStream};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fold {
    Exact,
    Lower,
    Ascii,
}

const EXPANSION: usize = 8;
const TABLE: usize = 0x800;
const EXPANDS: char = '\u{FFFF}';

fn ascii_folds() -> &'static HashMap<char, Box<[char]>> {
    static FOLDS: OnceLock<HashMap<char, Box<[char]>>> = OnceLock::new();
    FOLDS.get_or_init(|| {
        let originals: Vec<char> = ('\u{80}'..='\u{FFFF}').collect();
        let joined: String = originals.iter().flat_map(|&original| [original, '\0']).collect();
        let mut folded = String::new();
        let mut analyzer = TextAnalyzer::builder(RawTokenizer::default()).filter(AsciiFoldingFilter).build();
        analyzer.token_stream(&joined).process(&mut |token| folded.clone_from(&token.text));
        originals
            .into_iter()
            .zip(folded.split('\0'))
            .filter(|&(original, folded)| folded.chars().ne([original]))
            .map(|(original, folded)| (original, folded.chars().collect()))
            .collect()
    })
}

pub(super) fn expand(original: char, fold: Fold, mut push: impl FnMut(char)) {
    match fold {
        Fold::Exact => push(original),
        Fold::Lower => push(original.to_lowercase().next().unwrap_or(original)),
        Fold::Ascii => {
            for lowered in original.to_lowercase() {
                match ascii_folds().get(&lowered) {
                    Some(folded) => folded.iter().for_each(|&part| push(part)),
                    None => push(lowered),
                }
            }
        }
    }
}

pub(super) fn sources(folded: char, fold: Fold) -> &'static [char] {
    static LOWER: OnceLock<HashMap<char, Vec<char>>> = OnceLock::new();
    static ASCII: OnceLock<HashMap<char, Vec<char>>> = OnceLock::new();
    let build = || {
        let mut sources: HashMap<char, Vec<char>> = HashMap::new();
        for original in (0..=char::MAX as u32).filter_map(char::from_u32) {
            expand(original, fold, |part| {
                if part != original {
                    sources.entry(part).or_default().push(original);
                }
            });
        }
        sources
    };
    let sources = match fold {
        Fold::Exact => return &[],
        Fold::Lower => LOWER.get_or_init(build),
        Fold::Ascii => ASCII.get_or_init(build),
    };
    sources.get(&folded).map_or(&[], Vec::as_slice)
}

fn table(fold: Fold) -> &'static [char] {
    static LOWER: OnceLock<Vec<char>> = OnceLock::new();
    static ASCII: OnceLock<Vec<char>> = OnceLock::new();
    let build = || {
        (0..TABLE as u32)
            .map(|code| {
                let mut parts = Vec::new();
                expand(char::from_u32(code).unwrap_or(EXPANDS), fold, |part| parts.push(part));
                if parts.len() == 1 { parts[0] } else { EXPANDS }
            })
            .collect()
    };
    match fold {
        Fold::Exact => &[],
        Fold::Lower => LOWER.get_or_init(build),
        Fold::Ascii => ASCII.get_or_init(build),
    }
}

#[derive(Clone)]
pub struct Folded<'a> {
    text: &'a str,
    at: usize,
    fold: Fold,
    table: &'static [char],
    expansion: [char; EXPANSION],
    len: u8,
    pos: u8,
    start: usize,
}

impl<'a> Folded<'a> {
    pub fn new(text: &'a str, fold: Fold) -> Self {
        Self::at(text, 0, fold)
    }

    pub(super) fn at(text: &'a str, at: usize, fold: Fold) -> Self {
        Self { text, at, fold, table: table(fold), expansion: ['\0'; EXPANSION], len: 0, pos: 0, start: at }
    }

    fn single(&self, original: char) -> Option<char> {
        if self.fold == Fold::Exact {
            return Some(original);
        }
        if original.is_ascii() {
            return Some(original.to_ascii_lowercase());
        }
        self.table.get(original as usize).copied().filter(|&folded| folded != EXPANDS)
    }
}

impl Iterator for Folded<'_> {
    type Item = (char, usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos == self.len {
            let original = self.text[self.at..].chars().next()?;
            self.start = self.at;
            self.at += original.len_utf8();
            if let Some(folded) = self.single(original) {
                return Some((folded, self.start, self.at));
            }
            let (expansion, mut len) = (&mut self.expansion, 0);
            expand(original, self.fold, |part| {
                if len < EXPANSION {
                    expansion[len] = part;
                    len += 1;
                }
            });
            (self.len, self.pos) = (len as u8, 0);
        }
        let folded = self.expansion[usize::from(self.pos)];
        self.pos += 1;
        Some((folded, self.start, self.at))
    }
}

pub fn fold(text: &str, fold: Fold) -> Vec<char> {
    Folded::new(text, fold).map(|(folded, ..)| folded).collect()
}
