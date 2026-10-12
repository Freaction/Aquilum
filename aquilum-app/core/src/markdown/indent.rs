pub const TAB: usize = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Indent {
    pub bytes: usize,
    pub columns: usize,
}

impl Indent {
    pub fn of(line: &str) -> Indent {
        let (mut bytes, mut columns) = (0, 0);
        for byte in line.bytes() {
            match byte {
                b' ' => columns += 1,
                b'\t' => columns += TAB - columns % TAB,
                _ => break,
            }
            bytes += 1;
        }
        Indent { bytes, columns }
    }
}

#[cfg(test)]
mod tests {
    use super::Indent;

    #[test]
    fn a_tab_advances_to_the_next_tab_stop() {
        assert_eq!(Indent::of("text"), Indent { bytes: 0, columns: 0 });
        assert_eq!(Indent::of("  text"), Indent { bytes: 2, columns: 2 });
        assert_eq!(Indent::of("\ttext"), Indent { bytes: 1, columns: 4 });
        assert_eq!(Indent::of("  \ttext"), Indent { bytes: 3, columns: 4 });
        assert_eq!(Indent::of("    \t- a"), Indent { bytes: 5, columns: 8 });
        assert_eq!(Indent::of("   "), Indent { bytes: 3, columns: 3 });
    }
}
