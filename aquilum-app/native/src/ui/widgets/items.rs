use std::time::{Duration, Instant, SystemTime};

use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, CursorIcon, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, Update,
    UpdateCtx, Widget, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, RoundedRect, Size};
use masonry::layout::{LenReq, Length};
use masonry::peniko::Color;

use crate::ui::icons::Icon;
use crate::ui::tokens::size;
use crate::ui::{text, theme};

use super::{Press, Pressed};

/// Иконка без действия: квадрат `container`, внутри иконка `size` с обводкой `stroke` в
/// пикселях (как `vector-effect: non-scaling-stroke`).
pub struct IconView {
    icon: Icon,
    container: f64,
    size: f64,
    stroke: f64,
    color: fn() -> Color,
    spin: bool,
}

const SPIN_SECONDS: f64 = 1.0;
const SPIN_FRAME: Duration = Duration::from_millis(33);

impl IconView {
    pub fn new(icon: Icon, container: f64, size: f64, stroke: f64, color: fn() -> Color) -> NewWidget<Self> {
        NewWidget::new(IconView { icon, container, size, stroke, color, spin: false })
    }

    pub fn spinning(mut view: NewWidget<Self>) -> NewWidget<Self> {
        view.widget.spin = true;
        view
    }

    fn spins(&self) -> bool {
        self.spin && crate::ui::motion::on()
    }

    fn turn(&self) -> f64 {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
        if self.spins() { now.as_secs_f64() % SPIN_SECONDS / SPIN_SECONDS * std::f64::consts::TAU } else { 0.0 }
    }
}

impl Widget for IconView {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::WidgetAdded) && self.spins() {
            ctx.request_anim_frame();
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, _interval: u64) {
        if self.spins() {
            ctx.request_paint_only();
            crate::ui::timer::wake(ctx.widget_id(), Instant::now() + SPIN_FRAME);
        }
    }

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, _axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(self.container)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let origin = ctx.content_box().center() - masonry::kurbo::Vec2::new(self.size / 2.0, self.size / 2.0);
        // Обводка задана в пикселях, а `draw` ждёт единицы сетки 24×24.
        let stroke = self.stroke * 24.0 / self.size;
        self.icon.turned(self.turn()).draw(painter, origin, self.size, stroke, (self.color)());
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
pub struct Chip {
    label: String,
    selected: bool,
    line: text::Line,
    press: Press,
}

impl Chip {
    pub fn new(label: &str, selected: bool) -> NewWidget<Self> {
        NewWidget::new(Chip { label: label.to_owned(), selected, line: text::Line::default(), press: Press::default() })
    }
}

impl Widget for Chip {
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
        Length::px(match axis {
            Axis::Horizontal => text::measure(ctx, &self.label, text::Style::ui(size::SIZE_20)).ceil() + size::SPACE_12 * 2.0,
            Axis::Vertical => size::SIZE_20 + size::SPACE_4 * 2.0,
        })
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let (bg, color) = if self.selected {
            (t.bg_accent.multiply_alpha(0.1), t.text_accent)
        } else if ctx.is_hovered() {
            (t.bg_surface_hover, t.text_secondary)
        } else {
            (t.bg_surface_subtle, t.text_secondary)
        };
        painter.fill(RoundedRect::from_rect(bounds, bounds.height() / 2.0), bg).draw();
        let width = (bounds.width() - size::SPACE_12 * 2.0).max(0.0);
        let layout = self.line.layout(ctx, &self.label, width, text::Style::ui(size::SIZE_20));
        render_text(painter, Affine::translate((bounds.x0 + size::SPACE_12, bounds.y0 + size::SPACE_4)), layout, &[color.into()], true);
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        CursorIcon::Pointer
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
        node.set_selected(self.selected);
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
