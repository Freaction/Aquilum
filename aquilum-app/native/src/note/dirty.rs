use std::ops::Range;

use aquilum_core::text::content::Splice;

#[derive(Clone, Copy, Debug)]
pub struct Dirty {
    saved: usize,
    len: usize,
    prefix: usize,
    suffix: usize,
}

impl Dirty {
    pub fn clean(len: usize) -> Self {
        Self { saved: len, len, prefix: len, suffix: len }
    }

    pub fn edit(&mut self, range: Range<usize>, inserted: usize) {
        self.prefix = self.prefix.min(range.start);
        self.suffix = self.suffix.min(self.len.saturating_sub(range.end));
        self.len = self.len - range.len() + inserted;
    }

    pub fn splice(&self) -> Splice {
        let prefix = self.prefix.min(self.len).min(self.saved);
        let suffix = self.suffix.min(self.len - prefix).min(self.saved - prefix);
        Splice { at: prefix, removed: self.saved - prefix - suffix, inserted: self.len - prefix - suffix }
    }
}

#[cfg(test)]
mod tests {
    use super::Dirty;
    use aquilum_core::text::content::{splice, Splice};

    fn replace(text: &str, range: std::ops::Range<usize>, insert: &str) -> String {
        format!("{}{insert}{}", &text[..range.start], &text[range.end..])
    }

    #[test]
    fn the_dirty_part_covers_every_edit_since_the_save() {
        let saved = "0123456789abcdefghijklmnopqrstuvwxyz".to_owned();
        let mut dirty = Dirty::clean(saved.len());
        assert_eq!(dirty.splice(), Splice { at: saved.len(), removed: 0, inserted: 0 });
        let mut text = saved.clone();
        for (range, insert) in [(10..12, "ABCDE"), (30..30, "!"), (2..4, ""), (36..37, "конец")] {
            dirty.edit(range.clone(), insert.len());
            text = replace(&text, range, insert);
            let part = dirty.splice();
            let exact = splice(&saved.as_str(), &text.as_str()).unwrap();
            assert!(part.at <= exact.at && part.removed >= exact.removed, "{part:?} {exact:?}");
            let rebuilt = replace(&saved, part.at..part.at + part.removed, &text[part.at..part.at + part.inserted]);
            assert_eq!(rebuilt, text);
        }
    }
}
