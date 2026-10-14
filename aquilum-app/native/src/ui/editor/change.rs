use std::ops::Range;

pub fn shift(at: usize, range: &Range<usize>, inserted: usize) -> usize {
    if at < range.start || at == range.start && !range.is_empty() {
        at
    } else if at < range.end {
        range.start + inserted.min(at - range.start)
    } else {
        at - range.len() + inserted
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub at: usize,
    pub removed: String,
    pub inserted: String,
}

impl Change {
    pub fn map(&self, at: usize) -> usize {
        shift(at, &(self.at..self.at + self.removed.len()), self.inserted.len())
    }
}

pub fn map(at: usize, changes: &[Change]) -> usize {
    changes.iter().fold(at, |at, change| change.map(at))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(at: usize, removed: &str, inserted: &str) -> Change {
        Change { at, removed: removed.into(), inserted: inserted.into() }
    }

    #[test]
    fn a_position_moves_with_the_text_around_it() {
        let insert = change(4, "", "xx");
        assert_eq!([3, 4, 5].map(|at| insert.map(at)), [3, 6, 7]);
        let delete = change(4, "abc", "");
        assert_eq!([4, 5, 7, 9].map(|at| delete.map(at)), [4, 4, 4, 6]);
        let replace = change(4, "11", "9");
        assert_eq!([4, 5, 6, 8].map(|at| replace.map(at)), [4, 5, 5, 7]);
    }

    #[test]
    fn changes_apply_one_after_another() {
        let changes = [change(8, "", "!"), change(2, "ab", "")];
        assert_eq!([0, 3, 9].map(|at| map(at, &changes)), [0, 2, 8]);
    }
}
