use std::ops::Range;

pub(super) fn byte_of(text: &str, unit: usize) -> usize {
    let mut units = 0;
    for (byte, c) in text.char_indices() {
        if units >= unit {
            return byte;
        }
        units += c.len_utf16();
    }
    text.len()
}

pub(super) fn utf16_of(text: &str, byte: usize) -> usize {
    text[..byte].encode_utf16().count()
}

fn prefix(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

fn suffix(a: &[u8], b: &[u8]) -> usize {
    a.iter().rev().zip(b.iter().rev()).take_while(|(x, y)| x == y).count()
}

pub(super) fn distance(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let start = prefix(a, b);
    let end = suffix(&a[start..], &b[start..]);
    a.len() + b.len() - 2 * (start + end)
}

pub(super) fn changed<'a>(old: &str, old_selection: &Range<usize>, new: &'a str, new_selection: &Range<usize>) -> (usize, usize, &'a str) {
    let (a, b) = (old.as_bytes(), new.as_bytes());
    let mut start = prefix(a, b).min(old_selection.start).min(new_selection.start);
    while !old.is_char_boundary(start) {
        start -= 1;
    }
    let limit = (a.len() - old_selection.end).min(b.len() - new_selection.end);
    let mut end = suffix(&a[start..], &b[start..]).min(limit);
    while !old.is_char_boundary(a.len() - end) {
        end -= 1;
    }
    (start, a.len() - end, &new[start..b.len() - end])
}
