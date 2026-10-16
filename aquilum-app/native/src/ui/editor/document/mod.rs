mod checkpoints;
mod classify;
mod edges;
mod edit;
mod embeds;
mod estimate;
mod full;
mod geometry;
mod heights;
mod laid;
mod layout;
mod lines;
mod paragraph;
#[cfg(feature = "memory-probe")]
mod probe;
#[cfg(test)]
mod corpus_blocks;
#[cfg(test)]
mod corpus_inline;
mod source;
mod scan;
mod sparse;
#[cfg(test)]
mod tests;
mod text;
mod window;
#[cfg(test)]
mod window_tests;

use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

use super::embed::View;
use super::markdown::{self, Fonts};
use super::resolve::Resolver;
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
    full: crop::Rope,
    base: usize,
    text: String,
    entry: markdown::State,
    windowed: bool,
    points: checkpoints::Checkpoints,
    above: f64,
    below: f64,
    view: (f64, f64),
    jump: Option<(usize, f64, f64)>,
    grew: bool,
    lines: lines::Lines,
    blocks: classify::Blocks,
    heights: heights::Heights,
    laid: sparse::Sparse<laid::Laid>,
    embeds: sparse::Sparse<Embed>,
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
    edits: Option<Vec<super::Edit>>,
    anchor: Option<(usize, f64)>,
    front: usize,
    dirty: Option<Range<usize>>,
    pub rebroke: bool,
    placed: bool,
    shift: f64,
    line: f64,
}

impl Document {
    pub fn new(text: &str) -> Self {
        Document::entire(crop::Rope::from(text))
    }

    pub fn windowed(text: crop::Rope) -> Self {
        Document::of(text, true)
    }

    pub fn entire(text: crop::Rope) -> Self {
        Document::of(text, false)
    }

    fn of(full: crop::Rope, windowed: bool) -> Self {
        let mut doc = Document {
            full,
            base: 0,
            text: String::new(),
            entry: markdown::State::default(),
            windowed,
            points: Default::default(),
            above: 0.0,
            below: 0.0,
            view: (0.0, 0.0),
            jump: None,
            grew: false,
            lines: lines::Lines::new(""),
            blocks: classify::Blocks::new(1),
            heights: heights::Heights::new(1),
            laid: Default::default(),
            embeds: Default::default(),
            width: None,
            markdown: None,
            resolver: None,
            unresolved: (Arc::default(), 0),
            editing: None,
            uncropped: None,
            resized: None,
            source: None,
            keep: (0.0, 0.0),
            pins: Vec::new(),
            edits: None,
            anchor: None,
            front: 0,
            dirty: None,
            rebroke: false,
            placed: false,
            shift: 0.0,
            line: 0.0,
        };
        doc.rescan();
        doc.load(0, None);
        doc
    }

    fn mark(&mut self, range: Range<usize>) {
        self.dirty = Some(match self.dirty.take() {
            Some(dirty) => dirty.start.min(range.start)..dirty.end.max(range.end),
            None => range,
        });
    }
}
