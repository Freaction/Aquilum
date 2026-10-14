mod classify;
mod edit;
mod embeds;
mod layout;
mod paragraph;
#[cfg(feature = "memory-probe")]
mod probe;
#[cfg(test)]
mod corpus_blocks;
#[cfg(test)]
mod corpus_inline;
#[cfg(test)]
mod tests;
mod text;

use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

use super::embed::View;
use super::markdown::Fonts;
use super::resolve::Resolver;
use classify::States;
pub use paragraph::Paragraph;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fold {
    #[default]
    None,
    Head,
    Member,
    Above,
}

pub struct Embed {
    key: u64,
    pub view: View,
}

pub struct Document {
    text: String,
    paragraphs: Vec<Paragraph>,
    width: Option<f32>,
    pub markdown: Option<Fonts>,
    pub resolver: Option<Arc<dyn Resolver>>,
    unresolved: (Arc<HashSet<String>>, u64),
    pub editing: Option<(usize, (usize, usize))>,
    pub uncropped: Option<usize>,
    pub resized: Option<(usize, f64)>,
    pub source: Option<usize>,
    pub keep: (f64, f64),
    pub pins: Vec<usize>,
    shown: std::ops::RangeInclusive<usize>,
    edits: Option<Vec<super::Edit>>,
    anchor: Option<(usize, f64)>,
    states: States,
    front: usize,
    dirty: Option<Range<usize>>,
    pub rebroke: bool,
    laid: Range<usize>,
    held: Vec<usize>,
    placed: bool,
    shift: f64,
    line: f64,
}

impl Document {
    pub fn new(text: &str) -> Self {
        Document { text: text.to_owned(), paragraphs: paragraph::split(text, 0), width: None, markdown: None, resolver: None, unresolved: (Arc::default(), 0), editing: None, uncropped: None, resized: None, source: None, keep: (0.0, 0.0), pins: Vec::new(), shown: 0..=0, edits: None, anchor: None, states: States::default(), front: 0, dirty: None, rebroke: false, laid: 0..0, held: Vec::new(), placed: false, shift: 0.0, line: 0.0 }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn paragraphs(&self) -> &[Paragraph] {
        &self.paragraphs
    }

    pub fn paragraph(&self, index: usize) -> &Paragraph {
        &self.paragraphs[index]
    }

    pub fn height(&self) -> f64 {
        self.paragraphs.last().map_or(0.0, |p| p.top + p.height)
    }

    pub fn at_offset(&self, offset: usize) -> usize {
        self.paragraphs.partition_point(|p| p.start <= offset).saturating_sub(1)
    }

    pub fn at_y(&self, y: f64) -> usize {
        self.paragraphs.partition_point(|p| p.top <= y).saturating_sub(1)
    }

    pub fn take_shift(&mut self) -> f64 {
        std::mem::take(&mut self.shift)
    }

    pub fn anchor_at(&mut self, y: f64) {
        let p = &self.paragraphs[self.at_y(y + 8.0)];
        self.anchor = self.placed.then_some((p.start, p.top));
    }

    fn mark(&mut self, range: Range<usize>) {
        self.dirty = Some(match self.dirty.take() {
            Some(dirty) => dirty.start.min(range.start)..dirty.end.max(range.end),
            None => range,
        });
    }

    pub fn set_flavors(&mut self, callouts: bool, tags: bool) {
        if let Some(fonts) = self.markdown.as_mut()
            && (fonts.callouts != callouts || fonts.tags != tags)
        {
            fonts.callouts = callouts;
            fonts.tags = tags;
            self.invalidate_all();
        }
    }

    pub fn set_markdown(&mut self, fonts: Option<Fonts>) {
        self.markdown = fonts;
        self.invalidate_all();
        self.classify();
    }
}
