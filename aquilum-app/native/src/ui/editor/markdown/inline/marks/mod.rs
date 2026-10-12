mod atoms;
mod delimited;
#[cfg(test)]
mod tests;

use std::ops::Range;

use super::{Kind, Span};

fn span(kind: Kind, whole: Range<usize>, inner: Range<usize>) -> Span {
    Span { kind, whole, inner, target: None }
}

fn free(taken: &[Range<usize>], range: &Range<usize>) -> bool {
    !taken.iter().any(|t| t.start < range.end && range.start < t.end)
}

pub(super) fn extra(text: &str, taken: &[Range<usize>]) -> Vec<Span> {
    let mut out = atoms::html_pairs(text);
    out.extend(atoms::html_comments(text));
    out.extend(atoms::escapes(text));
    out.extend(atoms::scripts(text, '~', Kind::Sub));
    out.extend(atoms::scripts(text, '^', Kind::Sup));
    for (open, close) in delimited::paired(text, '=', true) {
        out.push(span(Kind::Highlight, open..close + 2, open + 2..close));
    }
    for (open, close) in delimited::paired(text, '%', false) {
        out.push(span(Kind::Comment, open..close + 2, open..close + 2));
    }
    for range in atoms::footnotes(text) {
        out.push(span(Kind::Footnote, range.clone(), range));
    }
    if let Some(range) = atoms::block_id(text) {
        out.push(span(Kind::BlockId, range.clone(), range));
    }
    out.retain(|s| free(taken, &s.whole));
    out
}
