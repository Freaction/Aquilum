use super::{Fold, Matcher, Mode, Options, EXACT, IGNORE_CASE as LOWER};
use crate::text::cancel::{Cancel, Generation};

const PREFIX: Options = Options { fold: Fold::Ascii, mode: Mode::WordPrefix, ..EXACT };

fn all(query: &str, text: &str, options: Options) -> Vec<std::ops::Range<usize>> {
    Matcher::new(query, options).find_all(text)
}

#[test]
fn lower_fold_ignores_case_and_returns_byte_ranges() {
    let text = "Текст, ТЕКСТ и текст.";
    let found = all("текст", text, LOWER);
    assert_eq!(found.len(), 3);
    assert!(found.iter().all(|range| text[range.clone()].to_lowercase() == "текст"));
    assert_eq!(all("B", "ab Ab", LOWER), [1..2, 4..5]);
}

#[test]
fn matches_do_not_overlap_and_reach_the_end() {
    assert_eq!(all("aa", "aaaa", LOWER), [0..2, 2..4]);
    assert_eq!(all("aa", "xaa", LOWER), [1..3]);
    assert_eq!(all("ЖИК", "ёжик", LOWER), [2..8]);
    assert!(all("", "текст", LOWER).is_empty());
    assert!(all("текст", "тек", LOWER).is_empty());
}

#[test]
fn exact_fold_keeps_case() {
    assert_eq!(all("текст", "Текст текст", EXACT), [11..21]);
}

#[test]
fn ascii_fold_drops_latin_diacritics_like_the_index() {
    let text = "Crème brûlée";
    let found = all("BRÛLÉE", text, PREFIX);
    assert_eq!(&text[found[0].clone()], "brûlée");
    assert_eq!(all("brulee", text, PREFIX), found);
    assert_eq!(all("ёлка", "Ёлка", PREFIX), [0..8]);
    assert!(all("елка", "Ёлка", PREFIX).is_empty());
    assert!(all("иод", "йод", PREFIX).is_empty());
}

#[test]
fn ascii_fold_is_the_fold_of_the_index_analyzer() {
    use tantivy::tokenizer::{AsciiFoldingFilter, LowerCaser, SimpleTokenizer, TextAnalyzer, TokenStream};
    let text = "Ærøskøbing Straße ŁÓDŹ Ёжик йогурт ﬁnal";
    let mut analyzer = TextAnalyzer::builder(SimpleTokenizer::default()).filter(LowerCaser).filter(AsciiFoldingFilter).build();
    let mut indexed = Vec::new();
    analyzer.token_stream(text).process(&mut |token| indexed.push(token.text.clone()));
    let folded: String = super::fold(text, Fold::Ascii).into_iter().collect();
    assert_eq!(folded.split(' ').collect::<Vec<_>>(), indexed);
}

#[test]
fn word_prefix_skips_matches_inside_words() {
    let text = "Документ и псевдодокумент";
    let found = all("док", text, PREFIX);
    assert_eq!(found.len(), 1);
    assert_eq!(&text[found[0].clone()], "Док");
}

#[test]
fn whole_word_needs_both_boundaries() {
    let options = Options { mode: Mode::WholeWord, ..LOWER };
    assert_eq!(all("кот", "кот котик скот кот.", options), [0..6, 27..33]);
}

#[test]
fn the_longest_query_wins_where_several_match() {
    let matcher = Matcher::any(&["тест", "тестов", "мар"], PREFIX);
    let text = "Тестовый Markdown, тест";
    let found = matcher.find_all(text);
    assert_eq!(found.iter().map(|range| &text[range.clone()]).collect::<Vec<_>>(), ["Тестов", "тест"]);
}

#[test]
fn an_expanded_character_maps_back_to_the_whole_original() {
    let text = "Straße";
    assert_eq!(all("strasse", text, PREFIX), [0..text.len()]);
    assert_eq!(all("stras", text, PREFIX), [0..6]);
    assert_eq!(all("is", "İstanbul", LOWER), [0..3]);
}

#[test]
fn limit_stops_the_search() {
    let options = Options { limit: 2, ..LOWER };
    assert_eq!(all("a", "a a a a", options).len(), 2);
    assert!(all("a", "a", Options { limit: 0, ..LOWER }).is_empty());
}

#[test]
fn lines_are_searched_one_by_one_with_offsets_of_the_whole_text() {
    let text: String = (0..300).map(|i| format!("строка {i} Текст\n")).collect();
    let matcher = Matcher::new("текст", LOWER);
    let cancel = Cancel::never();
    let mut lines = matcher.lines(&cancel);
    text.split('\n').all(|line| lines.feed(line));
    assert_eq!(lines.finish(), matcher.find_all(&text));
}

#[test]
fn a_cancelled_line_search_stops_after_the_current_line() {
    let generation = Generation::default();
    let cancel = generation.next();
    let matcher = Matcher::new("а", LOWER);
    let mut lines = matcher.lines(&cancel);
    assert!(lines.feed("а а"));
    generation.stop();
    assert!(!lines.feed("а"));
    assert_eq!(lines.finish().len(), 3);
}

fn char_by_char(matcher: &Matcher, text: &str) -> Vec<std::ops::Range<usize>> {
    let (mut found, mut rest, mut before) = (Vec::new(), super::Folded::new(text, matcher.options.fold), None);
    while let Some((first, start, end)) = rest.next() {
        if matcher.starts_word(before) {
            if let Some((end, after, last)) = matcher.match_at(first, end, &rest) {
                found.push(start..end);
                (rest, before) = (after, Some(last));
                while rest.clone().next().is_some_and(|(_, from, _)| from < end) {
                    rest.next();
                }
                continue;
            }
        }
        before = Some(first);
    }
    found
}

struct Random(u64);

impl Random {
    fn below(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % bound as u64) as usize
    }

    fn text(&mut self, len: usize) -> String {
        const LETTERS: [char; 30] = ['a', 'A', 's', 'S', 'ß', 'æ', 'Æ', 'é', 'É', 'e', 'E', 'i', 'I', 'İ', 'k', 'K', '\u{212A}', 'щ', 'Щ', 'ё', 'Ё', 'т', 'Т', ' ', '-', '1', '½', '(', '⑴', 'ﬁ'];
        (0..len).map(|_| LETTERS[self.below(LETTERS.len())]).collect()
    }
}

#[test]
fn candidate_jumps_find_exactly_what_a_char_by_char_search_finds() {
    let mut random = Random(0x9E37_79B9_7F4A_7C15);
    for _ in 0..3000 {
        let len = random.below(40);
        let text = random.text(len);
        let needles: Vec<String> = (0..1 + random.below(2)).map(|_| {
            let len = 1 + random.below(3);
            random.text(len)
        }).collect();
        for fold in [Fold::Exact, Fold::Lower, Fold::Ascii] {
            for mode in [Mode::Substring, Mode::WordPrefix, Mode::WholeWord] {
                let matcher = Matcher::any(&needles, Options { fold, mode, limit: usize::MAX });
                assert_eq!(matcher.find_all(&text), char_by_char(&matcher, &text), "{text:?} {needles:?} {fold:?} {mode:?}");
            }
        }
    }
}
