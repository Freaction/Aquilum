mod embeds;
mod layout;
#[cfg(feature = "memory-probe")]
mod probe;
#[cfg(test)]
mod tests;
mod text;

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;

use masonry::core::{BrushIndex, StyleSet};
use masonry::kurbo::Vec2;
use masonry::parley::{Alignment, AlignmentOptions, FontContext, Layout, LayoutContext};

use super::embed::View;
use super::markdown::{self, Block, Decor, Fonts};
use super::resolve::Resolver;

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

#[derive(Default)]
struct States {
    list: Vec<markdown::State>,
    ids: HashMap<markdown::State, u32>,
}

impl States {
    fn id(&mut self, state: markdown::State) -> u32 {
        *self.ids.entry(state).or_insert_with(|| {
            self.list.push(state);
            (self.list.len() - 1) as u32
        })
    }

    fn get(&self, id: u32) -> Option<markdown::State> {
        self.list.get(id as usize).copied()
    }
}

static PLAIN: Decor = Decor { x: 0.0, inset: 0.0, pad: (0.0, 0.0), hang: None, codes: Vec::new(), tags: Vec::new(), links: Vec::new(), caret: None };

pub struct Paragraph {
    pub start: usize,
    pub len: usize,
    pub top: f64,
    pub height: f64,
    pub layout: Option<Box<Layout<BrushIndex>>>,
    decor: Option<Box<Decor>>,
    pub embed: Option<Box<Embed>>,
    measured: Option<f32>,
    embedded: Option<f32>,
    lift: f32,
    exact: bool,
    pub fold: Fold,
    state: u32,
    pub block: Block,
}

impl Paragraph {
    fn new(start: usize, len: usize) -> Self {
        Paragraph { start, len, top: 0.0, height: 0.0, layout: None, decor: None, embed: None, measured: None, embedded: None, lift: 0.0, exact: false, fold: Fold::None, state: u32::MAX, block: Block::Text }
    }

    pub fn decor(&self) -> &Decor {
        self.decor.as_deref().unwrap_or(&PLAIN)
    }

    pub fn origin(&self) -> Vec2 {
        let decor = self.decor();
        Vec2::new(decor.x, self.top + f64::from(self.lift) + decor.pad.0)
    }

    fn caret(&self, caret: Option<usize>) -> Option<usize> {
        caret.filter(|c| self.start <= *c && *c <= self.end()).map(|c| c - self.start)
    }

    pub fn end(&self) -> usize {
        self.start + self.len
    }

    pub fn range(&self) -> Range<usize> {
        self.start..self.end()
    }

    pub fn layout(&self) -> &Layout<BrushIndex> {
        self.layout.as_deref().expect("абзац сверстан")
    }

    fn text_height(&self) -> f64 {
        self.measured.map_or(0.0, f64::from)
    }

    fn unbuild(&mut self) {
        self.layout = None;
        self.decor = None;
    }

    fn near(&self, (top, bottom): (f64, f64)) -> bool {
        self.top + self.height > top && self.top < bottom
    }

    fn forget(&mut self) {
        self.unbuild();
        self.measured = None;
        self.exact = false;
    }

    fn estimate(&mut self, text: &str, line: f64, ch: f64, wrap: Option<f32>) {
        let chars = text[self.range()].chars().count() as f64;
        let lines = wrap.filter(|w| *w > 0.0).map_or(1.0, |w| (chars * ch / f64::from(w)).ceil().max(1.0));
        self.measured = Some((lines * line) as f32);
        self.exact = false;
    }

    fn build(&mut self, text: &str, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>, composing: Option<(usize, &str)>, markdown: Option<(&Fonts, Option<usize>, &HashSet<String>)>) {
        let source = &text[self.range()];
        let (text, underline) = match composing {
            Some((at, preedit)) => {
                let local = at - self.start;
                (format!("{}{preedit}{}", &source[..local], &source[local..]), Some(local..local + preedit.len()))
            }
            None => (source.to_owned(), None),
        };
        let (layout, decor) = text::build(fcx, lcx, styles, &text, underline, width, markdown.map(|(fonts, local, unresolved)| (fonts, &self.block, local, unresolved)));
        self.measured = Some((f64::from(layout.height()) + decor.pad.0 + decor.pad.1) as f32);
        self.exact = true;
        self.layout = Some(Box::new(layout));
        self.decor = Some(Box::new(decor));
    }
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
    dirty: Option<Range<usize>>,
    pub rebroke: bool,
    laid: Range<usize>,
    held: Vec<usize>,
    placed: bool,
    shift: f64,
    line: f64,
}

fn split(text: &str, base: usize) -> Vec<Paragraph> {
    let mut start = 0;
    let mut out = Vec::with_capacity(text.bytes().filter(|&b| b == b'\n').count() + 1);
    for (i, _) in text.match_indices('\n') {
        out.push(Paragraph::new(base + start, i - start));
        start = i + 1;
    }
    out.push(Paragraph::new(base + start, text.len() - start));
    out
}

impl Document {
    pub fn new(text: &str) -> Self {
        Document { text: text.to_owned(), paragraphs: split(text, 0), width: None, markdown: None, resolver: None, unresolved: (Arc::default(), 0), editing: None, uncropped: None, resized: None, source: None, keep: (0.0, 0.0), pins: Vec::new(), shown: 0..=0, edits: None, anchor: None, states: States::default(), dirty: None, rebroke: false, laid: 0..0, held: Vec::new(), placed: false, shift: 0.0, line: 0.0 }
    }

    pub fn report_edits(&mut self) {
        self.edits = Some(Vec::new());
    }

    pub fn changed(&mut self) -> super::TextAction {
        match self.edits.as_mut() {
            Some(edits) => super::TextAction::Edited(std::mem::take(edits)),
            None => super::TextAction::Changed(self.text.clone()),
        }
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

    pub fn set_text(&mut self, text: &str) {
        self.anchor = None;
        self.placed = false;
        self.states = States::default();
        if let Some(edits) = self.edits.as_mut() {
            edits.clear();
        }
        self.text = text.to_owned();
        self.paragraphs = split(text, 0);
        self.laid = 0..0;
        self.held.clear();
        self.mark(0..self.paragraphs.len());
        self.classify();
    }

    fn mark(&mut self, range: Range<usize>) {
        self.dirty = Some(match self.dirty.take() {
            Some(dirty) => dirty.start.min(range.start)..dirty.end.max(range.end),
            None => range,
        });
    }

    pub fn set_callouts(&mut self, enabled: bool) {
        if let Some(fonts) = self.markdown.as_mut()
            && fonts.callouts != enabled
        {
            fonts.callouts = enabled;
            self.invalidate_all();
        }
    }

    pub fn set_markdown(&mut self, fonts: Option<Fonts>) {
        self.markdown = fonts;
        self.invalidate_all();
        self.classify();
    }

    fn classify(&mut self) {
        self.classify_from(0, self.paragraphs.len());
    }

    fn classify_from(&mut self, from: usize, until: usize) {
        if self.markdown.is_none() {
            return;
        }
        let mut state = from.checked_sub(1).and_then(|i| self.states.get(self.paragraphs[i].state)).unwrap_or_default();
        for j in from..self.paragraphs.len() {
            let line = &self.text[self.paragraphs[j].range()];
            let next = self.paragraphs.get(j + 1).map(|n| &self.text[n.range()]);
            let block = state.step(line, next);
            let id = self.states.id(state);
            let p = &mut self.paragraphs[j];
            if p.block != block {
                p.block = block;
                p.forget();
            }
            let same = p.state == id;
            p.state = id;
            if j + 1 >= until && same {
                self.mark(from..j + 1);
                return;
            }
        }
        self.mark(from..self.paragraphs.len());
    }

    pub fn replace(&mut self, range: Range<usize>, insert: &str) {
        let first = self.at_offset(range.start);
        let last = self.at_offset(range.end);
        let from = self.paragraphs[first].start;
        let to = self.paragraphs[last].end();
        self.text.replace_range(range.clone(), insert);
        if let Some((at, _)) = self.anchor.as_mut() {
            if *at >= range.end {
                *at = *at + insert.len() - range.len();
            } else if *at > range.start {
                *at = range.start;
            }
        }
        if let Some(edits) = self.edits.as_mut() {
            edits.push(super::Edit { range: range.clone(), insert: insert.to_owned() });
        }
        let delta = insert.len() as isize - range.len() as isize;
        let new_end = (to as isize + delta) as usize;
        let fresh = split(&self.text[from..new_end], from);
        for p in &mut self.paragraphs[last + 1..] {
            p.start = (p.start as isize + delta) as usize;
        }
        let top = self.paragraphs[first].top;
        let mut fresh = fresh;
        for p in &mut fresh {
            p.top = top;
        }
        let count = fresh.len();
        self.paragraphs.splice(first..=last, fresh);
        let remap = |i: usize, end: bool| match i {
            i if i < first || end && i == first => i,
            i if i > last + usize::from(end) => i - (last + 1) + first + count,
            _ => first + if end { count } else { 0 },
        };
        self.dirty = self.dirty.take().map(|d| remap(d.start, false)..remap(d.end, true));
        self.laid = remap(self.laid.start, false)..remap(self.laid.end, true);
        for i in &mut self.held {
            *i = remap(*i, false);
        }
        self.mark(first..first + count);
        self.classify_from(first.saturating_sub(1), first + count);
    }
}
