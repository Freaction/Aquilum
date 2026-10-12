use masonry::core::BrushIndex;
use masonry::kurbo::{Rect, Size};
use masonry::parley::{FontContext, LayoutContext};

use super::Settings;
use super::text::glyph_width;

const GAP: f64 = 0.07;
const MAX_BLOCK: f64 = 1440.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub columns: usize,
    pub column_width: f64,
    pub gap: f64,
    pub left: f64,
    pub top: f64,
    pub height: f64,
    pub size: f64,
}

pub fn advance(fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, settings: &Settings) -> f64 {
    let font = crate::ui::theme::editor_font(&settings.family);
    let width = f64::from(glyph_width(fcx, lcx, "0", font, settings.size, settings.weight));
    if width > 0.0 { width } else { f64::from(settings.size) * 0.5 }
}

pub fn geometry(view: Size, settings: &Settings, max_inline: f64) -> Geometry {
    if settings.scrolled {
        let width = max_inline.min(view.width - 2.0 * GAP / 2.0 * view.width).max(1.0);
        return Geometry { columns: 1, column_width: width, gap: 0.0, left: ((view.width - width) / 2.0).round(), top: 0.0, height: view.height, size: width };
    }
    let half = GAP / 2.0 * view.width;
    let portrait = view.height > view.width;
    let count = if portrait { 1.0 } else { 2.0 };
    let middle = (view.width - 4.0 * half).min(max_inline * count - GAP * view.width).max(1.0);
    let size = middle + 2.0 * half;
    let gap = GAP / (1.0 - GAP) * size;
    let columns = (count as usize).min((size / max_inline).ceil().max(1.0) as usize);
    let column_width = (size / columns as f64 - gap).max(1.0);
    let height = (view.height - 2.0 * settings.margin).clamp(1.0, MAX_BLOCK);
    Geometry { columns, column_width, gap, left: (view.width - size) / 2.0, top: ((view.height - height) / 2.0).round(), height, size }
}

impl Geometry {
    pub fn column_rect(&self, index: usize) -> Rect {
        let x = self.left + self.gap / 2.0 + index as f64 * (self.column_width + self.gap);
        Rect::new(x, self.top, x + self.column_width, self.top + self.height)
    }
}
