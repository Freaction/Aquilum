use std::ops::Range;

pub fn bounds(text: &str, at: usize) -> Range<usize> {
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    start..end
}

pub fn heading(line: &str) -> Option<(u8, usize)> {
    let level = line.bytes().take_while(|b| *b == b'#').count();
    match line.as_bytes().get(level) {
        _ if level == 0 || level > 6 => None,
        None => Some((level as u8, level)),
        Some(b' ') => Some((level as u8, level + 1)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::bounds;

    #[test]
    fn a_line_is_found_from_any_offset_inside_it_or_at_its_edges() {
        let text = "один\nдва\n\nчетыре";
        assert_eq!(bounds(text, 0), 0..8);
        assert_eq!(bounds(text, 8), 0..8);
        assert_eq!(bounds(text, 9), 9..15);
        assert_eq!(bounds(text, 15), 9..15);
        assert_eq!(bounds(text, 16), 16..16);
        assert_eq!(bounds(text, 17), 17..text.len());
        assert_eq!(bounds(text, text.len()), 17..text.len());
        assert_eq!(bounds("", 0), 0..0);
    }
}
