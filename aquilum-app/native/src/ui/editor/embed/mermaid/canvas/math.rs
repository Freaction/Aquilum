use masonry::core::BrushIndex;
use masonry::kurbo::{Affine, Point, Rect, Vec2};
use masonry::parley::Layout;

use super::super::super::drawing::Item;
use super::super::super::math::{formula, placed};
use super::super::super::{Font, Spec};
use super::text::{aligned, pieces, tall, width};
use super::Label;
use crate::ui::tokens::size;

pub(super) fn mixed(label: &Label, at: Affine, scale: f64, make: &mut dyn FnMut(&Spec) -> Layout<BrushIndex>, items: &mut Vec<Item>) {
    let size = label.size * scale;
    let px = size.round().clamp(8.0, 40.0) as u8;
    let anchor = at * label.at;
    let lines: Vec<&str> = label.text.lines().collect();
    let heights: Vec<f64> = lines.iter().map(|l| tall(l, size)).collect();
    let widths: Vec<f64> = lines.iter().map(|l| width(l, size)).collect();
    let total: f64 = heights.iter().sum();
    if label.chip {
        let wide = widths.iter().copied().fold(0.0, f64::max);
        super::plate(Rect::from_center_size(anchor, (wide + size::GAP_MD * 2.0 * scale, total + size::GAP_XS * 2.0 * scale)), items);
    }
    let mut y = anchor.y - total / 2.0;
    for ((line, h), w) in lines.iter().zip(heights).zip(widths) {
        let (mut x, mid) = (aligned(label.align, anchor.x, w), y + h / 2.0);
        for (math, piece) in pieces(line) {
            if let Some(f) = math.then(|| formula(piece, size, true)).flatten() {
                let high = f64::from(f.ascent + f.depth);
                items.extend(placed(piece, size, Vec2::new(x, mid - high / 2.0)).unwrap_or_default());
                x += f64::from(f.width);
            } else if !piece.is_empty() {
                let layout = make(&Spec { bold: label.bold, ..Spec::plain(piece, Font::Ui(px), label.brush.clone()) });
                let (w, h) = (f64::from(layout.width()), f64::from(layout.height()));
                items.push(Item::Text(Point::new(x, mid - h / 2.0), layout, piece.to_owned()));
                x += w;
            }
        }
        y += h;
    }
}
