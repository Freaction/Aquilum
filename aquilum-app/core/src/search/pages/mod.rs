use crate::files::document::normalize_tail;
use crate::markdown::fence::{Fence, INDENT_LIMIT};
use crate::text::content::{char_floor, Content, ContentReader};
use crate::text::utf16::utf16_len;
use std::fs::File;
use std::io::{self, Read};
use std::ops::Range;
use std::path::Path;

mod store;

pub use store::{count, delete, insert, key, place, remove, stored, PAGES_DDL};

const MIN_PAGE: usize = 256 * 1024;
const MAX_PAGE: usize = 4 * 1024 * 1024;
const HARD_PAGE: usize = 16 * 1024 * 1024;
const MAX_LINE: usize = 1 << 20;
const BLOCK: usize = 1 << 20;
const CUT_ONE_IN: u64 = 16;

pub struct Page {
    pub text: String,
    pub start: usize,
    pub start_utf16: usize,
    pub start_line: usize,
    pub chars: Range<usize>,
    pub first: bool,
    pub hash: blake3::Hash,
}

#[derive(Clone, Copy, Default)]
struct At {
    start: usize,
    utf16: usize,
    line: usize,
    chars: usize,
}

pub struct Pages<'a> {
    reader: Box<dyn Read + 'a>,
    buf: Vec<u8>,
    normalized: usize,
    scanned: usize,
    ended: bool,
    fence: Option<Fence>,
    at: At,
    first: bool,
}

impl<'a> Pages<'a> {
    pub fn open(path: &Path) -> io::Result<Self> {
        Ok(Pages::new(Box::new(File::open(path)?)))
    }

    pub fn of(content: &'a dyn Content) -> Self {
        Pages::new(Box::new(ContentReader::new(content)))
    }

    fn new(reader: Box<dyn Read + 'a>) -> Self {
        Self { reader, buf: Vec::new(), normalized: 0, scanned: 0, ended: false, fence: None, at: At::default(), first: true }
    }

    pub fn finished(&self) -> bool {
        self.ended && self.buf.is_empty()
    }

    pub fn next(&mut self) -> io::Result<Option<Page>> {
        if self.finished() && !self.first {
            return Ok(None);
        }
        loop {
            self.skip();
            let Some(end) = self.line_end()? else { break };
            if cuts(self.scanned, self.fence.is_none(), &self.buf[self.scanned..end]) {
                break;
            }
            self.step(end);
        }
        let rest = self.buf.split_off(self.scanned);
        let text = String::from_utf8(std::mem::replace(&mut self.buf, rest)).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        (self.normalized, self.scanned) = (self.normalized - text.len(), 0);
        let at = self.at;
        let chars = text.chars().count();
        self.at = At { start: at.start + text.len(), utf16: at.utf16 + utf16_len(&text), line: at.line + memchr::memchr_iter(b'\n', text.as_bytes()).count(), chars: at.chars + chars };
        let first = std::mem::take(&mut self.first);
        Ok(Some(Page { hash: blake3::hash(text.as_bytes()), text, start: at.start, start_utf16: at.utf16, start_line: at.line, chars: at.chars..self.at.chars, first }))
    }

    fn line_end(&mut self) -> io::Result<Option<usize>> {
        loop {
            let ready = &self.buf[self.scanned..self.normalized];
            if let Some(at) = memchr::memchr(b'\n', ready) {
                return Ok(Some(self.scanned + at + 1));
            }
            if ready.len() > MAX_LINE {
                return Ok(Some(self.scanned + char_floor(ready, MAX_LINE)));
            }
            if self.ended {
                return Ok((!ready.is_empty()).then_some(self.normalized));
            }
            self.fill()?;
        }
    }

    fn fill(&mut self) -> io::Result<()> {
        let start = self.buf.len();
        self.buf.resize(start + BLOCK, 0);
        let read = self.reader.read(&mut self.buf[start..])?;
        self.buf.truncate(start + read);
        self.ended = read == 0;
        self.normalized = normalize_tail(&mut self.buf, self.normalized, self.ended);
        Ok(())
    }

    fn step(&mut self, end: usize) {
        let line = &self.buf[self.scanned..end];
        if let Some(line) = std::str::from_utf8(line).ok().filter(|_| marker(line)) {
            self.fence = Fence::step(self.fence, line, INDENT_LIMIT);
        }
        self.scanned = end;
    }

    fn skip(&mut self) {
        let until = if self.fence.is_some() { HARD_PAGE } else { MIN_PAGE }.min(self.normalized);
        let mut from = self.scanned;
        while let Some(at) = self.buf.get(from..until).and_then(|rest| memchr::memchr2(b'`', b'~', rest)).map(|at| from + at) {
            let start = memchr::memrchr(b'\n', &self.buf[self.scanned..at]).map_or(self.scanned, |line| self.scanned + line + 1);
            if marker(&self.buf[start..=at]) {
                self.scanned = start;
                return;
            }
            from = at + 1;
        }
        if let Some(line) = self.buf.get(self.scanned..until).and_then(|rest| memchr::memrchr(b'\n', rest)) {
            self.scanned += line + 1;
        }
    }
}

pub fn read_chars(pages: &mut Pages<'_>, wanted: Range<usize>) -> io::Result<(String, usize)> {
    let (mut text, mut total) = (String::new(), 0);
    while let Some(page) = pages.next()? {
        if page.chars.end > wanted.start && page.chars.start < wanted.end {
            let skip = wanted.start.saturating_sub(page.chars.start);
            text.extend(page.text.chars().skip(skip).take(wanted.end.min(page.chars.end) - page.chars.start - skip));
        }
        total = page.chars.end;
    }
    Ok((text, total))
}

fn cuts(len: usize, outside: bool, line: &[u8]) -> bool {
    let block = outside && line.first().is_some_and(|byte| !byte.is_ascii_whitespace());
    len >= HARD_PAGE || block && len >= MIN_PAGE && (len >= MAX_PAGE || fnv(line) % CUT_ONE_IN == 0)
}

fn marker(line: &[u8]) -> bool {
    line.iter().find(|byte| !matches!(byte, b' ' | b'\t')).is_some_and(|byte| matches!(byte, b'`' | b'~'))
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3))
}

#[cfg(test)]
mod tests;
