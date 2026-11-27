use masonry::kurbo::{Point, Rect, RoundedRect};

use super::super::super::markdown::TEXT;
use super::super::tone::Tone;
use super::canvas::{BOLD, Canvas, FONT, LINE, Label, width};
use super::hue;
use crate::ui::tokens::size;

pub(super) const GAP: f64 = 16.0;

pub(super) fn columns(title: Option<&str>, sections: &[&str], of: &[Option<usize>], mut widths: Vec<f64>, canvas: &mut Canvas) -> (Vec<f64>, Vec<f64>, f64) {
    let groups: Vec<Vec<usize>> = (0..sections.len()).map(|s| (0..of.len()).filter(|&k| of[k] == Some(s)).collect()).collect();
    for (cols, name) in groups.iter().zip(sections) {
        let have = cols.iter().map(|&k| widths[k]).sum::<f64>() + GAP * cols.len().saturating_sub(1) as f64;
        let need = width(name, FONT) * BOLD + size::GAP_XL * 2.0;
        cols.iter().for_each(|&k| widths[k] += (need - have).max(0.0) / cols.len() as f64);
    }
    let lefts: Vec<f64> = widths.iter().scan(0.0, |x, w| Some(std::mem::replace(x, *x + w + GAP))).collect();
    let mut y = 0.0;
    if let Some(title) = title {
        canvas.label(Label::new(Point::new(0.0, LINE / 2.0), title, FONT + 2.0, TEXT).bold().start(0.0));
        y += LINE + GAP;
    }
    if !sections.is_empty() {
        let band = LINE + size::GAP_MD * 2.0;
        for (s, (cols, name)) in groups.iter().zip(sections).enumerate() {
            let (Some(&a), Some(&b)) = (cols.first(), cols.last()) else { continue };
            let r = Rect::new(lefts[a], y, lefts[b] + widths[b], y + band);
            canvas.shape(RoundedRect::from_rect(r, size::RADIUS_LG), Some(Tone::Panel(hue(s))), None);
            canvas.label(Label::new(r.center(), *name, FONT, TEXT).bold());
        }
        y += band + size::GAP_MD;
    }
    (lefts, widths, y)
}
