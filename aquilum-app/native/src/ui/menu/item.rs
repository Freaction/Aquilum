use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx,
    Widget, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, RoundedRect, Size};
use masonry::layout::{LenReq, Length};

use super::MenuChoice;
use crate::ui::tokens::size;
use crate::ui::widgets::Press;
use crate::ui::{icons, text, theme};

pub(super) struct MenuItem {
    index: usize,
    label: String,
    icon: Option<icons::Icon>,
    disabled: bool,
    checked: bool,
    checkable: bool,
    line: text::Line,
    press: Press,
}

impl MenuItem {
    pub(super) fn new(index: usize, label: &str, icon: Option<icons::Icon>, disabled: bool, checked: bool, checkable: bool) -> Self {
        MenuItem { index, label: label.to_owned(), icon, disabled, checked, checkable, line: text::Line::default(), press: Press::default() }
    }

    pub(super) fn height() -> f64 {
        size::PADDING_2XS * 2.0 + size::SIZE_20
    }
}

impl Widget for MenuItem {
    type Action = MenuChoice;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.press.pointer(ctx, event) && !self.disabled {
            ctx.submit_action::<MenuChoice>(MenuChoice::Item(self.index));
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click && !self.disabled {
            ctx.submit_action::<MenuChoice>(MenuChoice::Item(self.index));
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(_) => ctx.request_paint_only(),
            Update::FontsChanged => {
                self.line.clear();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(Self::height()),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => {
                let width = text::measure(ctx, &self.label, text::Style::ui(size::SIZE_20));
                let icon = if self.icon.is_some() { size::SIZE_20 + size::GAP_MD } else { 0.0 };
                let check = if self.checkable { size::SIZE_20 + size::GAP_MD } else { 0.0 };
                Length::px(width.ceil() + size::PADDING_XS * 2.0 + icon + check)
            }
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let hovered = ctx.is_hovered() && !self.disabled;
        if hovered {
            painter.fill(RoundedRect::from_rect(bounds, size::RADIUS_LG), t.bg_accent).draw();
        }
        let lead = if self.icon.is_some() { size::SIZE_20 + size::GAP_MD } else { 0.0 };
        if let Some(icon) = self.icon {
            let color = if self.disabled {
                t.text_disabled
            } else if hovered {
                t.icon_on_accent
            } else {
                t.icon_muted
            };
            icon.draw(painter, Point::new(bounds.x0 + size::PADDING_XS, bounds.y0 + size::PADDING_2XS), size::SIZE_20, 1.2, color);
        }
        let check = if self.checkable { size::SIZE_20 + size::GAP_MD } else { 0.0 };
        let available = (bounds.width() - size::PADDING_XS * 2.0 - check - lead).max(0.0);
        let layout = self.line.layout(ctx, &self.label, available, text::Style::ui(size::SIZE_20));
        let color = if self.disabled {
            t.text_disabled
        } else if hovered {
            t.text_on_accent
        } else {
            t.text_secondary
        };
        let origin = Affine::translate((bounds.x0 + size::PADDING_XS + lead, bounds.y0 + size::PADDING_2XS));
        render_text(painter, origin, layout, &[color.into()], true);
        if self.checked {
            let at = Point::new(bounds.x1 - size::PADDING_XS - size::SIZE_20, bounds.y0 + size::PADDING_2XS);
            let color = if hovered { t.icon_on_accent } else { t.icon_muted };
            icons::CHECK.draw(painter, at, size::SIZE_20, 1.6, color);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::MenuItem
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
        if self.checked {
            node.set_selected(true);
        }
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
