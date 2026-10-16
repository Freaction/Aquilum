use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetMut, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Size};
use masonry::layout::{LenReq, Length};

use super::Paint;
use crate::ui::tokens::size;
use crate::ui::{text, theme};

pub struct Caption {
    text: String,
    line: text::Line,
    paint: Paint,
    trailing: bool,
}

impl Caption {
    pub fn new(text: &str) -> Self {
        Caption { text: text.to_owned(), line: text::Line::default(), paint: |t| t.text_tertiary, trailing: false }
    }

    pub fn trailing(text: &str, paint: Paint) -> Self {
        Caption { paint, trailing: true, ..Caption::new(text) }
    }

    pub fn set_text(this: &mut WidgetMut<'_, Self>, text: &str) {
        if this.widget.text != text {
            this.widget.text = text.to_owned();
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }
}

impl Widget for Caption {
    type Action = masonry::core::NoAction;

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::FontsChanged) {
            self.line.clear();
            ctx.request_paint_only();
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(size::SIZE_20),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::ZERO,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let bounds = ctx.content_box();
        let layout = self.line.layout(ctx, &self.text, bounds.width(), text::Style::ui(size::SIZE_20));
        let room = (bounds.width() - f64::from(layout.width())).max(0.0);
        let x = bounds.x0 + if self.trailing { room } else { room / 2.0 };
        let y = bounds.center().y - size::SIZE_20 / 2.0;
        render_text(painter, Affine::translate((x, y)), layout, &[(self.paint)(&theme::current()).into()], true);
    }

    fn accessibility_role(&self) -> Role {
        Role::Label
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_value(self.text.clone());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
