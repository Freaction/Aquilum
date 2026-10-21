use std::ops::Range;

use aquilum_core::text::cancel::Cancel;
use aquilum_core::text::matcher::{EXACT, IGNORE_CASE, Matcher, Mode, Options};

pub(super) const FIND: Options = Options { limit: 19_999, ..IGNORE_CASE };
pub(super) const SELECTION: Options = EXACT;
pub(super) const HOVER: Options = Options { mode: Mode::WordPrefix, ..IGNORE_CASE };

pub(super) fn find_in(text: &crop::Rope, matcher: &Matcher, cancel: &Cancel) -> Vec<Range<usize>> {
    let mut lines = matcher.lines(cancel);
    crate::rope::each_block(text.byte_slice(..), |block| lines.feed(block));
    lines.finish()
}

#[derive(Clone, Debug, Default)]
pub struct Search {
    pub query: String,
    pub only: Option<usize>,
}

impl Search {
    pub fn ranges(&self, text: &str, seen: &mut usize) -> Vec<Range<usize>> {
        let found = Matcher::new(&self.query, FIND).find_all(text);
        let first = *seen;
        *seen += found.len();
        match self.only {
            None => found,
            Some(k) => found.into_iter().enumerate().filter(|(i, _)| first + i == k).map(|(_, r)| r).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FIND, find_in};
    use aquilum_core::text::cancel::Cancel;
    use aquilum_core::text::matcher::Matcher;

    #[test]
    fn the_whole_rope_is_searched_with_offsets_of_the_text() {
        let text: String = (0..3000).map(|i| format!("строка {i} Текст\n")).collect();
        let matcher = Matcher::new("текст", FIND);
        let found = find_in(&crop::Rope::from(text.as_str()), &matcher, &Cancel::never());
        assert_eq!(found.len(), 3000);
        assert_eq!(found, matcher.find_all(&text));
    }
}
