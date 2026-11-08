use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, CursorIcon, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, Update, UpdateCtx,
    NewWidget, PropertySet, Widget, WidgetMut,
};
use masonry::imaging::{Composite, GroupRef, PaintSink, Painter};
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LenReq, Length};
use masonry::peniko::BlendMode;
use masonry::properties::{BoxShadow, CornerRadius, Gap};
use masonry::widgets::Flex;

use super::{Joined, Look, Variant, paint_button};
use crate::ui::icons::Icon;
use crate::ui::tokens::number;
use crate::ui::widgets::{ControlSize, Press, Pressed};
use crate::ui::text;

pub struct TextButton {
    label: String,
    variant: Variant,
    pressed: bool,
    icons: [Option<Icon>; 2],
    line: text::Line,
    press: Press,
    size: ControlSize,
    joined: Option<Joined>,
    icon_only: bool,
}

impl TextButton {
    pub fn new(label: impl Into<String>, variant: Variant) -> Self {
        TextButton { label: label.into(), variant, pressed: false, icons: [None, None], line: text::Line::default(), press: Press::default(), size: ControlSize::S.fit(), joined: None, icon_only: false }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size.fit();
        self
    }

    pub fn icon_only(mut self, icon: Icon) -> Self {
        self.icons = [Some(icon), None];
        self.icon_only = true;
        self
    }

    /// Compose two base buttons, retaining their own actions and accessibility labels.
    pub fn split(mut left: NewWidget<Self>, mut right: NewWidget<Self>) -> NewWidget<Flex> {
        left.widget.joined = Some(Joined::Left);
        right.widget.joined = Some(Joined::Right);
        right.widget.size = left.widget.size;
        let size = left.widget.size;
        let mut props = PropertySet::new().with(Gap::new(Length::ZERO)).with(CornerRadius::all(Length::px(size.radius())));
        if left.widget.variant == Variant::Primary {
            let theme = crate::ui::theme::current();
            let shadows = if size == ControlSize::Xs { theme.shadow_action_xs } else { theme.shadow_action };
            if let Some(shadow) = shadows.first() {
                props = props.with(BoxShadow::new(shadow.color, (shadow.x, shadow.y)).blur(Length::px(shadow.blur / 2.0)));
            }
        }
        NewWidget::new(Flex::row().with_fixed(left).with_fixed(right)).with_props(props)
    }

    pub fn set_disabled(this: &mut WidgetMut<'_, Self>, disabled: bool) {
        let repaint = this.widget.press.down || this.ctx.is_disabled() != disabled;
        this.widget.press.down = false;
        this.ctx.set_disabled(disabled);
        if repaint { this.ctx.request_paint_only(); }
    }

    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub fn set_pressed(this: &mut WidgetMut<'_, Self>, pressed: bool) {
        if this.widget.pressed != pressed {
            this.widget.pressed = pressed;
            this.ctx.request_paint_only();
        }
    }

    pub fn icon_before(mut self, icon: Icon) -> Self {
        self.icons[0] = Some(icon);
        self
    }

    pub fn icon_after(mut self, icon: Icon) -> Self {
        self.icons[1] = Some(icon);
        self
    }

    fn chrome(&self) -> f64 {
        self.size.chrome(self.icons.iter().flatten().count())
    }

    fn draw<S: PaintSink + ?Sized>(&mut self, ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_, S>) {
        let bounds = ctx.content_box();
        let enabled = !ctx.is_disabled();
        let hovered = ctx.is_hovered() && enabled;
        let look = Look { variant: self.variant, size: self.size, hovered, active: hovered || (self.press.down && enabled) || self.pressed, joined: self.joined, icon_only: self.icon_only };
        let available = (bounds.width() - self.chrome()).max(0.0);
        let layout = self.line.layout(ctx, if self.icon_only { "" } else { &self.label }, available, self.size.text());
        paint_button(painter, bounds, look, layout, &self.icons);
    }
}

impl Widget for TextButton {
    type Action = Pressed;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.press.pointer(ctx, event) {
            ctx.submit_action::<Pressed>(Pressed);
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            ctx.submit_action::<Pressed>(Pressed);
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

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        match axis {
            Axis::Vertical => Length::px(self.size.height()),
            Axis::Horizontal => {
                if self.icon_only { return Length::px(self.size.height()); }
                let (fcx, lcx) = ctx.text_contexts();
                let width = self.line.width(fcx, lcx, &self.label, self.size.text());
                Length::px(width.ceil() + self.chrome())
            }
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if ctx.is_disabled() {
            let group = GroupRef::new().with_composite(Composite::new(BlendMode::default(), number::OPACITY_50 as f32));
            painter.with_group(group, |painter| self.draw(ctx, painter));
        } else {
            self.draw(ctx, painter);
        }
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        CursorIcon::Pointer
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
