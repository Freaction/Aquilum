use aquilum_core::text::content::{splice, Splice};
use crop::{Rope, RopeSlice};

pub struct Text<'a>(pub &'a Rope);

impl aquilum_core::text::content::Content for Text<'_> {
    fn byte_len(&self) -> usize {
        self.0.byte_len()
    }

    fn read(&self, range: std::ops::Range<usize>, visit: &mut dyn FnMut(&[u8]) -> std::io::Result<()>) -> std::io::Result<()> {
        let text = self.0;
        let mut high = range.end;
        while high < text.byte_len() && !text.is_char_boundary(high) {
            high += 1;
        }
        let mut at = floor(text, range.start);
        for chunk in text.byte_slice(at..high).chunks() {
            let (from, to) = (range.start.max(at) - at, range.end.min(at + chunk.len()).saturating_sub(at));
            if from < to {
                visit(&chunk.as_bytes()[from..to])?;
            }
            at += chunk.len();
        }
        Ok(())
    }
}

pub fn floor(text: &Rope, at: usize) -> usize {
    let mut at = at.min(text.byte_len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

pub fn each_line(text: RopeSlice<'_>, mut line: impl FnMut(&str) -> bool) {
    each_block(text, |block| block.split('\n').all(&mut line));
}

pub fn each_block(text: RopeSlice<'_>, mut block: impl FnMut(&str) -> bool) {
    let mut part = String::new();
    for chunk in text.chunks() {
        let Some(last) = chunk.rfind('\n') else {
            part.push_str(chunk);
            continue;
        };
        let mut whole = &chunk[..last];
        if !part.is_empty() {
            let first = whole.find('\n').unwrap_or(whole.len());
            part.push_str(&whole[..first]);
            if !block(&part) {
                return;
            }
            part.clear();
            whole = whole.get(first + 1..).unwrap_or_default();
            if first == last {
                part.push_str(&chunk[last + 1..]);
                continue;
            }
        }
        if !block(whole) {
            return;
        }
        part.push_str(&chunk[last + 1..]);
    }
    block(&part);
}

pub fn changed(old: &Rope, new: &Rope) -> Splice {
    splice(&Text(old), &Text(new)).unwrap_or(Splice { at: 0, removed: old.byte_len(), inserted: new.byte_len() })
}

#[cfg(test)]
mod tests {
    use super::{changed, each_block, each_line, Text};
    use aquilum_core::text::content::{read_bytes, Splice};
    use crop::Rope;

    #[test]
    fn text_reads_any_byte_range() {
        let source: String = (0..3000).map(|i| format!("правка {i}\n")).collect();
        let rope = Rope::from(source.as_str());
        for (start, end) in [(1, 2), (9, 4000), (5, source.len()), (source.len() - 3, source.len())] {
            assert_eq!(read_bytes(&Text(&rope), start..end).unwrap(), &source.as_bytes()[start..end]);
        }
    }

    #[test]
    fn the_changed_part_is_found_across_chunks() {
        let text: String = (0..5000).map(|i| format!("строка {i}\n")).collect();
        let (a, mut b) = (Rope::from(text.as_str()), Rope::from(text.as_str()));
        assert_eq!(changed(&a, &b), Splice { at: text.len(), removed: 0, inserted: 0 });
        b.insert(30_000, "x");
        assert_eq!(changed(&a, &b), Splice { at: 30_000, removed: 0, inserted: 1 });
    }

    #[test]
    fn each_line_matches_split() {
        let text: String = (0..3000).map(|i| format!("строка {i}\n")).collect::<String>() + "хвост";
        let mut lines = Vec::new();
        each_line(Rope::from(text.as_str()).byte_slice(..), |line| {
            lines.push(line.to_owned());
            true
        });
        assert_eq!(lines, text.split('\n').collect::<Vec<_>>());
    }

    #[test]
    fn blocks_are_whole_lines_that_join_back_into_the_text() {
        for tail in ["хвост", "", "\n"] {
            let text: String = (0..3000).map(|i| format!("строка {i}\n{}", "\n".repeat(i % 3))).collect::<String>() + tail;
            let mut blocks = Vec::new();
            each_block(Rope::from(text.as_str()).byte_slice(..), |block| {
                blocks.push(block.to_owned());
                true
            });
            assert!(blocks.len() > 1, "текст идёт кусками, а не одной строкой");
            assert_eq!(blocks.join("\n"), text);
        }
    }
}
