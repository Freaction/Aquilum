use std::ops::Range;

use masonry::core::BrushIndex;
use masonry::kurbo::Vec2;
use masonry::parley::Layout;

use super::super::markdown::{Block, Decor, Frame};
use super::laid::{Laid, PLAIN};
use super::{Document, Embed, Fold};

pub struct Paragraph<'a> {
    pub start: usize,
    pub len: u32,
    pub block: &'a Block,
    pub frame: &'a Frame,
    pub fold: Fold,
    pub embed: Option<&'a Embed>,
    lift: f64,
    scroll: f64,
    laid: Option<&'a Laid>,
}

impl<'a> Paragraph<'a> {
    pub fn end(&self) -> usize {
        self.start + self.len as usize
    }

    pub fn range(&self) -> Range<usize> {
        self.start..self.end()
    }

    pub fn laid(&self) -> Option<&'a Layout<BrushIndex>> {
        self.laid.map(|laid| &laid.layout)
    }

    pub fn layout(&self) -> &'a Layout<BrushIndex> {
        self.laid().expect("абзац сверстан")
    }

    pub fn decor(&self) -> &'a Decor {
        self.laid.map_or(&PLAIN, |laid| &laid.decor)
    }

    pub fn origin(&self, top: f64) -> Vec2 {
        let decor = self.decor();
        Vec2::new(decor.x - self.scroll, top + self.lift + decor.pad.0)
    }
}

impl Document {
    pub fn line_count(&self) -> usize {
        self.lines.count()
    }

    pub fn paragraph(&self, line: usize) -> Paragraph<'_> {
        let range = self.span(line);
        let (frame, block) = self.blocks.get(line);
        Paragraph {
            start: range.start,
            len: range.len() as u32,
            block,
            frame,
            fold: self.heights.fold(line),
            embed: self.embeds.get(line),
            lift: self.heights.lift(line),
            scroll: if matches!(block, Block::Code { fence: false, .. }) { self.scroll_of(range.start) } else { 0.0 },
            laid: self.laid.get(line),
        }
    }

    pub fn block(&self, line: usize) -> &Block {
        &self.blocks.get(line).1
    }

    pub fn frame(&self, line: usize) -> &Frame {
        &self.blocks.get(line).0
    }

    pub fn folded_end(&self, head: usize) -> usize {
        let last = (head + 1..self.lines.count()).take_while(|&i| self.heights.fold(i) == Fold::Member).last();
        self.base + self.lines.end(last.unwrap_or(head))
    }

    #[cfg(test)]
    pub fn paragraphs(&self) -> impl Iterator<Item = Paragraph<'_>> {
        (0..self.lines.count()).map(|line| self.paragraph(line))
    }
}
