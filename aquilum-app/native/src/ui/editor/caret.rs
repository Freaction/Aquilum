use std::ops::Range;

use masonry::core::BrushIndex;
use masonry::kurbo::Point;
use masonry::parley::{Affinity, Cursor, Layout, Selection};
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

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

pub fn unit(layout: &Layout<BrushIndex>, text: &str, at: Point, count: u8) -> Range<usize> {
    match count {
        2 => Selection::word_from_point(layout, at.x as f32, at.y as f32).text_range(),
        3.. => 0..text.len(),
        _ => {
            let (index, _) = hit(layout, text, at);
            index..index
        }
    }
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

fn reach<'a>(parts: impl Iterator<Item = &'a str>) -> usize {
    let mut len = 0;
    for part in parts {
        len += part.len();
        if !part.chars().all(char::is_whitespace) {
            break;
        }
    }
    len
}

pub fn erase(text: &str, at: usize, forward: bool, word: bool) -> Range<usize> {
    let at = text.floor_char_boundary(at);
    let (before, after) = text.split_at(at);
    if forward {
        let len = if word { reach(after.split_word_bounds()) } else { reach(after.graphemes(true).take(1)) };
        at..at + len
    } else {
        let len = if word { reach(before.split_word_bounds().rev()) } else { reach(before.graphemes(true).rev().take(1)) };
        at - len..at
    }
}

pub fn around(selection: Range<usize>, before: usize, after: usize, len: usize, floor: impl Fn(usize) -> usize) -> [Range<usize>; 2] {
    [floor(selection.start.saturating_sub(before))..selection.start, selection.end..floor((selection.end + after).min(len))]
}

#[cfg(test)]
mod tests {
    use super::erase;

    #[test]
    fn one_character_goes_whole_in_either_direction() {
        let text = "a👨‍👩‍👧b";
        assert_eq!(erase(text, 1, true, false), 1..19);
        assert_eq!(erase(text, 19, false, false), 1..19);
        assert_eq!(erase(text, 0, false, false), 0..0);
        assert_eq!(erase(text, text.len(), true, false), text.len()..text.len());
    }

    #[test]
    fn a_word_takes_its_spaces_on_the_side_it_goes() {
        assert_eq!(erase("раз два три", 20, false, true), 14..20);
        assert_eq!(erase("раз два три ", 21, false, true), 14..21);
        assert_eq!(erase("раз два три", 0, true, true), 0..6);
        assert_eq!(erase("раз два три", 6, true, true), 6..13);
        assert_eq!(erase("раз, два", 8, false, true), 6..8);
        assert_eq!(erase("раз два", 0, false, true), 0..0);
    }

    #[test]
    fn an_offset_off_the_text_or_inside_a_letter_is_floored() {
        assert_eq!(erase("ab", 9, false, false), 1..2);
        assert_eq!(erase("яб", 1, true, false), 0..2);
    }
}
