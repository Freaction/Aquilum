use masonry::core::BrushIndex;
use masonry::kurbo::{Point as Pos, Rect};
use masonry::parley::{Affinity, Cursor, Layout, Selection};

use crate::reader::flow::Block;
use crate::reader::hyphen;
use crate::reader::page::Item;
use super::{Session, Spot};

const SEAM: f64 = 2.0;

fn words(text: &str, breaks: &[(usize, char)], a: usize, b: usize) -> (usize, usize) {
    let (from, to) = (hyphen::to_text(breaks, a), hyphen::to_text(breaks, b));
    let span = text.get(from..to).unwrap_or_default();
    let lead = span.len() - span.trim_start().len();
    (hyphen::to_layout(breaks, from + lead), hyphen::to_layout(breaks, from + lead + span.trim().len()))
}

fn line_spans(layout: &Layout<BrushIndex>, breaks: &[(usize, char)], text: &str, start: usize, end: usize) -> Vec<(f64, f64, f64, f64)> {
    let (lo, hi) = (hyphen::to_layout(breaks, start), hyphen::to_layout(breaks, end));
    let edge = |index, affinity| Cursor::from_byte_index(layout, index, affinity).geometry(layout, 0.0).x0;
    layout
        .lines()
        .filter_map(|line| {
            let range = line.text_range();
            let (a, b) = words(text, breaks, lo.max(range.start), hi.min(range.end));
            let metrics = line.metrics();
            (a < b).then(|| (edge(a, Affinity::Downstream), f64::from(metrics.min_coord), edge(b, Affinity::Upstream), f64::from(metrics.max_coord)))
        })
        .collect()
}

fn close_seams(rects: &mut [Rect]) {
    for i in 1..rects.len() {
        if (rects[i].y0 - rects[i - 1].y1).abs() <= SEAM {
            rects[i - 1].y1 = rects[i].y0;
        }
    }
}

impl Session {
    pub fn hit(&self, p: Pos, word: bool) -> Option<(Spot, Spot)> {
        let laid = self.laid.as_ref()?;
        let visible = self.visible();
        let (rect, y0, column) = visible.iter().min_by(|a, b| {
            let d = |r: &Rect| if p.x < r.x0 { r.x0 - p.x } else if p.x > r.x1 { p.x - r.x1 } else { 0.0 };
            d(&a.0).total_cmp(&d(&b.0))
        })?;
        let piece = column
            .pieces
            .iter()
            .find(|piece| p.y < y0 + piece.y + (piece.to - piece.from))
            .or_else(|| column.pieces.last())?;
        let spot = |byte| Spot { block: piece.item, byte };
        match &laid.items[piece.item] {
            Item::Text { layout, x, breaks, .. } => {
                let local_x = (p.x - rect.x0 - x) as f32;
                let local_y = ((p.y - y0 - piece.y).max(0.0) + piece.from).min(piece.to - 0.5) as f32;
                if word {
                    let range = Selection::word_from_point(layout, local_x, local_y).text_range();
                    return Some((spot(hyphen::to_text(breaks, range.start)), spot(hyphen::to_text(breaks, range.end))));
                }
                let at = hyphen::to_text(breaks, Selection::from_point(layout, local_x, local_y).focus().index());
                Some((spot(at), spot(at)))
            }
            _ => Some((spot(0), spot(0))),
        }
    }

    fn block_range(&self, block: usize, from: Spot, to: Spot) -> Option<(usize, usize)> {
        let Block::Text { text, .. } = self.blocks.get(block)? else { return None };
        if !(from.block..=to.block).contains(&block) {
            return None;
        }
        let start = if block == from.block { from.byte } else { 0 };
        let end = if block == to.block { to.byte } else { text.len() };
        (start < end).then_some((start, end))
    }

    pub fn rects(&self, from: Spot, to: Spot) -> Vec<Rect> {
        let Some(laid) = &self.laid else { return Vec::new() };
        let mut out = Vec::new();
        for (rect, y0, column) in self.visible() {
            let first = out.len();
            for piece in &column.pieces {
                let Some((start, end)) = self.block_range(piece.item, from, to) else { continue };
                let (Item::Text { layout, x, breaks, .. }, Block::Text { text, .. }) = (&laid.items[piece.item], &self.blocks[piece.item]) else { continue };
                let dy = y0 + piece.y - piece.from;
                for (left, top, right, bottom) in line_spans(layout, breaks, text, start, end) {
                    if (piece.from..piece.to).contains(&((top + bottom) / 2.0)) {
                        out.push(Rect::new(rect.x0 + x + left, top + dy, rect.x0 + x + right, bottom + dy));
                    }
                }
            }
            close_seams(&mut out[first..]);
        }
        out
    }

    pub fn text(&self, from: Spot, to: Spot) -> String {
        let mut parts = Vec::new();
        for block in from.block..=to.block.min(self.blocks.len().saturating_sub(1)) {
            if let (Some((start, end)), Block::Text { text, .. }) = (self.block_range(block, from, to), &self.blocks[block]) {
                parts.push(&text[start..end]);
            }
        }
        parts.join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
    }

    pub fn range_cfi(&self, from: Spot, to: Spot) -> Option<String> {
        let start = self.point(from.block, from.byte, true)?;
        let end = self.point(to.block, to.byte, false).unwrap_or(start);
        self.book.cfi(self.section, start, end)
    }

    pub fn resolve(&self, cfi: &str) -> Option<(Spot, Spot)> {
        let (index, start, end) = self.book.resolve(cfi)?;
        if index != self.section {
            return None;
        }
        let (sb, sy) = self.locate(start)?;
        let (eb, ey) = self.locate(end)?;
        Some((Spot { block: sb, byte: sy }, Spot { block: eb, byte: ey }))
    }
}
