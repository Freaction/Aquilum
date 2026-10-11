use super::{Fold, TextEditor};

impl TextEditor {
    pub fn breaks(&self) -> Vec<f64> {
        let mut out = Vec::new();
        for p in self.doc.paragraphs().iter().filter(|p| p.height > 0.0) {
            out.push(p.top);
            if p.fold == Fold::None
                && let Some(layout) = &p.layout
            {
                let origin = p.origin().y;
                out.extend(layout.lines().map(|line| origin + f64::from(line.metrics().min_coord)));
            }
            out.push(p.top + p.height);
        }
        out.sort_by(f64::total_cmp);
        out.dedup();
        out
    }
}
