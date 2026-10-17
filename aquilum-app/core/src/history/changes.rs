use crate::text::content::{read_string as text, Content};
use std::io;
use std::ops::Range;

const CONTEXT: usize = 400;
const SHOWN: usize = 64 * 1024;
const MAX_HUNKS: usize = 200;

#[derive(Debug, PartialEq, Eq)]
pub struct Hunk {
    pub before: String,
    pub removed: String,
    pub added: String,
    pub after: String,
}

pub fn hunks(previous: &dyn Content, current: &dyn Content, regions: &[(Range<usize>, Range<usize>)]) -> io::Result<Vec<Hunk>> {
    regions
        .iter()
        .take(MAX_HUNKS)
        .map(|(old, new)| {
            let before = text(previous, old.start.saturating_sub(CONTEXT)..old.start)?;
            let after = text(previous, old.end..(old.end + CONTEXT).min(previous.byte_len()))?;
            Ok(Hunk {
                before: before.split_once('\n').filter(|_| old.start > CONTEXT).map_or(before.as_str(), |(_, rest)| rest).to_owned(),
                removed: text(previous, old.start..old.end.min(old.start + SHOWN))?,
                added: text(current, new.start..new.end.min(new.start + SHOWN))?,
                after: after.rsplit_once('\n').filter(|_| old.end + CONTEXT < previous.byte_len()).map_or(after.as_str(), |(rest, _)| rest).to_owned(),
            })
        })
        .collect()
}


#[cfg(test)]
mod tests {
    use super::{hunks, Hunk};

    #[test]
    fn a_hunk_shows_what_changed_with_lines_around() {
        let previous = "раз\nдва\nтри\nчетыре\n";
        let current = "раз\nдва\nТРИ\nчетыре\n";
        let at = previous.find("три").unwrap();
        let found = hunks(&previous, &current, &[(at..at + "три".len(), at..at + "ТРИ".len())]).unwrap();
        assert_eq!(found, [Hunk { before: "раз\nдва\n".to_owned(), removed: "три".to_owned(), added: "ТРИ".to_owned(), after: "\nчетыре\n".to_owned() }]);
    }
}
