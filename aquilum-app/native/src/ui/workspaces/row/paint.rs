use masonry::core::{PaintCtx, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Point, RoundedRect, Stroke};

use super::{Kind, WorkspaceRow};
use crate::ui::tokens::size;
use crate::ui::widgets::paint_icon;
use crate::ui::{icons, text, theme};

impl WorkspaceRow {
    pub(super) fn draw(&mut self, ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let shape = RoundedRect::from_rect(bounds.inflate(-0.5, -0.5), size::WORKSPACE_ROW_RADIUS);
        if ctx.is_hovered() {
            painter.fill(shape, t.workspace_row_bg_hover).draw();
            painter.stroke(&shape, &Stroke::new(size::BORDER_1), t.workspace_row_border).draw();
        }
        let gutter = size::WORKSPACE_ROW_GUTTER;
        let icon = size::SIZE_20;
        let mark = match self.kind {
            Kind::Current => Some((icons::CHECK, t.icon_primary)),
            Kind::Cloud => Some((icons::CLOUD, t.icon_secondary)),
            Kind::Known => None,
        };
        if let Some((mark, color)) = mark {
            mark.draw(painter, Point::new(bounds.x0 + gutter, bounds.center().y - icon / 2.0), icon, 1.2, color);
        }
        let (text_x, width, top) = self.text_span(bounds);
        let style = text::Style::ui(size::SIZE_20);
        if self.field.is_none() {
            let name = self.name_line.layout(ctx, &self.name, width, style);
            render_text(painter, Affine::translate((text_x, top)), name, &[t.text_secondary.into()], true);
        }
        let detail = self.detail_line.layout(ctx, &self.detail, width, style);
        render_text(painter, Affine::translate((text_x, top + size::SIZE_20)), detail, &[t.text_tertiary.into()], true);
        let alpha = if self.always { 1.0 } else { self.more.value() as f32 };
        if alpha > 0.0 && self.has_menu() {
            paint_icon(painter, Self::more_rect(bounds), icons::MORE_HORIZONTAL, self.more_hovered, alpha, None);
        }
    }
}
