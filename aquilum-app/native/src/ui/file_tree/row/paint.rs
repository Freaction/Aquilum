use masonry::core::{PaintCtx, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Point, Rect, RoundedRect};

use crate::ui::file_tree::model::Row;
use crate::ui::tokens::size;
use crate::ui::{icons, palette, text, theme};

#[derive(Default)]
pub struct Texts {
    name: text::Line,
    count: text::Line,
}

impl Texts {
    pub fn clear(&mut self) {
        self.name.clear();
        self.count.clear();
    }
}

#[derive(Clone, Copy, Default)]
pub struct Look {
    pub hovered: bool,
    pub drop_over: bool,
}

pub fn icon_x(depth: usize) -> f64 {
    size::SIDEBAR_ITEM_PADDING + depth as f64 * size::SIDEBAR_TREE_INDENT
}

pub fn text_x(depth: usize) -> f64 {
    icon_x(depth) + size::SIDEBAR_ICON_SIZE + size::SIDEBAR_ITEM_GAP
}

pub fn paint(ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_>, bounds: Rect, row: &Row, look: Look, texts: &mut Texts) {
    let t = theme::current();
    let Look { hovered, drop_over } = look;

    let background = if drop_over {
        Some(t.bg_accent.multiply_alpha(0.05))
    } else if row.selected {
        Some(t.bg_accent.multiply_alpha(0.1))
    } else if hovered {
        Some(t.sidebar_item_hover_bg)
    } else if row.active {
        Some(t.sidebar_item_active_bg)
    } else {
        None
    };
    if let Some(color) = background {
        painter.fill(RoundedRect::from_rect(bounds, size::SIDEBAR_ITEM_RADIUS), color).draw();
    }
    if drop_over {
        let ring = RoundedRect::from_rect(bounds.inflate(-0.6, -0.6), size::SIDEBAR_ITEM_RADIUS - 0.6);
        painter.stroke(&ring, &masonry::kurbo::Stroke::new(1.2), t.border_accent).draw();
    }

    for &guide in &row.guides {
        let x = bounds.x0 + size::SIDEBAR_ITEM_PADDING + guide as f64 * size::SIDEBAR_TREE_INDENT + size::SIDEBAR_ICON_SIZE / 2.0;
        painter.fill(Rect::new(x, bounds.y0, x + 1.0, bounds.y1), t.sidebar_tree_guide).draw();
    }

    let tint = row.color.and_then(|name| palette::color(name, &t));
    let icon_color = if row.selected {
        t.icon_primary
    } else if hovered || row.active {
        t.sidebar_icon_hover
    } else {
        t.sidebar_icon
    };
    let icon_color = tint.unwrap_or(icon_color);
    if row.folder {
        let icon = if row.expanded { icons::CHEVRON_DOWN } else { icons::CHEVRON_RIGHT };
        let glyph = 16.0;
        let inset = (size::SIDEBAR_ICON_SIZE - glyph) / 2.0;
        let origin = Point::new(bounds.x0 + icon_x(row.depth) + inset, bounds.y0 + size::SIDEBAR_ITEM_PADDING + inset);
        icon.draw(painter, origin, glyph, 1.6, icon_color);
    }

    let style = text::Style::ui(size::SIDEBAR_ICON_SIZE);
    let top = bounds.y0 + size::SIDEBAR_ITEM_PADDING;
    let mut available = (bounds.width() - text_x(row.depth) - size::SIDEBAR_ITEM_PADDING).max(0.0);
    if let Some(items) = row.items {
        let layout = texts.count.layout(ctx, &items.to_string(), available, style);
        let width = f64::from(layout.width());
        let origin = Affine::translate((bounds.x1 - size::SIDEBAR_ITEM_PADDING - width, top));
        render_text(painter, origin, layout, &[t.text_primary.multiply_alpha(0.4).into()], true);
        available = (available - width - size::SPACE_6).max(0.0);
    }
    let text_color = tint.unwrap_or(if row.selected { t.text_primary } else { t.sidebar_text });
    let layout = texts.name.layout(ctx, &row.name, available, style);
    let origin = Point::new(bounds.x0 + text_x(row.depth), top);
    text::paint_marks(painter, layout, &row.marks, origin);
    render_text(painter, Affine::translate(origin.to_vec2()), layout, &[text_color.into()], true);
}
