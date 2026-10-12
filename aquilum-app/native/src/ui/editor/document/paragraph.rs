use std::collections::HashSet;
use std::ops::Range;

use masonry::core::{BrushIndex, StyleSet};
use masonry::kurbo::Vec2;
use masonry::parley::{Alignment, FontContext, Layout, LayoutContext};

use super::super::markdown::{Block, Decor, Fonts};
use super::{Embed, Fold, text};

pub(super) static PLAIN: Decor = Decor { x: 0.0, inset: 0.0, pad: (0.0, 0.0), hang: None, codes: Vec::new(), marks: Vec::new(), tags: Vec::new(), links: Vec::new(), caret: None };

pub struct Paragraph {
    pub start: usize,
    pub len: usize,
    pub top: f64,
    pub height: f64,
    pub layout: Option<Box<Layout<BrushIndex>>>,
    pub(super) decor: Option<Box<Decor>>,
    pub embed: Option<Box<Embed>>,
    pub(super) measured: Option<f32>,
    pub(super) embedded: Option<f32>,
    pub(super) lift: f32,
    pub(super) exact: bool,
    pub fold: Fold,
    pub(super) state: u32,
    pub block: Block,
}

pub(super) fn split(text: &str, base: usize) -> Vec<Paragraph> {
    let mut start = 0;
    let mut out = Vec::with_capacity(text.bytes().filter(|&b| b == b'\n').count() + 1);
    for (i, _) in text.match_indices('\n') {
        out.push(Paragraph::new(base + start, i - start));
        start = i + 1;
    }
    out.push(Paragraph::new(base + start, text.len() - start));
    out
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

    pub(super) fn caret(&self, caret: Option<usize>) -> Option<usize> {
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

    pub(super) fn text_height(&self) -> f64 {
        self.measured.map_or(0.0, f64::from)
    }

    pub(super) fn unbuild(&mut self) {
        self.layout = None;
        self.decor = None;
    }

    pub(super) fn near(&self, (top, bottom): (f64, f64)) -> bool {
        self.top + self.height > top && self.top < bottom
    }

    pub(super) fn forget(&mut self) {
        self.unbuild();
        self.measured = None;
        self.exact = false;
    }

    pub(super) fn estimate(&mut self, text: &str, line: f64, ch: f64, wrap: Option<f32>) {
        let chars = text[self.range()].chars().count() as f64;
        let lines = wrap.filter(|w| *w > 0.0).map_or(1.0, |w| (chars * ch / f64::from(w)).ceil().max(1.0));
        self.measured = Some((lines * line) as f32);
        self.exact = false;
    }

    pub(super) fn build(&mut self, text: &str, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>, composing: Option<(usize, &str)>, markdown: Option<(&Fonts, Option<usize>, &HashSet<String>)>) {
        let source = &text[self.range()];
        let (text, underline) = match composing {
            Some((at, preedit)) => {
                let local = at - self.start;
                (format!("{}{preedit}{}", &source[..local], &source[local..]), Some(local..local + preedit.len()))
            }
            None => (source.to_owned(), None),
        };
        let (layout, decor) = text::build(fcx, lcx, styles, &text, underline, width, Alignment::Start, markdown.map(|(fonts, local, unresolved)| (fonts, &self.block, local, unresolved)));
        self.measured = Some((f64::from(layout.height()) + decor.pad.0 + decor.pad.1) as f32);
        self.exact = true;
        self.layout = Some(Box::new(layout));
        self.decor = Some(Box::new(decor));
    }
}
