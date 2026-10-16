use std::ops::Range;

use super::{Document, Fold};

impl Document {
    pub fn line_height(&self) -> f64 {
        if self.line > 0.0 { self.line } else { self.markdown.as_ref().map_or(20.0, |fonts| f64::from(fonts.size) * 1.6) }
    }

    fn columns(&self) -> Option<f64> {
        let ch = self.markdown.as_ref().map_or(0.0, |fonts| f64::from(fonts.ch));
        self.width.filter(|w| *w > 0.0 && ch > 0.0).map(|w| f64::from(w) / ch)
    }

    pub(super) fn estimate(&self, line: usize, height: f64) -> f64 {
        let chars = self.text[self.lines.range(line)].chars().count() as f64;
        self.columns().map_or(1.0, |columns| (chars / columns).ceil().max(1.0)) * height
    }

    pub(super) fn guess(&self, bytes: Range<usize>) -> f64 {
        if bytes.is_empty() {
            return 0.0;
        }
        let lines = (self.line_of(bytes.end) - self.line_of(bytes.start) + 1) as f64;
        let units = (self.full.utf16_code_unit_of_byte(bytes.end) - self.full.utf16_code_unit_of_byte(bytes.start)) as f64;
        let visual = self.columns().map_or(lines, |columns| lines + ((units - lines * columns * 0.5) / columns).max(0.0));
        visual * self.line_height()
    }

    pub(super) fn estimated(&mut self, lines: Range<usize>) -> f64 {
        let line = self.line_height();
        let mut sum = 0.0;
        for i in lines {
            let height = self.estimate(i, line);
            self.heights.set_text(i, height);
            self.heights.set(i, height, Fold::None);
            sum += height;
        }
        sum
    }
}
