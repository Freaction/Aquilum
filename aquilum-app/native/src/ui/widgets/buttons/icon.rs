use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, CursorIcon, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, Update, UpdateCtx,
    Widget, WidgetMut,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::peniko::Color;
use masonry::layout::{LenReq, Length};

use crate::ui::icons::Icon;
use crate::ui::theme;
use crate::ui::tokens::size;
use crate::ui::widgets::{Press, Pressed};

pub struct IconButton {
    icon: Icon,
    label: String,
    press: Press,
    pressed: Option<bool>,
    white: bool,
    size: f64,
}

impl IconButton {
    pub fn new(icon: Icon, label: impl Into<String>) -> Self {
        IconButton { icon, label: label.into(), press: Press::default(), pressed: None, white: false, size: size::SIZE_32 }
    }

    pub fn mode(icon: Icon, label: impl Into<String>, pressed: bool) -> Self {
        IconButton { pressed: Some(pressed), ..Self::new(icon, label) }
    }

    pub fn white(icon: Icon, label: impl Into<String>) -> Self {
        IconButton { white: true, ..Self::new(icon, label) }
    }

    pub fn small(self) -> Self {
        IconButton { size: size::SIZE_24, ..self }
    }

    pub fn xs(self) -> Self {
        IconButton { size: size::SIZE_20, ..self }
    }

    pub fn set_label(this: &mut WidgetMut<'_, Self>, label: &str) {
        if this.widget.label != label {
            this.widget.label = label.to_owned();
            this.ctx.request_accessibility_update();
        }
    }

    pub fn set_icon(this: &mut WidgetMut<'_, Self>, icon: Icon) {
        this.widget.icon = icon;
        this.ctx.request_paint_only();
    }

    pub fn set_pressed(this: &mut WidgetMut<'_, Self>, pressed: bool) {
        if this.widget.pressed != Some(pressed) {
            this.widget.pressed = Some(pressed);
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }
}

impl Widget for IconButton {
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
        if matches!(event, Update::HoveredChanged(_)) {
            ctx.request_paint_only();
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, _axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(self.size)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        if self.white {
            let shape = RoundedRect::from_rect(bounds, size::ROUNDED_LG);
            crate::ui::menu::box_shadow(painter, shape, t.shadow_action);
            painter.fill(shape, t.bg_scrim).draw();
            if ctx.is_hovered() || self.press.down {
                painter.fill(shape, t.bg_surface_overlay).draw();
                painter.stroke(&shape.rect().inset(-0.5).to_rounded_rect(size::ROUNDED_LG), &Stroke::new(size::BORDER_1), t.border_contrast_inverse).draw();
            }
            let glyph = size::SIZE_16;
            self.icon.draw(painter, bounds.center() - Vec2::new(glyph / 2.0, glyph / 2.0), glyph, 1.6, t.icon_muted);
            return;
        }
        let idle = match self.pressed {
            Some(true) => t.icon_secondary,
            Some(false) => t.icon_muted,
            None => t.icon_button_icon,
        };
        let (background, color) = if ctx.is_disabled() {
            (None, t.icon_disabled)
        } else if self.press.down {
            (Some(t.icon_button_bg_active), t.icon_button_icon_hover)
        } else if self.pressed == Some(true) {
            (Some(t.icon_button_bg_active), idle)
        } else if ctx.is_hovered() {
            (Some(t.icon_button_bg_hover), t.icon_button_icon_hover)
        } else {
            (None, idle)
        };
        let (glyph, stroke) = if self.pressed.is_some() { (size::SIZE_20, 1.2) } else { (size::SIZE_16, 1.6) };
        paint_glyph(painter, bounds, self.icon, background, color, glyph, stroke);
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

pub fn paint_glyph(painter: &mut Painter<'_>, bounds: Rect, icon: Icon, background: Option<Color>, color: Color, glyph: f64, stroke: f64) {
    if let Some(bg) = background {
        painter.fill(RoundedRect::from_rect(bounds, size::ROUNDED_LG), bg).draw();
    }
    icon.draw(painter, bounds.center() - Vec2::new(glyph / 2.0, glyph / 2.0), glyph, stroke, color);
}

pub fn paint_icon(painter: &mut Painter<'_>, bounds: Rect, icon: Icon, hovered: bool, alpha: f32, tint: Option<Color>) {
    let t = theme::current();
    let color = tint.unwrap_or(if hovered { t.icon_button_icon_hover } else { t.icon_button_icon });
    paint_glyph(painter, bounds, icon, hovered.then(|| t.icon_button_bg_hover.multiply_alpha(alpha)), color.multiply_alpha(alpha), size::SIZE_16, 1.6);
}
