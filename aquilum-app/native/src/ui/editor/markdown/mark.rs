use std::ops::Range;

use super::inline::{Kind, Span, inline};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Bold,
    Italic,
    Strike,
}

impl Mark {
    pub fn marker(self) -> &'static str {
        match self {
            Mark::Bold => "**",
            Mark::Italic => "*",
            Mark::Strike => "~~",
        }
    }

    fn kind(self) -> Kind {
        match self {
            Mark::Bold => Kind::Strong,
            Mark::Italic => Kind::Emphasis,
            Mark::Strike => Kind::Strike,
        }
    }
}

pub fn enclosing(line: &str, mark: Mark, selection: Range<usize>) -> Option<(Range<usize>, Range<usize>)> {
    let spans = inline(line);
    let of = || spans.iter().filter(|s| s.kind == mark.kind());
    let around = |s: &&Span| {
        if selection.is_empty() {
            s.whole.start < selection.start && selection.start < s.whole.end
        } else {
            s.whole.start <= selection.start && selection.end <= s.whole.end
        }
    };
    let covered = |s: &&Span| !selection.is_empty() && selection.start <= s.whole.start && s.whole.end <= selection.end;
    of().filter(around).min_by_key(|s| s.whole.len()).or_else(|| of().filter(covered).max_by_key(|s| s.whole.len())).map(|s| (s.whole.clone(), s.inner.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mark_is_found_whatever_characters_made_it() {
        assert_eq!(enclosing("a **b** c", Mark::Bold, 4..5), Some((2..7, 4..5)));
        assert_eq!(enclosing("a __b__ c", Mark::Bold, 4..5), Some((2..7, 4..5)));
        assert_eq!(enclosing("a _b_ c", Mark::Italic, 3..4), Some((2..5, 3..4)));
        assert_eq!(enclosing("a ~~b~~ c", Mark::Strike, 4..5), Some((2..7, 4..5)));
    }

    #[test]
    fn a_caret_must_be_strictly_inside_but_a_selection_may_touch_the_edges() {
        assert_eq!(enclosing("**b**", Mark::Bold, 0..0), None);
        assert_eq!(enclosing("**b**", Mark::Bold, 5..5), None);
        assert_eq!(enclosing("**b**", Mark::Bold, 3..3), Some((0..5, 2..3)));
        assert_eq!(enclosing("**b**", Mark::Bold, 0..5), Some((0..5, 2..3)));
        assert_eq!(enclosing("x **b** y", Mark::Bold, 0..9), Some((2..7, 4..5)));
    }

    #[test]
    fn only_the_asked_kind_counts_and_the_innermost_wins() {
        assert_eq!(enclosing("*b*", Mark::Bold, 1..2), None);
        assert_eq!(enclosing("**a **b** c**", Mark::Bold, 6..7), Some((4..9, 6..7)));
    }
}
