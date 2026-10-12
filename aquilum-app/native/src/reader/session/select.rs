use masonry::kurbo::{Point as Pos, Rect};
use masonry::parley::{Affinity, Cursor, Selection};

use crate::reader::flow::Block;
use crate::reader::hyphen;
use crate::reader::page::Item;
use super::{Session, Spot};

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
        let start = if block == from.block { from.byte } else { 0 };
        let end = if block == to.block { to.byte } else { text.len() };
        (start < end).then_some((start, end))
    }

    pub fn rects(&self, from: Spot, to: Spot) -> Vec<Rect> {
        let Some(laid) = &self.laid else { return Vec::new() };
        let mut out = Vec::new();
        for (rect, y0, column) in self.visible() {
            for piece in &column.pieces {
                if piece.item < from.block || piece.item > to.block {
                    continue;
                }
                let Some((start, end)) = self.block_range(piece.item, from, to) else { continue };
                let Item::Text { layout, x, breaks, .. } = &laid.items[piece.item] else { continue };
                let anchor = Cursor::from_byte_index(layout, hyphen::to_layout(breaks, start), Affinity::Downstream);
                let focus = Cursor::from_byte_index(layout, hyphen::to_layout(breaks, end), Affinity::Upstream);
                let mut lines: Vec<(usize, f64, f64)> = Vec::new();
                for (b, line) in Selection::new(anchor, focus).geometry(layout) {
                    match lines.iter_mut().find(|(l, ..)| *l == line) {
                        Some((_, x0, x1)) => (*x0, *x1) = (x0.min(b.x0), x1.max(b.x1)),
                        None => lines.push((line, b.x0, b.x1)),
                    }
                }
                let dy = y0 + piece.y - piece.from;
                for (line, x0, x1) in lines {
                    let Some(metrics) = layout.get(line).map(|l| *l.metrics()) else { continue };
                    let (top, bottom) = (f64::from(metrics.min_coord), f64::from(metrics.max_coord));
                    if bottom <= piece.from + 0.5 || top >= piece.to - 0.5 {
                        continue;
                    }
                    out.push(Rect::new(rect.x0 + x + x0, top + dy, rect.x0 + x + x1, bottom + dy));
                }
            }
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
