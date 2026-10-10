mod embeds;
mod text;

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

pub struct Paragraph {
    pub start: usize,
    pub len: usize,
    pub layout: Option<Layout<BrushIndex>>,
    pub top: f64,
    pub height: f64,
    pub block: Block,
    pub decor: Decor,
    pub fold: Fold,
    pub embed: Option<Box<Embed>>,
    lift: f64,
}

impl Paragraph {
    fn new(start: usize, len: usize) -> Self {
        Paragraph { start, len, layout: None, top: 0.0, height: 0.0, block: Block::Text, decor: Decor::default(), fold: Fold::None, embed: None, lift: 0.0 }
    }

    pub fn origin(&self) -> Vec2 {
        Vec2::new(self.decor.x, self.top + self.lift + self.decor.pad.0)
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
        self.layout.as_ref().expect("абзац сверстан")
    }

    fn text_height(&self) -> f64 {
        f64::from(self.layout().height()) + self.decor.pad.0 + self.decor.pad.1
    }
}

pub struct Document {
    text: String,
    paragraphs: Vec<Paragraph>,
    width: Option<f32>,
    pub markdown: Option<Fonts>,
    pub resolver: Option<Arc<dyn Resolver>>,
    pub editing: Option<(usize, (usize, usize))>,
    pub uncropped: Option<usize>,
    pub resized: Option<(usize, f64)>,
}

fn split(text: &str, base: usize) -> Vec<Paragraph> {
    let mut start = 0;
    let mut out = Vec::new();
    for (i, _) in text.match_indices('\n') {
        out.push(Paragraph::new(base + start, i - start));
        start = i + 1;
    }
    out.push(Paragraph::new(base + start, text.len() - start));
    out
}

impl Document {
    pub fn new(text: &str) -> Self {
        Document { text: text.to_owned(), paragraphs: split(text, 0), width: None, markdown: None, resolver: None, editing: None, uncropped: None, resized: None }
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

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_owned();
        self.paragraphs = split(text, 0);
        self.classify();
    }

    pub fn set_markdown(&mut self, fonts: Option<Fonts>) {
        self.markdown = fonts;
        self.invalidate_all();
        self.classify();
    }

    fn classify(&mut self) {
        if self.markdown.is_none() {
            return;
        }
        let blocks = markdown::classify(self.paragraphs.iter().map(|p| &self.text[p.range()]));
        for (p, block) in self.paragraphs.iter_mut().zip(blocks) {
            if p.block != block {
                p.block = block;
                p.layout = None;
            }
        }
    }

    pub fn replace(&mut self, range: Range<usize>, insert: &str) {
        let first = self.at_offset(range.start);
        let last = self.at_offset(range.end);
        let from = self.paragraphs[first].start;
        let to = self.paragraphs[last].end();
        self.text.replace_range(range.clone(), insert);
        let delta = insert.len() as isize - range.len() as isize;
        let new_end = (to as isize + delta) as usize;
        let fresh = split(&self.text[from..new_end], from);
        for p in &mut self.paragraphs[last + 1..] {
            p.start = (p.start as isize + delta) as usize;
        }
        self.paragraphs.splice(first..=last, fresh);
        self.classify();
    }

    pub fn layout(&mut self, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>, compose: Option<(usize, &str)>, caret: Option<usize>) {
        if let Some(fonts) = self.markdown.as_mut()
            && fonts.ch == 0.0
        {
            fonts.ch = text::measure_ch(fcx, lcx, fonts);
        }
        let rebreak = self.width != width;
        self.width = width;
        for p in &mut self.paragraphs {
            let composing = compose.filter(|(at, _)| p.range().contains(at) || *at == p.end());
            let local = self.markdown.as_ref().and_then(|_| p.caret(caret));
            let moved = self.markdown.is_some() && local != p.decor.caret;
            let wrap = width.map(|w| (w - p.decor.inset as f32).max(0.0));
            match (&mut p.layout, composing) {
                (Some(layout), None) if rebreak && !moved => {
                    layout.break_all_lines(wrap);
                    layout.align(wrap, Alignment::Start, AlignmentOptions::default());
                }
                (Some(_), None) if !moved => {}
                (_, composing) => {
                    let source = &self.text[p.range()];
                    let (text, underline) = match composing {
                        Some((at, preedit)) => {
                            let local = at - p.start;
                            (format!("{}{preedit}{}", &source[..local], &source[local..]), Some(local..local + preedit.len()))
                        }
                        None => (source.to_owned(), None),
                    };
                    let (layout, decor) = text::build(fcx, lcx, styles, &text, underline, width, self.markdown.as_ref().map(|fonts| (fonts, &p.block, local)));
                    p.layout = Some(layout);
                    p.decor = decor;
                }
            }
        }
        self.place(fcx, lcx, styles, width, caret);
        if let Some(resolver) = &self.resolver {
            resolver.sweep();
        }
    }

    fn place(&mut self, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>, caret: Option<usize>) {
        let mut top = 0.0;
        let mut i = 0;
        while i < self.paragraphs.len() {
            if let Some((end, reveal)) = self.run_end(i) {
                let span = self.paragraphs[i].start..=self.paragraphs[end].end();
                let inside = caret.is_some_and(|c| span.contains(&c));
                if let Some(height) = (!(inside && reveal == embeds::Reveal::Source)).then(|| self.embed(i, end, fcx, lcx, styles, width)).flatten() {
                    if inside && reveal == embeds::Reveal::Above {
                        let p = &mut self.paragraphs[i];
                        p.fold = Fold::Above;
                        p.lift = height;
                        p.top = top;
                        p.height = height + p.text_height();
                        top += p.height;
                        i += 1;
                        continue;
                    }
                    for (k, p) in self.paragraphs[i..=end].iter_mut().enumerate() {
                        p.fold = if k == 0 { Fold::Head } else { Fold::Member };
                        p.lift = 0.0;
                        p.top = if k == 0 { top } else { top + height };
                        p.height = if k == 0 { height } else { 0.0 };
                    }
                    top += height;
                    i = end + 1;
                    continue;
                }
            }
            let p = &mut self.paragraphs[i];
            p.fold = Fold::None;
            p.lift = 0.0;
            p.top = top;
            p.height = p.text_height();
            top += p.height;
            i += 1;
        }
    }

    pub fn invalidate(&mut self, offset: usize) {
        let i = self.at_offset(offset);
        self.paragraphs[i].layout = None;
    }

    pub fn invalidate_all(&mut self) {
        for p in &mut self.paragraphs {
            p.layout = None;
        }
    }
}
