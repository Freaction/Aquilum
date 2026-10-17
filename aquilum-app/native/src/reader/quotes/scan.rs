use std::ops::Range;

use aquilum_core::markdown::fence::Fence;
use aquilum_core::search::pages::Pages;
use aquilum_core::text::content::Content;
use aquilum_core::markdown::quote as syntax;

use super::Quote;
use crate::links::reader::{LEGACY_SCHEME, SCHEME, parse_href, strip_scheme};

fn callout_end(line: &str, name: &str) -> Option<usize> {
    Some(syntax::quote(line)?.callout.filter(|c| c.name == name)?.range.end)
}

fn header_len(line: &str) -> Option<usize> {
    let end = callout_end(line, "quote")?;
    Some(line.len() - line[end..].trim_start().len())
}

fn link_at_end(body: &str) -> Option<(usize, &str)> {
    let trimmed = body.trim_end();
    let inner = trimmed.strip_suffix(')')?;
    let open = inner.rfind(&format!("({SCHEME}")).max(inner.rfind(&format!("({LEGACY_SCHEME}")))?;
    let href = &inner[open + 1..];
    (!href.contains(')') && strip_scheme(href).is_some_and(|rest| !rest.is_empty())).then_some((open, href))
}

fn parse_body(body: &str) -> Option<(usize, String, Option<String>)> {
    let (open, href) = link_at_end(body)?;
    let before = &body[..open];
    if let Some(label) = before.strip_suffix(']').and_then(|b| b.rfind('[').map(|at| (at, &b[at + 1..]))).filter(|(_, d)| !d.is_empty() && d.bytes().all(|c| c.is_ascii_digit())) {
        let (at, digits) = label;
        let text = &before[..at];
        if text.ends_with(char::is_whitespace) && !text.trim_end().is_empty() {
            return Some((text.trim_end().len(), href.to_owned(), Some(digits.to_owned())));
        }
    }
    for marker in ["[[→]]", "[[↗]]"] {
        if let Some(text) = before.strip_suffix(marker).filter(|t| !t.is_empty()) {
            return Some((text.trim_end().len(), href.to_owned(), None));
        }
    }
    None
}

fn parse_line(line: &str, start: usize) -> Option<(Range<usize>, String, Option<String>)> {
    let trimmed = line.trim_end();
    if let Some(header) = header_len(trimmed) {
        let (len, href, label) = parse_body(&trimmed[header..])?;
        return Some((start + header..start + header + len, href, label));
    }
    let marker = syntax::quote(trimmed)?.marker;
    let body = &trimmed[marker..];
    let body_at = start + marker;
    if let Some((len, href, label)) = parse_body(body) {
        return Some((body_at..body_at + len, href, label));
    }
    let (open, href) = link_at_end(body)?;
    let text = body[..open].strip_prefix('[')?.strip_suffix(']')?;
    (!text.is_empty()).then(|| (body_at + 1..body_at + 1 + text.len(), href.to_owned(), None))
}

fn is_book_header(line: &str) -> bool {
    callout_end(line, "book").is_some()
}

#[derive(Default)]
struct Scanner {
    quotes: Vec<Quote>,
    inside: Option<Fence>,
    skipping: bool,
}

impl Scanner {
    fn line(&mut self, start: usize, line: &str) {
        if self.skipping {
            if syntax::quote(line).is_some() && header_len(line.trim_end()).is_none() && !is_book_header(line) {
                return;
            }
            self.skipping = false;
        }
        let was = self.inside.is_some();
        self.inside = Fence::step(self.inside, line, usize::MAX);
        if was || self.inside.is_some() {
            return;
        }
        if let Some((text, href, label)) = parse_line(line, start)
            && let Some((cfi, book)) = parse_href(&href)
        {
            let label = label.unwrap_or_else(|| (self.quotes.len() + 1).to_string());
            self.quotes.push(Quote { line: start..start + line.len(), text, cfi, book, label });
        }
        self.skipping = header_len(line.trim_end()).is_some();
    }
}

pub fn find(text: &dyn Content) -> Vec<Quote> {
    let mut scanner = Scanner::default();
    let mut pages = Pages::of(text);
    while let Ok(Some(page)) = pages.next() {
        let mut at = page.start;
        for line in page.text.split_inclusive('\n') {
            scanner.line(at, line.strip_suffix('\n').unwrap_or(line));
            at += line.len();
        }
    }
    scanner.quotes
}
