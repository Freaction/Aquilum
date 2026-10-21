use super::excerpt::excerpt;
use crate::search::headings::find_spot;
use crate::search::pages::{Keys, Pages};
use crate::text::cancel::Cancel;
use crate::text::matcher::Matcher;
use crate::text::utf16::utf16_len;
use std::collections::HashSet;
use std::io;
use std::path::Path;

pub struct Located {
    pub utf16: usize,
    pub snippet: String,
    pub heading: Option<String>,
}

impl Located {
    pub fn new(page: &str, start_utf16: usize, at: usize, ended: bool, heading: Option<String>) -> Self {
        Located { utf16: start_utf16 + utf16_len(&page[..at]), snippet: excerpt(page, at, ended), heading }
    }
}

pub fn locate(path: &Path, matcher: Option<(&Matcher, &HashSet<String>)>, cancel: &Cancel) -> io::Result<Option<Located>> {
    let mut pages = Pages::open(path)?;
    let (note, mut keys) = (path.to_string_lossy(), Keys::default());
    let spot = find_spot(&mut pages, cancel, |page| match matcher {
        Some((matcher, hits)) => hits.contains(&keys.next(&note, page)).then(|| matcher.first(&page.text)).flatten().map(|found| found.start),
        None => Some(0),
    })?;
    Ok(spot.map(|spot| Located::new(&spot.page.text, spot.page.start_utf16, spot.at, pages.finished(), matcher.and(spot.heading))))
}
