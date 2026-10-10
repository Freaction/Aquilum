mod cfi;
mod dom;
mod epub;
mod fb2;
pub mod flow;
pub mod hyphen;
pub mod page;
pub mod quotes;
pub mod session;
mod text;
mod zip;
#[cfg(test)]
mod tests;

use std::cell::OnceCell;
use std::collections::HashMap;
use std::path::Path;

pub use cfi::Point;
pub use dom::{Dom, NodeId};

pub struct Section {
    pub base: String,
    dom: OnceCell<Option<Dom>>,
}

pub struct Book {
    epub: Option<epub::Epub>,
    binaries: HashMap<String, String>,
    pub sections: Vec<Section>,
}

impl Book {
    pub fn open(path: &Path) -> Option<Book> {
        let data = std::fs::read(path).ok()?;
        let fb2 = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("fb2"));
        if fb2 {
            let text = text::decode(&data);
            let doc = text::parse_xml(&text)?;
            let sections = fb2::sections(&doc)
                .into_iter()
                .enumerate()
                .map(|(i, dom)| Section { base: format!("epubcfi(/6/{})", (i + 1) * 2), dom: OnceCell::from(Some(dom)) })
                .collect();
            let binaries = doc
                .root_element()
                .children()
                .filter(|c| c.is_element() && c.tag_name().name() == "binary")
                .filter_map(|c| Some((c.attribute("id")?.to_owned(), c.text().unwrap_or_default().to_owned())))
                .collect();
            return Some(Book { epub: None, binaries, sections });
        }
        let epub = epub::Epub::open(data)?;
        let sections = epub.sections.iter().map(|(_, base)| Section { base: base.clone(), dom: OnceCell::new() }).collect();
        Some(Book { epub: Some(epub), binaries: HashMap::new(), sections })
    }

    pub fn document(&self, index: usize) -> Option<&Dom> {
        let section = self.sections.get(index)?;
        section.dom.get_or_init(|| self.epub.as_ref().and_then(|e| e.section(index))).as_ref()
    }

    pub fn size(&self, index: usize) -> usize {
        match &self.epub {
            Some(epub) => epub.size(index),
            None => self.document(index).map_or(0, |dom| (0..dom.len() as NodeId).filter_map(|n| dom.text_of(n)).map(str::len).sum()),
        }
    }

    pub fn resource(&self, index: usize, src: &str) -> Option<Vec<u8>> {
        if let Some(epub) = &self.epub {
            return epub.resource(index, src);
        }
        use base64::Engine;
        let data = self.binaries.get(src.strip_prefix('#')?)?;
        let clean: String = data.chars().filter(|c| !c.is_whitespace()).collect();
        base64::engine::general_purpose::STANDARD.decode(clean).ok()
    }

    pub fn resolve(&self, source: &str) -> Option<(usize, Point, Point)> {
        let parsed = cfi::parse(source);
        let index = parsed.section()?;
        let (start, end) = cfi::resolve(self.document(index)?, &parsed.local())?;
        Some((index, start, end))
    }

    pub fn cfi(&self, index: usize, start: (NodeId, u32), end: (NodeId, u32)) -> Option<String> {
        let local = cfi::from_range(self.document(index)?, start, end);
        Some(cfi::join(&self.sections.get(index)?.base, &local.to_string()))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn text(&self, index: usize, start: Point, end: Point) -> Option<String> {
        Some(cfi::text(self.document(index)?, start, end))
    }
}
