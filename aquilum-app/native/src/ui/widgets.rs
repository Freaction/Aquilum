//! Мелкие элементы оформления: кнопка-иконка (`IconButton.tsx`), кнопка в стиле строки дерева
//! (переключатель базы в подвале), граница с одной стороны.

use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx,
    PointerButton, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update,
    UpdateCtx, Widget, WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Stroke};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};
use masonry::peniko::{Color, Gradient};

use super::icons::Icon;
use super::tokens::{number, size};
use super::{text, theme};

/// Нажатие кнопки.
#[derive(Debug)]
pub struct Pressed;

/// Общая логика нажатия: захват указателя, действие при отпускании над кнопкой.
#[derive(Default)]
pub struct Press {
    pub down: bool,
}

impl Press {
    /// `true` — кнопка нажата, надо отправить действие.
    pub fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) -> bool {
        match event {
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                self.down = true;
                ctx.capture_pointer();
                ctx.request_paint_only();
                false
            }
            PointerEvent::Up(e) if e.button == Some(PointerButton::Primary) && self.down => {
                self.down = false;
                ctx.request_paint_only();
                ctx.is_hovered()
            }
            PointerEvent::Cancel(_) => {
                self.down = false;
                false
            }
            _ => false,
        }
    }
}

/// `.q-icon-button--medium`: 32×32, иконка 16 с обводкой 1.6, фон при наведении и нажатии.
pub struct IconButton {
    icon: Icon,
    label: String,
    press: Press,
    pressed: Option<bool>,
    white: bool,
}

impl IconButton {
    pub fn new(icon: Icon, label: impl Into<String>) -> Self {
        IconButton { icon, label: label.into(), press: Press::default(), pressed: None, white: false }
    }

    pub fn mode(icon: Icon, label: impl Into<String>, pressed: bool) -> Self {
        IconButton { icon, label: label.into(), press: Press::default(), pressed: Some(pressed), white: false }
    }

    pub fn white(icon: Icon, label: impl Into<String>) -> Self {
        IconButton { icon, label: label.into(), press: Press::default(), pressed: None, white: true }
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
        Length::px(size::SIZE_32)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        if self.white {
            let shape = RoundedRect::from_rect(bounds, size::ROUNDED_LG);
            super::menu::box_shadow(painter, shape, t.shadow_action);
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
        let bg = if self.active { Some(t.sidebar_item_active_bg) } else { hovered.then_some(t.sidebar_item_hover_bg) };
        if let Some(bg) = bg {
            painter.fill(RoundedRect::from_rect(bounds, size::SIDEBAR_ITEM_RADIUS), bg).draw();
        }
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
    disabled: bool,
}

impl TextButton {
    pub fn new(label: impl Into<String>, variant: Variant) -> Self {
        TextButton { label: label.into(), variant, pressed: false, icon: None, line: text::Line::default(), press: Press::default(), size: ButtonSize::S, disabled: false }
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn set_disabled(this: &mut WidgetMut<'_, Self>, disabled: bool) {
        if this.widget.disabled != disabled {
            this.widget.disabled = disabled;
            this.widget.press.down = false;
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }

    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
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
        let hovered = ctx.is_hovered() && !self.disabled;
        let color = match self.variant {
            Variant::Primary => {
                let shadow = if self.size == ButtonSize::Xs { t.shadow_action_xs } else { t.shadow_action };
                paint_primary(painter, shape, hovered, shadow);
                t.text_on_accent
            }
            Variant::Ghost => {
                if hovered || (self.press.down && !self.disabled) || self.pressed {
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

pub fn paint_primary<S: masonry::imaging::PaintSink + ?Sized>(painter: &mut Painter<'_, S>, shape: RoundedRect, hovered: bool, shadow: &[super::tokens::Shadow]) {
    let t = theme::current();
    let bounds = shape.rect();
    super::menu::box_shadow(painter, shape, shadow);
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
        if !self.disabled && self.press.pointer(ctx, event) {
            ctx.submit_action::<Pressed>(Pressed);
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if !self.disabled && event.action == Action::Click {
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
        if self.disabled {
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

/// Иконка без действия: квадрат `container`, внутри иконка `size` с обводкой `stroke` в
/// пикселях (как `vector-effect: non-scaling-stroke`).
pub struct IconView {
    icon: Icon,
    container: f64,
    size: f64,
    stroke: f64,
    color: fn() -> Color,
}

impl IconView {
    pub fn new(icon: Icon, container: f64, size: f64, stroke: f64, color: fn() -> Color) -> NewWidget<Self> {
        NewWidget::new(IconView { icon, container, size, stroke, color })
    }
}

impl Widget for IconView {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, _axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(self.container)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let origin = ctx.content_box().center() - masonry::kurbo::Vec2::new(self.size / 2.0, self.size / 2.0);
        // Обводка задана в пикселях, а `draw` ждёт единицы сетки 24×24.
        let stroke = self.stroke * 24.0 / self.size;
        self.icon.draw(painter, origin, self.size, stroke, (self.color)());
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

/// Граница с одной стороны панели: полоса толщиной 1 px поперёк `axis`, растянутая вдоль
/// (у `BorderWidth` в Masonry одна ширина на все стороны, у Flex нет выравнивания «растянуть»).
pub struct Rule {
    /// Направление полосы: `Vertical` — вертикальная линия между колонками.
    axis: Axis,
    color: fn() -> Color,
    base: Option<fn() -> Color>,
    hidden: bool,
}

impl Rule {
    pub fn new(axis: Axis, color: fn() -> Color) -> NewWidget<Self> {
        NewWidget::new(Rule { axis, color, base: None, hidden: false })
    }

    pub fn on(axis: Axis, color: fn() -> Color, base: fn() -> Color, hidden: bool) -> NewWidget<Self> {
        NewWidget::new(Rule { axis, color, base: Some(base), hidden })
    }

    pub fn set_hidden(this: &mut WidgetMut<'_, Self>, hidden: bool) {
        this.widget.hidden = hidden;
        this.ctx.request_paint_only();
    }
}

impl Widget for Rule {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis == self.axis, len_req) {
            (true, LenReq::FitContent(space)) => space,
            (true, _) => Length::ZERO,
            (false, _) => Length::px(size::BORDER_1),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if let Some(base) = self.base {
            painter.fill(ctx.content_box(), base()).draw();
        }
        if !self.hidden {
            painter.fill(ctx.content_box(), (self.color)()).draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

/// `max-width` по центру: ребёнок шириной с доступное место, но не шире `max`.
pub struct MaxWidth<W: Widget> {
    child: WidgetPod<W>,
    max: f64,
}

impl<W: Widget> MaxWidth<W> {
    pub fn new(child: NewWidget<W>, max: f64) -> NewWidget<Self> {
        NewWidget::new(MaxWidth { child: child.to_pod(), max })
    }
}

impl<W: Widget> Widget for MaxWidth<W> {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        match axis {
            Axis::Horizontal => match len_req {
                LenReq::MinContent => Length::ZERO,
                LenReq::MaxContent => Length::px(self.max),
                LenReq::FitContent(space) => space,
            },
            Axis::Vertical => {
                let inner = Length::px(cross.map_or(self.max, |c| c.get().min(self.max)));
                let context = LayoutSize::maybe(Axis::Horizontal, Some(inner));
                ctx.compute_length(&mut self.child, LenReq::MaxContent.into(), context, Axis::Vertical, Some(inner))
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let inner = size.width.min(self.max);
        let auto = SizeDef::new(LenDef::FitContent(Length::px(inner)), LenDef::MaxContent);
        let child = ctx.compute_size(&mut self.child, auto, Size::new(inner, size.height).into());
        let child = Size::new(inner, child.height);
        ctx.run_layout(&mut self.child, child);
        ctx.place_child(&mut self.child, Point::new(((size.width - inner) / 2.0).round(), 0.0));
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}

/// Гнездо на всё доступное место с заменяемым содержимым: так окно пересобирает интерфейс
/// целиком (смена темы, языка, шрифта), не пересоздавая `RenderRoot`.
pub struct Slot {
    child: WidgetPod<dyn Widget>,
    fit: bool,
}

impl Slot {
    pub fn new(child: NewWidget<dyn Widget>) -> NewWidget<Self> {
        NewWidget::new(Slot { child: child.to_pod(), fit: false })
    }

    pub fn fit(child: NewWidget<dyn Widget>) -> NewWidget<Self> {
        NewWidget::new(Slot { child: child.to_pod(), fit: true })
    }

    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<dyn Widget>) {
        this.ctx.remove_child(std::mem::replace(&mut this.widget.child, child.to_pod()));
        this.ctx.children_changed();
    }
}

impl Widget for Slot {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        if self.fit {
            let context = ctx.context_size();
            return ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross);
        }
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}

pub struct LinkButton {
    label: String,
    enabled: bool,
    line: text::Line,
    press: Press,
}

impl LinkButton {
    pub fn new(label: &str, enabled: bool) -> NewWidget<Self> {
        NewWidget::new(LinkButton { label: label.to_owned(), enabled, line: text::Line::default(), press: Press::default() })
    }

    fn style() -> text::Style {
        text::Style::ui((f64::from(theme::ui_font_settings().size) * number::FONT_LINE_HEIGHT_TIGHT).round())
    }
}

impl Widget for LinkButton {
    type Action = Pressed;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.enabled && self.press.pointer(ctx, event) {
            ctx.submit_action::<Pressed>(Pressed);
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if self.enabled && event.action == Action::Click {
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
        let color = if self.enabled { t.text_link } else { t.text_disabled };
        let layout = self.line.layout(ctx, &self.label, width, style);
        let text_width = f64::from(layout.width());
        let origin = Point::new(bounds.x0 + size::SPACE_12, bounds.y0 + size::SPACE_8);
        render_text(painter, Affine::translate(origin.to_vec2()), layout, &[color.into()], true);
        if self.enabled && (ctx.is_hovered() || self.press.down) {
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
        if self.enabled {
            node.add_action(Action::Click);
        } else {
            node.set_disabled();
        }
    }

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

#[derive(Debug)]
pub enum ItemAction {
    Open { new_tab: bool },
    Menu(Point),
}

pub struct DocumentItem {
    label: String,
    counter: Option<String>,
    selected: bool,
    line: text::Line,
    counter_line: text::Line,
    down: bool,
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
            down: false,
        })
    }
}

impl Widget for DocumentItem {
    type Action = ItemAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                self.down = true;
                ctx.capture_pointer();
            }
            PointerEvent::Down(e) if e.button == Some(PointerButton::Secondary) => {
                let at = ctx.to_window(ctx.local_position(e.state.position));
                ctx.submit_action::<ItemAction>(ItemAction::Menu(at));
            }
            PointerEvent::Up(e) if e.button == Some(PointerButton::Primary) && self.down => {
                self.down = false;
                if ctx.is_hovered() {
                    let m = e.state.modifiers;
                    ctx.submit_action::<ItemAction>(ItemAction::Open { new_tab: m.ctrl() || m.meta() });
                }
            }
            PointerEvent::Cancel(_) => self.down = false,
            _ => {}
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            ctx.submit_action::<ItemAction>(ItemAction::Open { new_tab: false });
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(_) => ctx.request_paint_only(),
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
        let bg = if self.selected { Some(t.sidebar_item_active_bg) } else { ctx.is_hovered().then_some(t.sidebar_item_hover_bg) };
        if let Some(bg) = bg {
            painter.fill(RoundedRect::from_rect(bounds, size::SIDEBAR_ITEM_RADIUS), bg).draw();
        }
        let style = text::Style::ui(size::SIZE_20);
        let pad = size::SPACE_6;
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

pub struct Wrap {
    children: Vec<WidgetPod<dyn Widget>>,
    gap: f64,
}

impl Wrap {
    pub fn new(children: Vec<NewWidget<dyn Widget>>, gap: f64) -> NewWidget<Self> {
        NewWidget::new(Wrap { children: children.into_iter().map(NewWidget::to_pod).collect(), gap })
    }

    fn sizes(&mut self, ctx: &mut MeasureCtx<'_>) -> Vec<Size> {
        self.children
            .iter_mut()
            .map(|child| {
                let width = ctx.compute_length(child, LenReq::MaxContent.into(), LayoutSize::NONE, Axis::Horizontal, None).get();
                let context = LayoutSize::maybe(Axis::Horizontal, Some(Length::px(width)));
                let height = ctx.compute_length(child, LenReq::MaxContent.into(), context, Axis::Vertical, Some(Length::px(width))).get();
                Size::new(width, height)
            })
            .collect()
    }

    fn rows(sizes: &[Size], width: f64, gap: f64) -> Vec<(usize, Point)> {
        let (mut x, mut y, mut line) = (0.0, 0.0, 0.0_f64);
        let mut placed = Vec::with_capacity(sizes.len());
        for (index, size) in sizes.iter().enumerate() {
            if x > 0.0 && x + size.width > width {
                x = 0.0;
                y += line + gap;
                line = 0.0;
            }
            placed.push((index, Point::new(x, y)));
            x += size.width + gap;
            line = line.max(size.height);
        }
        placed
    }
}

impl Widget for Wrap {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for child in &mut self.children {
            ctx.register_child(child);
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let sizes = self.sizes(ctx);
        match axis {
            Axis::Horizontal => match len_req {
                LenReq::FitContent(space) => space,
                LenReq::MinContent => Length::px(sizes.iter().map(|s| s.width).fold(0.0, f64::max)),
                LenReq::MaxContent => {
                    Length::px(sizes.iter().map(|s| s.width).sum::<f64>() + self.gap * sizes.len().saturating_sub(1) as f64)
                }
            },
            Axis::Vertical => {
                let width = cross.map_or(f64::INFINITY, |c| c.get());
                let placed = Self::rows(&sizes, width, self.gap);
                let bottom = placed.iter().map(|(i, p)| p.y + sizes[*i].height).fold(0.0, f64::max);
                Length::px(bottom)
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let mut sizes = Vec::with_capacity(self.children.len());
        for child in &mut self.children {
            let auto = SizeDef::new(LenDef::MaxContent, LenDef::MaxContent);
            sizes.push(ctx.compute_size(child, auto, LayoutSize::NONE));
        }
        for (index, origin) in Self::rows(&sizes, size.width, self.gap) {
            let child = &mut self.children[index];
            ctx.run_layout(child, sizes[index]);
            ctx.place_child(child, origin);
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        self.children.iter().map(WidgetPod::id).collect()
    }
}

pub struct Underlined {
    child: WidgetPod<dyn Widget>,
    base: fn() -> Color,
    line: fn() -> Color,
}

impl Underlined {
    pub fn new(child: NewWidget<impl Widget + ?Sized>, base: fn() -> Color, line: fn() -> Color) -> NewWidget<Self> {
        NewWidget::new(Underlined { child: child.erased().to_pod(), base, line })
    }
}

impl Widget for Underlined {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let context = masonry::layout::LayoutSize::maybe(axis.cross(), cross);
        let inner = ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross);
        if axis == Axis::Vertical { Length::px(inner.get() + size::BORDER_1) } else { inner }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let inner = Size::new(size.width, (size.height - size::BORDER_1).max(0.0));
        ctx.run_layout(&mut self.child, inner);
        ctx.place_child(&mut self.child, Point::ZERO);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let bounds = ctx.content_box();
        painter.fill(bounds, (self.base)()).draw();
        painter.fill(Rect::new(bounds.x0, bounds.y1 - size::BORDER_1, bounds.x1, bounds.y1), (self.line)()).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
