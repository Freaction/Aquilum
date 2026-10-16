use super::{Fold, TextEditor};

impl TextEditor {
    pub fn breaks(&self) -> Vec<f64> {
        let mut out = Vec::new();
        let last = self.doc.line_count().saturating_sub(1);
        for (i, top, height) in self.doc.placed(0..=last).filter(|&(_, _, height)| height > 0.0) {
            let p = self.doc.paragraph(i);
            out.push(top);
            if p.fold == Fold::None
                && let Some(layout) = p.laid()
            {
                let origin = p.origin(top).y;
                out.extend(layout.lines().map(|line| origin + f64::from(line.metrics().block_min_coord)));
            }
            out.push(top + height);
        }
        out.sort_by(f64::total_cmp);
        out.dedup();
        out
    }
}
