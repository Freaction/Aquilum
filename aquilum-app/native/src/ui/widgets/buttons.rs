use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update,
    UpdateCtx, Widget, WidgetMut, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, RoundedRect, Size, Stroke};
use masonry::layout::{LenReq, Length};
use masonry::peniko::{Color, Gradient};

use crate::ui::icons::Icon;
use crate::ui::tokens::{number, size};
use crate::ui::{text, theme};

use super::{Press, Pressed};

/// `.q-icon-button--medium`: 32×32, иконка 16 с обводкой 1.6, фон при наведении и нажатии.
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
        IconButton { size: size::SIZE_20, ..self }
    }

    pub fn set_label(this: &mut WidgetMut<'_, Self>, label: &str) {
        if this.widget.label != label {
            this.widget.label = label.to_owned();
            this.ctx.request_accessibility_update();
        }
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
            self.icon.draw(painter, bounds.center() - masonry::kurbo::Vec2::new(glyph / 2.0, glyph / 2.0), glyph, 1.6, t.icon_muted);
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
        if let Some(bg) = background {
            painter.fill(RoundedRect::from_rect(bounds, size::ROUNDED_LG), bg).draw();
        }
        let (glyph, stroke) = if self.pressed.is_some() { (size::SIZE_20, 1.2) } else { (size::SIZE_16, 1.6) };
        let origin = bounds.center() - masonry::kurbo::Vec2::new(glyph / 2.0, glyph / 2.0);
        self.icon.draw(painter, origin, glyph, stroke, color);
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

/// Кнопка в стиле строки дерева (`.q-file-item`): иконка в слоте 20, текст с многоточием.
/// Активная (`.q-file-item.active`) — пункт навигации, на котором стоим.
pub struct ItemButton {
    icon: Icon,
    text: String,
    line: text::Line,
    press: Press,
    active: bool,
}

impl ItemButton {
    pub fn new(icon: Icon, text: impl Into<String>) -> Self {
        ItemButton { icon, text: text.into(), line: text::Line::default(), press: Press::default(), active: false }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn set_active(this: &mut WidgetMut<'_, Self>, active: bool) {
        if this.widget.active != active {
            this.widget.active = active;
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }
}

impl Widget for ItemButton {
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
            // Шрифты регистрируются после создания окна: текст надо разложить и измерить заново.
            Update::FontsChanged => {
                self.line.clear();
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
            (Axis::Horizontal, _) => Length::px(size::SIZE_32),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let hovered = ctx.is_hovered() || self.press.down;
        paint_item_bg(painter, bounds, self.active, hovered);
        let hovered = hovered || self.active;
        let pad = size::SIDEBAR_ITEM_PADDING;
        let slot = size::SIDEBAR_ICON_SIZE;
        let glyph = size::SIZE_16;
        let inset = (slot - glyph) / 2.0;
        let icon_color = if hovered { t.sidebar_icon_hover } else { t.sidebar_icon };
        self.icon.draw(painter, Point::new(bounds.x0 + pad + inset, bounds.center().y - glyph / 2.0), glyph, 1.6, icon_color);

        let text_x = pad + slot + size::SIDEBAR_ITEM_GAP;
        let available = (bounds.width() - text_x - pad).max(0.0);
        let layout = self.line.layout(ctx, &self.text, available, text::Style::ui(slot));
        let origin = Affine::translate((bounds.x0 + text_x, bounds.center().y - slot / 2.0));
        render_text(painter, origin, layout, &[t.sidebar_text.into()], true);
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.text.clone());
        if self.active {
            node.set_selected(true);
        }
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

/// Вариант кнопки с текстом (`.q-button--primary`, `.q-button--ghost`).
#[derive(Clone, Copy, PartialEq)]
pub enum Variant {
    Primary,
    Ghost,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonSize {
    M,
    S,
    Xs,
}

impl ButtonSize {
    fn height(self) -> f64 {
        match self {
            ButtonSize::M => size::BUTTON_HEIGHT,
            ButtonSize::S => size::BUTTON_S_HEIGHT,
            ButtonSize::Xs => size::BUTTON_XS_HEIGHT,
        }
    }

    fn padding(self) -> f64 {
        match self {
            ButtonSize::M => size::BUTTON_PADDING_X,
            ButtonSize::S => size::BUTTON_S_PADDING_X,
            ButtonSize::Xs => size::BUTTON_XS_PADDING_X,
        }
    }

    fn radius(self) -> f64 {
        match self {
            ButtonSize::M => size::BUTTON_RADIUS,
            ButtonSize::S => size::BUTTON_S_RADIUS,
            ButtonSize::Xs => size::BUTTON_XS_RADIUS,
        }
    }

    fn line_height(self) -> f64 {
        match self {
            ButtonSize::M => size::BUTTON_LINE_HEIGHT,
            ButtonSize::S => size::BUTTON_S_LINE_HEIGHT,
            ButtonSize::Xs => size::BUTTON_XS_LINE_HEIGHT,
        }
    }
}

/// Кнопка с текстом размера `s` — как в подвале диалогов (`.q-dialog__footer .q-button`).
pub struct TextButton {
    label: String,
    variant: Variant,
    pressed: bool,
    icon: Option<Icon>,
    line: text::Line,
    press: Press,
    size: ButtonSize,
}

impl TextButton {
    pub fn new(label: impl Into<String>, variant: Variant) -> Self {
        TextButton { label: label.into(), variant, pressed: false, icon: None, line: text::Line::default(), press: Press::default(), size: ButtonSize::S }
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn set_disabled(this: &mut WidgetMut<'_, Self>, disabled: bool) {
        this.widget.press.down = false;
        this.ctx.set_disabled(disabled);
        this.ctx.request_paint_only();
    }

    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub fn set_pressed(this: &mut WidgetMut<'_, Self>, pressed: bool) {
        this.widget.pressed = pressed;
        this.ctx.request_paint_only();
    }

    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    fn icon_width(&self) -> f64 {
        if self.icon.is_some() { size::SIZE_16 + size::SPACE_6 } else { 0.0 }
    }

    fn draw<S: masonry::imaging::PaintSink + ?Sized>(&mut self, ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_, S>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let shape = RoundedRect::from_rect(bounds, self.size.radius());
        let enabled = !ctx.is_disabled();
        let hovered = ctx.is_hovered() && enabled;
        let color = match self.variant {
            Variant::Primary => {
                let shadow = if self.size == ButtonSize::Xs { t.shadow_action_xs } else { t.shadow_action };
                paint_primary(painter, shape, hovered, shadow);
                t.text_on_accent
            }
            Variant::Ghost => {
                if hovered || (self.press.down && enabled) || self.pressed {
                    painter.fill(shape, t.bg_surface_overlay).draw();
                }
                t.text_tertiary
            }
        };
        let style = self.style();
        let icon_width = self.icon_width();
        let available = (bounds.width() - self.size.padding() * 2.0 - icon_width).max(0.0);
        let layout = self.line.layout(ctx, &self.label, available, style);
        let x = bounds.x0 + (bounds.width() - f64::from(layout.width()) - icon_width) / 2.0;
        let y = bounds.y0 + (bounds.height() - style.line_height) / 2.0;
        if let Some(icon) = &self.icon {
            let glyph = size::SIZE_16;
            icon.draw(painter, Point::new(x, bounds.center().y - glyph / 2.0), glyph, 1.6, color);
        }
        render_text(painter, Affine::translate((x + icon_width, y)), layout, &[color.into()], true);
    }

    fn style(&self) -> text::Style {
        let weight = match self.variant {
            Variant::Primary => number::FONT_WEIGHT_UI_MEDIUM,
            Variant::Ghost => number::FONT_WEIGHT_UI_BASE,
        };
        text::Style { size: size::BUTTON_FONT_SIZE as f32, weight: weight as f32, line_height: self.size.line_height() }
    }
}

pub fn paint_primary<S: masonry::imaging::PaintSink + ?Sized>(painter: &mut Painter<'_, S>, shape: RoundedRect, hovered: bool, shadow: &[crate::ui::tokens::Shadow]) {
    let t = theme::current();
    let bounds = shape.rect();
    crate::ui::menu::box_shadow(painter, shape, shadow);
    painter.stroke(&shape, &Stroke::new(1.0), t.bg_accent).draw();
    painter.fill(shape, t.bg_accent).draw();
    let (fill, ring) = if hovered {
        (number::BUTTON_SHINE_FILL_HOVER, number::BUTTON_SHINE_RING_HOVER)
    } else {
        (number::BUTTON_SHINE_FILL_DEFAULT, number::BUTTON_SHINE_RING_DEFAULT)
    };
    let shine = |alpha: f64| Gradient::new_linear((bounds.x0, bounds.y0), (bounds.x0, bounds.y1)).with_stops([Color::WHITE.multiply_alpha(alpha as f32), Color::WHITE.multiply_alpha(0.0)]);
    painter.fill(shape, &shine(fill)).draw();
    let edge = size::BUTTON_HIGHLIGHT_WIDTH;
    let inner = RoundedRect::from_rect(bounds.inset(-edge / 2.0), shape.radii().top_left - edge / 2.0);
    painter.stroke(&inner, &Stroke::new(edge), &shine(ring)).draw();
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
            // Шрифты регистрируются после создания окна: текст надо разложить и измерить заново.
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
                let width = text::measure(ctx, &self.label, self.style());
                Length::px(width.ceil() + self.icon_width() + self.size.padding() * 2.0)
            }
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if ctx.is_disabled() {
            let group = masonry::imaging::GroupRef::new().with_composite(masonry::imaging::Composite::new(masonry::peniko::BlendMode::default(), number::OPACITY_50 as f32));
            painter.with_group(group, |painter| self.draw(ctx, painter));
        } else {
            self.draw(ctx, painter);
        }
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
pub struct LinkButton {
    label: String,
    line: text::Line,
    press: Press,
}

impl LinkButton {
    pub fn new(label: &str) -> NewWidget<Self> {
        NewWidget::new(LinkButton { label: label.to_owned(), line: text::Line::default(), press: Press::default() })
    }

    fn style() -> text::Style {
        text::Style::ui((f64::from(theme::ui_font_settings().size) * number::FONT_LINE_HEIGHT_TIGHT).round())
    }
}

impl Widget for LinkButton {
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
        let style = Self::style();
        Length::px(match axis {
            Axis::Horizontal => text::measure(ctx, &self.label, style).ceil() + size::SPACE_12 * 2.0,
            Axis::Vertical => style.line_height + size::SPACE_8 * 2.0,
        })
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let style = Self::style();
        let width = (bounds.width() - size::SPACE_12 * 2.0).max(0.0);
        let enabled = !ctx.is_disabled();
        let color = if enabled { t.text_link } else { t.text_disabled };
        let layout = self.line.layout(ctx, &self.label, width, style);
        let text_width = f64::from(layout.width());
        let origin = Point::new(bounds.x0 + size::SPACE_12, bounds.y0 + size::SPACE_8);
        render_text(painter, Affine::translate(origin.to_vec2()), layout, &[color.into()], true);
        if enabled && (ctx.is_hovered() || self.press.down) {
            let y = origin.y + style.line_height - 2.0;
            let underline = masonry::kurbo::Line::new((origin.x, y), (origin.x + text_width, y));
            painter.stroke(underline, &Stroke::new(1.0), color).draw();
        }
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

pub fn paint_close(painter: &mut Painter<'_>, rect: masonry::kurbo::Rect, hovered: bool, alpha: f32) {
    let t = theme::current();
    if hovered {
        painter.fill(RoundedRect::from_rect(rect, size::ROUNDED_LG), t.icon_button_bg_hover.multiply_alpha(alpha)).draw();
    }
    let glyph = size::SIZE_16;
    let color = if hovered { t.icon_button_icon_hover } else { t.icon_button_icon };
    crate::ui::icons::X.draw(painter, rect.center() - masonry::kurbo::Vec2::new(glyph / 2.0, glyph / 2.0), glyph, 1.6, color.multiply_alpha(alpha));
}

pub fn paint_item_bg(painter: &mut Painter<'_>, bounds: masonry::kurbo::Rect, active: bool, hovered: bool) {
    let t = theme::current();
    let bg = if active { Some(t.sidebar_item_active_bg) } else { hovered.then_some(t.sidebar_item_hover_bg) };
    if let Some(bg) = bg {
        painter.fill(RoundedRect::from_rect(bounds, size::SIDEBAR_ITEM_RADIUS), bg).draw();
    }
}
