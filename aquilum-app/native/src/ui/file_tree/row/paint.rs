use masonry::core::{PaintCtx, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Point, Rect, RoundedRect};

use super::TreeRow;
use crate::ui::tokens::size;
use crate::ui::{icons, text, theme};

impl TreeRow {
    pub(super) fn draw(&mut self, ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let hovered = ctx.is_hovered();

        let background = if self.drop_over {
            Some(t.bg_accent.multiply_alpha(0.05))
        } else if self.row.selected {
            Some(t.bg_accent.multiply_alpha(0.1))
        } else if hovered {
            Some(t.sidebar_item_hover_bg)
        } else if self.row.active {
            Some(t.sidebar_item_active_bg)
        } else {
            None
        };
        if let Some(color) = background {
            painter.fill(RoundedRect::from_rect(bounds, size::SIDEBAR_ITEM_RADIUS), color).draw();
        }
        if self.drop_over {
            let ring = RoundedRect::from_rect(bounds.inflate(-0.6, -0.6), size::SIDEBAR_ITEM_RADIUS - 0.6);
            painter.stroke(&ring, &masonry::kurbo::Stroke::new(1.2), t.border_accent).draw();
        }

        for &guide in &self.row.guides {
            let x = bounds.x0
                + size::SIDEBAR_ITEM_PADDING
                + guide as f64 * size::SIDEBAR_TREE_INDENT
                + size::SIDEBAR_ICON_SIZE / 2.0;
            painter.fill(Rect::new(x, bounds.y0, x + 1.0, bounds.y1), t.sidebar_tree_guide).draw();
        }

        let icon_color = if self.row.selected {
            t.icon_primary
        } else if hovered || self.row.active {
            t.sidebar_icon_hover
        } else {
            t.sidebar_icon
        };
        if self.row.folder {
            let icon = if self.row.expanded { icons::CHEVRON_DOWN } else { icons::CHEVRON_RIGHT };
            let glyph = 16.0;
            let inset = (size::SIDEBAR_ICON_SIZE - glyph) / 2.0;
            let origin = Point::new(bounds.x0 + self.icon_x() + inset, bounds.y0 + size::SIDEBAR_ITEM_PADDING + inset);
            icon.draw(painter, origin, glyph, 1.6, icon_color);
        }

        let text_x = self.text_x();
        let available = (bounds.width() - text_x - size::SIDEBAR_ITEM_PADDING).max(0.0);
        let text_color = if self.row.selected { t.text_primary } else { t.sidebar_text };
        let layout = self.name.layout(ctx, &self.row.name, available, text::Style::ui(size::SIDEBAR_ICON_SIZE));
        let origin = Affine::translate((bounds.x0 + text_x, bounds.y0 + size::SIDEBAR_ITEM_PADDING));
        render_text(painter, origin, layout, &[text_color.into()], true);
    }
}
