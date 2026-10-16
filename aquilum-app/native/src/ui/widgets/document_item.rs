use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx,
    PointerButton, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update,
    UpdateCtx, Widget, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Size};
use masonry::layout::{LenReq, Length};

use crate::ui::tokens::size;
use crate::ui::{text, theme};

use super::Press;

#[derive(Clone, Debug, PartialEq)]
pub enum ItemAction {
    Open { new_tab: bool },
    Menu(Point),
    Hover(bool),
}

pub struct DocumentItem {
    label: String,
    counter: Option<String>,
    selected: bool,
    line: text::Line,
    counter_line: text::Line,
    press: Press,
}

impl DocumentItem {
    pub fn new(label: &str, counter: Option<String>) -> NewWidget<Self> {
        Self::with_selection(label, counter, false)
    }

    pub fn with_selection(label: &str, counter: Option<String>, selected: bool) -> NewWidget<Self> {
        NewWidget::new(DocumentItem {
            label: label.to_owned(),
            counter,
            selected,
            line: text::Line::default(),
            counter_line: text::Line::default(),
            press: Press::default(),
        })
    }
}

impl Widget for DocumentItem {
    type Action = ItemAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if let PointerEvent::Down(e) = event
            && e.button == Some(PointerButton::Secondary)
        {
            let at = ctx.to_window(ctx.local_position(e.state.position));
            ctx.submit_action::<ItemAction>(ItemAction::Menu(at));
        } else if let Some(e) = self.press.click(ctx, event) {
            let m = e.state.modifiers;
            ctx.submit_action::<ItemAction>(ItemAction::Open { new_tab: crate::ui::keymap::action(m) });
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            ctx.submit_action::<ItemAction>(ItemAction::Open { new_tab: false });
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(hovered) => {
                ctx.submit_action::<ItemAction>(ItemAction::Hover(*hovered));
                ctx.request_paint_only();
            }
            Update::FontsChanged => {
                self.line.clear();
                self.counter_line.clear();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(size::SIZE_32),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::ZERO,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        super::paint_item_bg(painter, bounds, self.selected, ctx.is_hovered());
        let pad = size::SPACE_6;
        let style = text::Style::ui(size::SIZE_20);
        let y = bounds.center().y - size::SIZE_20 / 2.0;
        let mut right = bounds.x1 - pad;
        if let Some(counter) = &self.counter {
            let layout = self.counter_line.layout(ctx, counter, bounds.width(), style);
            right -= f64::from(layout.width());
            render_text(painter, Affine::translate((right, y)), layout, &[t.text_primary.multiply_alpha(0.4).into()], true);
            right -= size::SPACE_8;
        }
        let width = (right - bounds.x0 - pad).max(0.0);
        let layout = self.line.layout(ctx, &self.label, width, style);
        render_text(painter, Affine::translate((bounds.x0 + pad, y)), layout, &[t.text_link.into()], true);
    }

    fn accessibility_role(&self) -> Role {
        Role::Link
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
