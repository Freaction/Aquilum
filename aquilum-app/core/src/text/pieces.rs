use super::content::Content;
use std::io;
use std::ops::Range;

#[derive(Clone, Copy)]
struct Part {
    start: usize,
    len: usize,
    source: usize,
    offset: usize,
}

pub struct Pieces<S> {
    sources: Vec<S>,
    parts: Vec<Part>,
    len: usize,
}

impl<S: Content> Pieces<S> {
    pub fn new(sources: Vec<S>) -> Self {
        Self { sources, parts: Vec::new(), len: 0 }
    }

    pub fn source(&mut self, source: S) -> usize {
        self.sources.push(source);
        self.sources.len() - 1
    }

    pub fn push(&mut self, source: usize, range: Range<usize>) {
        if range.is_empty() {
            return;
        }
        self.parts.push(Part { start: self.len, len: range.len(), source, offset: range.start });
        self.len += range.len();
    }

    pub fn copy(&self, range: Range<usize>, into: &mut Pieces<S>) {
        self.each(range, |source, from| into.push(source, from));
    }

    pub fn take(&mut self) -> Pieces<S> {
        Pieces { sources: Vec::new(), parts: std::mem::take(&mut self.parts), len: std::mem::take(&mut self.len) }
    }

    fn each(&self, range: Range<usize>, mut visit: impl FnMut(usize, Range<usize>)) {
        if range.is_empty() {
            return;
        }
        let first = self.parts.partition_point(|part| part.start + part.len <= range.start);
        for part in &self.parts[first..] {
            if part.start >= range.end {
                break;
            }
            let from = range.start.max(part.start) - part.start;
            let to = range.end.min(part.start + part.len) - part.start;
            visit(part.source, part.offset + from..part.offset + to);
        }
    }
}

impl<S: Content> Content for Pieces<S> {
    fn byte_len(&self) -> usize {
        self.len
    }

    fn read(&self, range: Range<usize>, visit: &mut dyn FnMut(&[u8]) -> io::Result<()>) -> io::Result<()> {
        let mut result = Ok(());
        self.each(range, |source, from| {
            if result.is_ok() {
                result = self.sources[source].read(from, visit);
            }
        });
        result
    }
}

#[cfg(test)]
mod tests {
    use super::Pieces;
    use crate::text::content::{collect, Content};

    #[test]
    fn pieces_read_across_sources() {
        let mut pieces = Pieces::new(vec!["первый", "второй"]);
        pieces.push(0, 0..6);
        pieces.push(1, 6..12);
        pieces.push(0, 6..12);
        assert_eq!(collect(&pieces), "перройвый");
        let mut copy = Pieces::new(Vec::new());
        pieces.copy(2..10, &mut copy);
        assert_eq!(copy.byte_len(), 8);
        copy.sources = pieces.sources.clone();
        assert_eq!(collect(&copy), "ерро");
    }
}
