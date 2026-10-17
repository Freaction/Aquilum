use super::excerpt::excerpt;
use crate::search::headings::find_spot;
use crate::search::pages::Pages;
use crate::text::cancel::Cancel;
use crate::text::matcher::Matcher;
use crate::text::utf16::utf16_len;
use std::io;
use std::path::Path;

pub struct Located {
    pub utf16: usize,
    pub snippet: String,
    pub heading: Option<String>,
}

pub fn locate(path: &Path, matcher: Option<&Matcher>, cancel: &Cancel) -> io::Result<Option<Located>> {
    let mut pages = Pages::open(path)?;
    let spot = find_spot(&mut pages, cancel, |page| match matcher {
        Some(matcher) => matcher.first(&page.text).map(|found| found.start),
        None => Some(0),
    })?;
    Ok(spot.map(|spot| Located {
        utf16: spot.page.start_utf16 + utf16_len(&spot.page.text[..spot.at]),
        snippet: excerpt(&spot.page.text, spot.at, pages.finished()),
        heading: matcher.and(spot.heading),
    }))
}
