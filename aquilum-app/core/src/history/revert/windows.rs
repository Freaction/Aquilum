use super::Map;
use crate::text::content::{find_byte, read_bytes, Content};
use std::io;
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub previous: Range<usize>,
    pub version: Range<usize>,
    pub current: Range<usize>,
}

pub fn windows(version: &dyn Content, own: &Map, later: &Map) -> io::Result<Vec<Window>> {
    let back: Map = own.iter().map(|(previous, version)| (version.clone(), previous.clone())).collect();
    let mut grown = back.iter().map(|(seed, _)| grow(version, seed.clone(), &back, later)).collect::<io::Result<Vec<_>>>()?;
    grown.sort_by_key(|span| (span.start, span.end));
    let mut spans: Vec<Range<usize>> = Vec::new();
    for span in grown {
        match spans.last_mut() {
            Some(last) if span.start < last.end => last.end = last.end.max(span.end),
            _ => spans.push(span),
        }
    }
    let (mut to_previous, mut to_current) = (Walk::default(), Walk::default());
    Ok(spans
        .into_iter()
        .map(|span| Window {
            previous: to_previous.project(&span, &back),
            current: to_current.project(&span, later),
            version: span,
        })
        .collect())
}

fn grow(version: &dyn Content, mut span: Range<usize>, own: &Map, later: &Map) -> io::Result<Range<usize>> {
    loop {
        let lines = line_start(version, span.start)?..line_end(version, span.end)?;
        let mut next = lines.clone();
        for pairs in [own, later] {
            let near = pairs.partition_point(|(old, _)| old.end < lines.start)..pairs.partition_point(|(old, _)| old.start <= lines.end);
            for (old, _) in pairs[near].iter().filter(|(old, _)| crosses(&lines, old)) {
                next = next.start.min(old.start)..next.end.max(old.end);
            }
        }
        if next == span {
            return Ok(span);
        }
        span = next;
    }
}

fn crosses(span: &Range<usize>, old: &Range<usize>) -> bool {
    match (span.is_empty(), old.is_empty()) {
        (true, true) => span.start == old.start,
        (false, true) => span.start < old.start && old.start < span.end,
        _ => old.start < span.end && span.start < old.end,
    }
}

#[derive(Default)]
struct Walk {
    next: usize,
    shift: isize,
}

impl Walk {
    fn project(&mut self, span: &Range<usize>, pairs: &Map) -> Range<usize> {
        self.pass(pairs, |old| old.end <= span.start && !crosses(span, old));
        let start = span.start.saturating_add_signed(self.shift);
        self.pass(pairs, |old| crosses(span, old));
        start..span.end.saturating_add_signed(self.shift)
    }

    fn pass(&mut self, pairs: &Map, take: impl Fn(&Range<usize>) -> bool) {
        while let Some((old, new)) = pairs.get(self.next).filter(|(old, _)| take(old)) {
            self.shift += new.len() as isize - old.len() as isize;
            self.next += 1;
        }
    }
}

fn boundary(text: &dyn Content, at: usize) -> io::Result<bool> {
    Ok(at == 0 || at == text.byte_len() || read_bytes(text, at - 1..at)? == b"\n")
}

fn line_start(text: &dyn Content, at: usize) -> io::Result<usize> {
    if boundary(text, at)? {
        return Ok(at);
    }
    Ok(find_byte(text, 0..at, true, |byte| byte == b'\n')?.map_or(0, |index| index + 1))
}

fn line_end(text: &dyn Content, at: usize) -> io::Result<usize> {
    if boundary(text, at)? {
        return Ok(at);
    }
    Ok(find_byte(text, at..text.byte_len(), false, |byte| byte == b'\n')?.map_or(text.byte_len(), |index| index + 1))
}
