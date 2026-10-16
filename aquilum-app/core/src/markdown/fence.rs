use super::indent::Indent;

pub const INDENT_LIMIT: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Fence {
    pub symbol: u8,
    pub length: usize,
    info: usize,
}

impl Fence {
    pub fn open(line: &str, max_columns: usize) -> Option<Fence> {
        let indent = Indent::of(line);
        if indent.columns > max_columns {
            return None;
        }
        let rest = &line[indent.bytes..];
        let symbol = *rest.as_bytes().first().filter(|b| matches!(b, b'`' | b'~'))?;
        let length = rest.bytes().take_while(|b| *b == symbol).count();
        if length < 3 || symbol == b'`' && rest[length..].contains('`') {
            return None;
        }
        Some(Fence { symbol, length, info: indent.bytes + length })
    }

    pub fn info<'a>(&self, line: &'a str) -> &'a str {
        line[self.info..].trim()
    }

    pub fn closes(&self, line: &str) -> bool {
        let rest = line.trim_start_matches([' ', '\t']);
        let run = rest.bytes().take_while(|b| *b == self.symbol).count();
        run >= self.length && rest[run..].trim().is_empty()
    }
}

pub fn outside(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut inside: Option<Fence> = None;
    text.lines().enumerate().filter(move |(_, line)| match inside {
        Some(fence) => {
            if fence.closes(line) {
                inside = None;
            }
            false
        }
        None => {
            inside = Fence::open(line, usize::MAX);
            inside.is_none()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fence_hides_its_lines_until_the_same_marker_closes_it() {
        let text = "до\n````md\n```\nвнутри\n````\nпосле\n~~~\nтильды\n~~~";
        assert_eq!(outside(text).collect::<Vec<_>>(), vec![(0, "до"), (5, "после")]);
    }

    #[test]
    fn the_opening_line_decides_the_info_string_and_the_limit() {
        let fence = Fence::open("   ```rust title=x", INDENT_LIMIT).unwrap();
        assert_eq!((fence.symbol, fence.length, fence.info("   ```rust title=x")), (b'`', 3, "rust title=x"));
        assert!(Fence::open("    ```", INDENT_LIMIT).is_none());
        assert!(Fence::open("\t```", INDENT_LIMIT).is_none());
        assert!(Fence::open("\t```", usize::MAX).is_some());
        assert!(Fence::open("```a`b", INDENT_LIMIT).is_none());
        assert!(Fence::open("~~~a`b", INDENT_LIMIT).is_some());
        assert!(Fence::open("``", INDENT_LIMIT).is_none());
    }

    #[test]
    fn only_a_bare_run_of_the_same_symbol_closes() {
        let fence = Fence::open("````", INDENT_LIMIT).unwrap();
        assert!(fence.closes("````"));
        assert!(fence.closes("  ``````  "));
        assert!(!fence.closes("```"));
        assert!(!fence.closes("````js"));
        assert!(!fence.closes("~~~~"));
    }
}
