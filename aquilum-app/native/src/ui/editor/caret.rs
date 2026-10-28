use std::ops::Range;

use masonry::core::BrushIndex;
use masonry::kurbo::Point;
use masonry::parley::{Affinity, Cursor, Layout};
use unicode_segmentation::GraphemeCursor;

fn on_boundary(text: &str, index: usize) -> bool {
    GraphemeCursor::new(index, text.len(), true).is_boundary(text, 0).unwrap_or(true)
}

fn nearest_boundary(text: &str, index: usize) -> usize {
    let index = index.min(text.len());
    if on_boundary(text, index) {
        return index;
    }
    let before = GraphemeCursor::new(index, text.len(), true).prev_boundary(text, 0).ok().flatten().unwrap_or(0);
    let after = GraphemeCursor::new(index, text.len(), true).next_boundary(text, 0).ok().flatten().unwrap_or(text.len());
    if index - before <= after - index { before } else { after }
}

pub fn hit(layout: &Layout<BrushIndex>, text: &str, at: Point) -> (usize, Affinity) {
    let cursor = Cursor::from_point(layout, at.x as f32, at.y as f32);
    (nearest_boundary(text, cursor.index()), cursor.affinity())
}

pub fn step(layout: &Layout<BrushIndex>, text: &str, from: Cursor, forward: bool, word: bool) -> Cursor {
    let advance = |cursor: Cursor| match (forward, word) {
        (true, false) => cursor.next_visual(layout),
        (false, false) => cursor.previous_visual(layout),
        (true, true) => cursor.next_visual_word(layout),
        (false, true) => cursor.previous_visual_word(layout),
    };
    let mut moved = advance(from);
    while !on_boundary(text, moved.index()) {
        let next = advance(moved);
        if next.index() == moved.index() {
            break;
        }
        moved = next;
    }
    moved
}

pub fn around(selection: Range<usize>, before: usize, after: usize, len: usize, floor: impl Fn(usize) -> usize) -> [Range<usize>; 2] {
    [floor(selection.start.saturating_sub(before))..selection.start, selection.end..floor((selection.end + after).min(len))]
}
