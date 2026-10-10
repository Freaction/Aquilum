use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx,
    Widget, WidgetMut, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Stroke};
use masonry::layout::{LenReq, Length};

use super::menu::box_shadow;
use super::text::{Line, Style};
use super::theme;
use super::tokens::{number, size};
use crate::i18n::t;

pub struct UpdateSplash {
    installing: bool,
    percent: Option<f64>,
    title: Line,
    detail: Line,
}

impl UpdateSplash {
    pub fn new(installing: bool, percent: Option<f64>) -> NewWidget<Self> {
        NewWidget::new(UpdateSplash { installing, percent, title: Line::default(), detail: Line::default() })
    }

    pub fn set(this: &mut WidgetMut<'_, Self>, installing: bool, percent: Option<f64>) {
        if (this.widget.installing, this.widget.percent) != (installing, percent) {
            this.widget.installing = installing;
            this.widget.percent = percent;
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }

    fn texts(&self) -> (String, String) {
        let title = if self.installing { t("update.installing") } else { t("update.downloading") };
        let detail = match (self.installing, self.percent) {
            (true, _) => t("update.willOpen"),
            (false, Some(p)) => format!("{}%", p.round()),
            (false, None) => t("update.preparing"),
        };
        (title, detail)
    }
}

impl Widget for UpdateSplash {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let window = ctx.content_box();
        painter.fill(window, tm.dialog_overlay_bg).draw();
        let (title, detail) = self.texts();
        let font = theme::ui_font_settings();
        let line = (f64::from(font.size) * number::FONT_LINE_HEIGHT_NORMAL).round();
        let medium = Style { size: font.size, weight: number::FONT_WEIGHT_UI_MEDIUM as f32, line_height: line };
        let pad = size::SPACE_24;
        let gap = size::SPACE_12;
        let width = size::SIZE_256.max(240.0) + pad * 2.0;
        let track = self.percent.filter(|_| !self.installing).or(self.installing.then_some(100.0));
        let height = pad * 2.0 + line * 2.0 + gap + track.map_or(0.0, |_| gap + size::SIZE_4);
        let card = Rect::from_origin_size(Point::new(((window.width() - width) / 2.0).round(), ((window.height() - height) / 2.0).round()), Size::new(width, height));
        let shape = RoundedRect::from_rect(card, size::DIALOG_RADIUS);
        box_shadow(painter, shape, tm.dialog_shadow);
        painter.fill(shape, tm.dialog_bg).draw();
        painter.stroke(&RoundedRect::from_rect(card.inflate(-0.5, -0.5), size::DIALOG_RADIUS - 0.5), &Stroke::new(size::BORDER_1), tm.dialog_border).draw();
        let inner = card.width() - pad * 2.0;
        let mut y = card.y0 + pad;
        let layout = self.title.layout(ctx, &title, inner, medium);
        render_text(painter, Affine::translate((card.x0 + (card.width() - f64::from(layout.width())) / 2.0, y)), layout, &[tm.text_primary.into()], true);
        y += line + gap;
        let layout = self.detail.layout(ctx, &detail, inner, Style::ui(line));
        render_text(painter, Affine::translate((card.x0 + (card.width() - f64::from(layout.width())) / 2.0, y)), layout, &[tm.text_secondary.into()], true);
        y += line + gap;
        if let Some(percent) = track {
            let bar = Rect::new(card.x0 + pad, y, card.x1 - pad, y + size::SIZE_4);
            painter.fill(RoundedRect::from_rect(bar, size::SIZE_4 / 2.0), tm.bg_surface_overlay).draw();
            let value = Rect::new(bar.x0, bar.y0, bar.x0 + bar.width() * percent.clamp(0.0, 100.0) / 100.0, bar.y1);
            painter.fill(RoundedRect::from_rect(value, size::SIZE_4 / 2.0), tm.text_primary).draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Status
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        let (title, detail) = self.texts();
        node.set_label(format!("{title}. {detail}"));
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for UpdateSplash {
    fn capture_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &PointerEvent) {
        ctx.set_handled();
    }
}
