use super::parse::Word;
use crate::search::error::SearchError;
use crate::search::index::SearchFields;
use levenshtein_automata::{Distance, LevenshteinAutomatonBuilder, DFA, SINK_STATE};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use tantivy::postings::TermInfo;
use tantivy::schema::Field;
use tantivy::{InvertedIndexReader, Searcher};
use tantivy_fst::Automaton;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Title,
    Headings,
    Body,
}

pub const PLACES: [Place; 3] = [Place::Title, Place::Headings, Place::Body];

impl Place {
    pub fn field(self, fields: SearchFields) -> Field {
        match self {
            Place::Title => fields.title,
            Place::Headings => fields.headings,
            Place::Body => fields.body,
        }
    }
}

#[derive(Default)]
pub struct Derived {
    pub any: u32,
    pub exact: u32,
    pub near: u32,
    pub doc_freq: u64,
    pub infos: Vec<Option<TermInfo>>,
}

pub type Terms = [BTreeMap<String, Derived>; 3];

struct Pattern {
    full: DFA,
    prefix: Option<DFA>,
    first: char,
    headings: bool,
}

static BUILDERS: [OnceLock<LevenshteinAutomatonBuilder>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];

impl Pattern {
    fn of(word: &Word) -> Self {
        let typos = match word.stem.chars().count() {
            _ if word.exact => 0,
            9.. => 2,
            5.. => 1,
            _ => 0,
        };
        let builder = BUILDERS[typos].get_or_init(|| LevenshteinAutomatonBuilder::new(typos as u8, true));
        let prefix = word.prefix.then(|| builder.build_prefix_dfa(&word.stem));
        Pattern { full: builder.build_dfa(&word.stem), prefix, first: word.stem.chars().next().unwrap_or_default(), headings: word.headings }
    }

    fn each(&self, inverted: &InvertedIndexReader, mut keep: impl FnMut(&str, &TermInfo, usize)) -> Result<(), SearchError> {
        let mut stream = inverted.terms().search(Dfa(self.prefix.as_ref().unwrap_or(&self.full))).into_stream()?;
        while stream.advance() {
            let Ok(term) = std::str::from_utf8(stream.key()) else { continue };
            if let Some(cost) = term.starts_with(self.first).then(|| self.cost(term)).flatten() {
                keep(term, stream.value(), cost);
            }
        }
        Ok(())
    }

    fn cost(&self, term: &str) -> Option<usize> {
        match (self.full.eval(term), self.prefix.as_ref().map(|dfa| dfa.eval(term))) {
            (Distance::Exact(typos), _) => Some(usize::from(typos)),
            (_, Some(Distance::Exact(typos))) => Some(usize::from(typos) + 1),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct Dfa<'a>(&'a DFA);

impl Automaton for Dfa<'_> {
    type State = u32;

    fn start(&self) -> u32 {
        self.0.initial_state()
    }

    fn is_match(&self, state: &u32) -> bool {
        matches!(self.0.distance(*state), Distance::Exact(_))
    }

    fn can_match(&self, state: &u32) -> bool {
        *state != SINK_STATE
    }

    fn accept(&self, state: &u32, byte: u8) -> u32 {
        self.0.transition(*state, byte)
    }
}

pub fn derive(searcher: &Searcher, fields: SearchFields, words: &[Word]) -> Result<Terms, SearchError> {
    let segments = searcher.segment_readers();
    let patterns: Vec<Pattern> = words.iter().map(Pattern::of).collect();
    let mut found = Terms::default();
    for (ord, segment) in segments.iter().enumerate() {
        for place in PLACES {
            let (inverted, terms) = (segment.inverted_index(place.field(fields))?, &mut found[place as usize]);
            for (index, pattern) in patterns.iter().enumerate().filter(|(_, pattern)| place == Place::Headings || !pattern.headings) {
                pattern.each(&inverted, |term, info, cost| {
                    if !terms.contains_key(term) {
                        terms.insert(term.to_owned(), Derived { infos: vec![None; segments.len()], ..Derived::default() });
                    }
                    let Some(derived) = terms.get_mut(term) else { return };
                    let bit = 1 << index;
                    derived.any |= bit;
                    derived.exact |= if cost == 0 { bit } else { 0 };
                    derived.near |= if cost <= 1 { bit } else { 0 };
                    if derived.infos[ord].is_none() {
                        derived.doc_freq += u64::from(info.doc_freq);
                        derived.infos[ord] = Some(info.clone());
                    }
                })?;
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::{Pattern, Word};

    fn cost(stem: &str, term: &str, prefix: bool) -> Option<usize> {
        Pattern::of(&Word { stem: stem.to_owned(), prefix, ..Word::default() }).cost(term)
    }

    #[test]
    fn an_edit_counts_once_and_a_longer_word_counts_only_as_a_prefix() {
        assert_eq!(cost("кот", "кот", true), Some(0));
        assert_eq!(cost("кот", "котлета", true), Some(1));
        assert_eq!(cost("кот", "кит", true), None);
        assert_eq!(cost("заметок", "заметок", false), Some(0));
        assert_eq!(cost("заметок", "заметк", false), Some(1));
        assert_eq!(cost("заметок", "замерить", false), None);
        assert_eq!(cost("замет", "заметками", true), Some(1));
        assert_eq!(cost("замет", "заметками", false), None);
        assert_eq!(cost("синхронизац", "синхранизац", false), Some(1));
    }
}
