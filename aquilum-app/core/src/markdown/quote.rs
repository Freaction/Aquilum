use std::ops::Range;

use super::fence::INDENT_LIMIT;
use super::indent::Indent;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Callout {
    pub range: Range<usize>,
    pub name: String,
    pub fold: Option<char>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quote {
    pub marker: usize,
    pub depth: usize,
    pub callout: Option<Callout>,
}

fn callout(line: &str, marker: usize) -> Option<Callout> {
    let rest = line[marker..].strip_prefix("[!")?;
    let close = rest.find(']')?;
    let name = &rest[..close];
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-')) {
        return None;
    }
    let mut end = marker + 2 + close + 1;
    let fold = line[end..].chars().next().filter(|c| matches!(c, '+' | '-'));
    end += fold.map_or(0, char::len_utf8);
    end += usize::from(line[end..].starts_with(' '));
    Some(Callout { range: marker..end, name: name.to_lowercase(), fold })
}

pub fn quote(line: &str) -> Option<Quote> {
    let indent = Indent::of(line);
    if indent.columns > INDENT_LIMIT {
        return None;
    }
    let mut marker = indent.bytes;
    let mut depth = 0;
    while line[marker..].starts_with('>') {
        marker += 1 + usize::from(line[marker + 1..].starts_with(' '));
        depth += 1;
    }
    (depth > 0).then(|| Quote { marker, depth, callout: callout(line, marker) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_markers_are_one_prefix() {
        let found = quote("> > > Третий").unwrap();
        assert_eq!((found.marker, found.depth), (6, 3));
        assert_eq!(quote(">>x").map(|q| (q.marker, q.depth)), Some((2, 2)));
        assert_eq!(quote("   > a").map(|q| q.marker), Some(5));
        assert!(quote("    > a").is_none());
        assert!(quote("\t> a").is_none());
        assert!(quote("текст > не цитата").is_none());
        assert_eq!(quote(">").map(|q| (q.marker, q.depth)), Some((1, 1)));
    }

    #[test]
    fn a_callout_header_follows_the_prefix_directly() {
        let found = quote("> [!Quote] текст").unwrap();
        let callout = found.callout.unwrap();
        assert_eq!((callout.range, callout.name.as_str(), callout.fold), (2..11, "quote", None));
        let folded = quote("> [!note]- Заголовок").unwrap().callout.unwrap();
        assert_eq!((folded.range, folded.fold), (2..11, Some('-')));
        assert!(quote(">  [!quote] два пробела").unwrap().callout.is_none());
        assert!(quote("> [!] пусто").unwrap().callout.is_none());
        assert!(quote("> [! x] пробел").unwrap().callout.is_none());
        assert_eq!(quote("> > [!book] [[a]]").unwrap().callout.unwrap().name, "book");
    }
}
