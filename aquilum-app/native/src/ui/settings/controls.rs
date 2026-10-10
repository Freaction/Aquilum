use masonry::accesskit::{Action, Node, Role, Toggled};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx,
    PointerButton, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget,
    WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Circle, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};

use crate::ui::icons::{self, Icon};
use crate::ui::menu::box_shadow;
use crate::ui::tokens::{Shadow, size};
use crate::ui::widgets::Press;
use crate::ui::{text, theme};

#[derive(Clone, Debug, PartialEq)]
pub enum Changed {
    Bool(bool),
    Choice(usize),
    OpenMenu(Rect),
    Step(f64),
    Value(f64),
}

pub struct Slider {
    label: String,
    hint: String,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    dragging: bool,
    label_line: text::Line,
    hint_line: text::Line,
}

impl Slider {
    pub fn new(label: &str, value: f64, min: f64, max: f64, step: f64, hint: &str) -> NewWidget<Self> {
        NewWidget::new(Slider { label: label.to_owned(), hint: hint.to_owned(), value, min, max, step, dragging: false, label_line: text::Line::default(), hint_line: text::Line::default() })
    }

    pub fn set(this: &mut WidgetMut<'_, Self>, value: f64, hint: &str) {
        if this.widget.value != value || this.widget.hint != hint {
            this.widget.value = value;
            hint.clone_into(&mut this.widget.hint);
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }

    fn line() -> f64 {
        (f64::from(theme::ui_font_settings().size) * crate::ui::tokens::number::FONT_LINE_HEIGHT_NORMAL).round()
    }

    fn track(&self, bounds: Rect) -> (f64, f64, f64) {
        let r = size::SIZE_12 / 2.0;
        let y = bounds.y0 + Self::line() + size::SPACE_4 + size::SIZE_16 / 2.0;
        (bounds.x0 + r, bounds.x1 - r, y)
    }

    fn snap(&self, value: f64) -> f64 {
        let steps = ((value - self.min) / self.step).round();
        (self.min + steps * self.step).clamp(self.min, self.max)
    }

    fn set_from(&mut self, ctx: &mut EventCtx<'_>, x: f64) {
        let (x0, x1, _) = self.track(ctx.content_box());
        let share = ((x - x0) / (x1 - x0).max(1.0)).clamp(0.0, 1.0);
        let value = self.snap(self.min + share * (self.max - self.min));
        self.move_to(ctx, value);
    }

    fn move_to(&mut self, ctx: &mut EventCtx<'_>, value: f64) {
        if (value - self.value).abs() > f64::EPSILON {
            self.value = value;
            ctx.request_paint_only();
            ctx.submit_action::<Changed>(Changed::Value(value));
        }
    }
}

impl Widget for Slider {
    type Action = Changed;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down(down) if down.button.is_none_or(|b| b == PointerButton::Primary) => {
                self.dragging = true;
                ctx.capture_pointer();
                ctx.request_focus();
                let x = ctx.local_position(down.state.position).x;
                self.set_from(ctx, x);
            }
            PointerEvent::Move(update) if self.dragging => {
                let x = ctx.local_position(update.current.position).x;
                self.set_from(ctx, x);
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => self.dragging = false,
            _ => {}
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &masonry::core::TextEvent) {
        use masonry::core::keyboard::{Key, KeyState, NamedKey};
        let masonry::core::TextEvent::Keyboard(k) = event else { return };
        if k.state != KeyState::Down {
            return;
        }
        let value = match &k.key {
            Key::Named(NamedKey::ArrowRight | NamedKey::ArrowUp) => self.value + self.step,
            Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowDown) => self.value - self.step,
            Key::Named(NamedKey::Home) => self.min,
            Key::Named(NamedKey::End) => self.max,
            _ => return,
        };
        let value = self.snap(value);
        self.move_to(ctx, value);
        ctx.set_handled();
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        let value = match event.action {
            Action::Increment => self.value + self.step,
            Action::Decrement => self.value - self.step,
            _ => return,
        };
        let value = self.snap(value);
        self.move_to(ctx, value);
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FocusChanged(_) => ctx.request_paint_only(),
            Update::FontsChanged => {
                self.label_line.clear();
                self.hint_line.clear();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match axis {
            Axis::Horizontal => match len_req {
                LenReq::FitContent(space) => space,
                _ => Length::px(size::SIZE_128),
            },
            Axis::Vertical => Length::px(Self::line() + size::SPACE_4 + size::SIZE_16),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let line = Self::line();
        let hint_width = {
            let layout = self.hint_line.layout(ctx, &self.hint, bounds.width(), text::Style::ui(line));
            let width = f64::from(layout.width());
            render_text(painter, Affine::translate((bounds.x1 - width, bounds.y0)), layout, &[t.text_tertiary.into()], true);
            width
        };
        let label = self.label_line.layout(ctx, &self.label, (bounds.width() - hint_width - size::SPACE_8).max(1.0), text::Style::ui(line));
        render_text(painter, Affine::translate((bounds.x0, bounds.y0)), label, &[t.text_secondary.into()], true);
        let (x0, x1, y) = self.track(bounds);
        let half = size::UNIT_3 / 2.0;
        painter.fill(RoundedRect::new(bounds.x0, y - half, bounds.x1, y + half, size::ROUNDED_SM), t.bg_surface_overlay).draw();
        let share = if self.max > self.min { (self.value - self.min) / (self.max - self.min) } else { 0.0 };
        let center = Point::new(x0 + (x1 - x0) * share.clamp(0.0, 1.0), y);
        painter.fill(Circle::new(center, size::SIZE_12 / 2.0), t.text_accent).draw();
        if ctx.is_focus_target() {
            let ring = RoundedRect::from_rect(Rect::new(bounds.x0, y - size::SIZE_16 / 2.0, bounds.x1, y + size::SIZE_16 / 2.0).inflate(size::SPACE_4, size::SPACE_4), size::ROUNDED_SM);
            painter.stroke(&ring, &Stroke::new(size::BORDER_1), t.text_accent).draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Slider
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
        node.set_numeric_value(self.value);
        node.set_min_numeric_value(self.min);
        node.set_max_numeric_value(self.max);
        node.set_numeric_value_step(self.step);
        node.set_value(self.hint.clone());
        node.add_action(Action::Increment);
        node.add_action(Action::Decrement);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub struct Switch {
    on: bool,
    enabled: bool,
    label: String,
    press: Press,
    travel: f64,
}

const SWITCH_SECONDS: f64 = 0.2;

fn ease_standard(t: f64) -> f64 {
    let (x1, y1, x2, y2) = (0.4, 0.0, 0.2, 1.0);
    let bezier = |t: f64, a: f64, b: f64| 3.0 * a * t * (1.0 - t) * (1.0 - t) + 3.0 * b * t * t * (1.0 - t) + t * t * t;
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if bezier(mid, x1, x2) < t { lo = mid } else { hi = mid }
    }
    bezier((lo + hi) / 2.0, y1, y2)
}

impl Switch {
    pub fn new(on: bool, label: &str) -> NewWidget<Self> {
        NewWidget::new(Switch { on, enabled: true, label: label.to_owned(), press: Press::default(), travel: if on { 1.0 } else { 0.0 } })
    }

    pub fn locked(on: bool, label: &str) -> NewWidget<Self> {
        NewWidget::new(Switch { on, enabled: false, label: label.to_owned(), press: Press::default(), travel: if on { 1.0 } else { 0.0 } })
    }

    fn toggle(&mut self, ctx: &mut EventCtx<'_>) {
        if !self.enabled {
            return;
        }
        self.on = !self.on;
        ctx.request_anim_frame();
        ctx.request_paint_only();
        ctx.request_accessibility_update();
        ctx.submit_action::<Changed>(Changed::Bool(self.on));
    }
}

impl Widget for Switch {
    type Action = Changed;

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let goal = if self.on { 1.0 } else { 0.0 };
        let step = interval as f64 / 1e9 / SWITCH_SECONDS;
        self.travel = if self.travel < goal { (self.travel + step).min(goal) } else { (self.travel - step).max(goal) };
        ctx.request_paint_only();
        if self.travel != goal {
            ctx.request_anim_frame();
        }
    }

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.press.pointer(ctx, event) {
            self.toggle(ctx);
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            self.toggle(ctx);
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(match axis {
            Axis::Horizontal => size::SWITCH_WIDTH,
            Axis::Vertical => size::SWITCH_HEIGHT,
        })
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let alpha = if self.enabled { 1.0 } else { 0.5 };
        let track = RoundedRect::from_rect(bounds, bounds.height() / 2.0);
        let track_color = if self.on { t.bg_accent } else { t.bg_surface_active };
        painter.fill(track, track_color.multiply_alpha(alpha)).draw();
        let inset = size::SWITCH_HANDLE_INSET;
        let x = bounds.x0 + inset + size::SWITCH_HANDLE_TRAVEL * ease_standard(self.travel);
        let r = size::SWITCH_HANDLE_SIZE / 2.0;
        let handle = Circle::new((x + r, bounds.center().y), r);
        let rect = RoundedRect::from_rect(Rect::from_center_size(handle.center, Size::new(r * 2.0, r * 2.0)), r);
        let glow = |blur, color: masonry::peniko::Color| Shadow { x: 0.0, y: 0.0, blur, spread: 0.0, color: color.multiply_alpha(alpha) };
        box_shadow(painter, rect, &[glow(1.0, t.shadow_sm), glow(4.0, t.shadow_sm), glow(44.0, t.shadow_md)]);
        painter.fill(handle, t.bg_static_white.multiply_alpha(alpha)).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::Switch
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
        node.set_toggled(if self.on { Toggled::True } else { Toggled::False });
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

pub struct Segmented {
    options: Vec<String>,
    selected: usize,
    hovered: Option<usize>,
    lines: Vec<text::Line>,
    rects: Vec<Rect>,
    stretch: bool,
    shown: f64,
}

const SEGMENT_PAD: f64 = 3.0;
const SEGMENT_GAP: f64 = 2.0;
const SEGMENT_HEIGHT: f64 = 24.0;
const SEGMENT_SECONDS: f64 = 0.15;

impl Segmented {
    pub fn new(options: Vec<String>, selected: usize) -> NewWidget<Self> {
        Self::build(options, selected, false)
    }

    pub fn stretched(options: Vec<String>, selected: usize) -> NewWidget<Self> {
        Self::build(options, selected, true)
    }

    fn build(options: Vec<String>, selected: usize, stretch: bool) -> NewWidget<Self> {
        let lines = options.iter().map(|_| text::Line::default()).collect();
        NewWidget::new(Segmented { options, selected, hovered: None, lines, rects: Vec::new(), stretch, shown: selected as f64 })
    }

    fn style() -> text::Style {
        text::Style::ui(size::SIZE_20)
    }

    fn naturals(&self, fcx: &mut masonry::parley::FontContext, lcx: &mut masonry::parley::LayoutContext<masonry::core::BrushIndex>) -> Vec<f64> {
        self.options.iter().map(|o| text::measure_with(fcx, lcx, o, Self::style()).ceil() + size::PADDING_XS * 2.0).collect()
    }

    fn total(widths: &[f64]) -> f64 {
        widths.iter().sum::<f64>() + SEGMENT_GAP * widths.len().saturating_sub(1) as f64 + SEGMENT_PAD * 2.0
    }

    fn segment_at(&self, p: Point) -> Option<usize> {
        self.rects.iter().position(|r| r.inflate(SEGMENT_GAP / 2.0, SEGMENT_PAD).contains(p))
    }

    fn choose(&mut self, ctx: &mut EventCtx<'_>, index: usize) {
        if index != self.selected && index < self.options.len() {
            self.selected = index;
            ctx.request_anim_frame();
            ctx.request_paint_only();
            ctx.request_accessibility_update();
            ctx.submit_action::<Changed>(Changed::Choice(index));
        }
    }

    fn indicator(&self) -> Option<Rect> {
        let last = self.rects.len().checked_sub(1)?;
        let at = self.shown.clamp(0.0, last as f64);
        let (i, t) = (at.floor() as usize, at.fract());
        let (a, b) = (self.rects[i], self.rects[(i + 1).min(last)]);
        let mix = |x: f64, y: f64| x + (y - x) * t;
        Some(Rect::new(mix(a.x0, b.x0), a.y0, mix(a.x1, b.x1), a.y1))
    }
}

impl Widget for Segmented {
    type Action = Changed;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Move(update) => {
                let hovered = self.segment_at(ctx.local_position(update.current.position));
                if hovered != self.hovered {
                    self.hovered = hovered;
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Leave(_) => {
                self.hovered = None;
                ctx.request_paint_only();
            }
            PointerEvent::Down(e) if e.button.is_none_or(|b| b == PointerButton::Primary) => {
                if let Some(index) = self.segment_at(ctx.local_position(e.state.position)) {
                    self.choose(ctx, index);
                }
                ctx.set_handled();
            }
            _ => {}
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &masonry::core::TextEvent) {
        use masonry::core::keyboard::{Key, KeyState, NamedKey};
        let masonry::core::TextEvent::Keyboard(k) = event else { return };
        if k.state != KeyState::Down {
            return;
        }
        let last = self.options.len().saturating_sub(1);
        let index = match &k.key {
            Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowUp) => self.selected.saturating_sub(1),
            Key::Named(NamedKey::ArrowRight | NamedKey::ArrowDown) => (self.selected + 1).min(last),
            Key::Named(NamedKey::Home) => 0,
            Key::Named(NamedKey::End) => last,
            _ => return,
        };
        self.choose(ctx, index);
        ctx.set_handled();
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let goal = self.selected as f64;
        let step = interval as f64 / 1e9 / SEGMENT_SECONDS;
        self.shown = if self.shown < goal { (self.shown + step).min(goal) } else { (self.shown - step).max(goal) };
        ctx.request_paint_only();
        if self.shown != goal {
            ctx.request_anim_frame();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FontsChanged => {
                self.lines.iter_mut().for_each(text::Line::clear);
                ctx.request_layout();
            }
            Update::FocusChanged(_) => ctx.request_paint_only(),
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match axis {
            Axis::Vertical => Length::px(SEGMENT_HEIGHT + SEGMENT_PAD * 2.0),
            Axis::Horizontal => {
                let (fcx, lcx) = ctx.text_contexts();
                let natural = Self::total(&self.naturals(fcx, lcx));
                match len_req {
                    LenReq::FitContent(space) if self.stretch => Length::px(space.get().max(natural)),
                    _ => Length::px(natural),
                }
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let (fcx, lcx) = ctx.text_contexts();
        let natural = self.naturals(fcx, lcx);
        let extra = if self.stretch { ((size.width - Self::total(&natural)) / natural.len().max(1) as f64).max(0.0) } else { 0.0 };
        let mut left = SEGMENT_PAD;
        self.rects = natural
            .iter()
            .map(|w| {
                let rect = Rect::new(left, SEGMENT_PAD, left + w + extra, size.height - SEGMENT_PAD);
                left = rect.x1 + SEGMENT_GAP;
                rect
            })
            .collect();
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let shape = RoundedRect::from_rect(bounds, size::RADIUS_XL);
        painter.fill(shape, t.bg_surface).draw();
        ring(painter, shape, t.border_default);
        if let Some(indicator) = self.indicator() {
            painter.fill(RoundedRect::from_rect(indicator, size::RADIUS_LG), t.bg_accent).draw();
        }
        let style = Self::style();
        for (i, option) in self.options.iter().enumerate() {
            let Some(rect) = self.rects.get(i).copied() else { break };
            let color = if (self.shown - i as f64).abs() < 0.5 {
                t.text_on_accent
            } else if self.hovered == Some(i) {
                t.text_secondary
            } else {
                t.text_tertiary
            };
            let layout = self.lines[i].layout(ctx, option, rect.width(), style);
            let x = rect.x0 + (rect.width() - f64::from(layout.width())) / 2.0;
            let y = rect.center().y - style.line_height / 2.0;
            render_text(painter, Affine::translate((x, y)), layout, &[color.into()], true);
        }
        if ctx.is_focus_target() {
            painter.stroke(&RoundedRect::from_rect(bounds.inflate(1.0, 1.0), size::RADIUS_XL + 1.0), &Stroke::new(size::BORDER_1), t.text_accent).draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::RadioGroup
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        if let Some(option) = self.options.get(self.selected) {
            node.set_value(option.clone());
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub struct DropdownButton {
    label: String,
    line: text::Line,
    press: Press,
    open: bool,
}

impl DropdownButton {
    pub fn new(label: &str) -> NewWidget<Self> {
        NewWidget::new(DropdownButton { label: label.to_owned(), line: text::Line::default(), press: Press::default(), open: false })
    }

    pub fn set_open(this: &mut WidgetMut<'_, Self>, open: bool) {
        this.widget.open = open;
        this.ctx.request_paint_only();
    }

    pub fn set_label(this: &mut WidgetMut<'_, Self>, label: &str) {
        this.widget.label = label.to_owned();
        this.ctx.request_paint_only();
        this.ctx.request_accessibility_update();
    }
}

impl Widget for DropdownButton {
    type Action = Changed;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.press.pointer(ctx, event) {
            let origin = ctx.to_window(Point::ZERO);
            let rect = Rect::from_origin_size(origin, ctx.content_box().size());
            ctx.submit_action::<Changed>(Changed::OpenMenu(rect));
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

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(match axis {
            Axis::Horizontal => size::INPUT_WIDTH,
            Axis::Vertical => size::INPUT_PADDING_Y * 2.0 + size::INPUT_CONTENT_HEIGHT,
        })
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let active = ctx.is_hovered() || self.press.down || self.open;
        let bg = if active { t.bg_surface_active } else { t.bg_surface_overlay };
        painter.fill(RoundedRect::from_rect(bounds, size::RADIUS_XL), bg).draw();
        let icon = size::SIZE_20;
        let text_width = bounds.width() - size::INPUT_PADDING_X * 2.0 - icon - size::INPUT_GAP;
        let style = text::Style::ui(size::SIZE_20);
        let layout = self.line.layout(ctx, &self.label, text_width.max(0.0), style);
        let origin = Affine::translate((bounds.x0 + size::INPUT_PADDING_X, bounds.y0 + size::INPUT_PADDING_Y));
        render_text(painter, origin, layout, &[t.text_primary.into()], true);
        let chevron = Point::new(bounds.x1 - size::INPUT_PADDING_X - icon, bounds.center().y - icon / 2.0);
        let glyph = if self.open { icons::CHEVRON_UP } else { icons::CHEVRON_DOWN };
        glyph.draw(painter, chevron, icon, 1.2, t.icon_faint);
    }

    fn accessibility_role(&self) -> Role {
        Role::ComboBox
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_value(self.label.clone());
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub struct StepButton {
    icon: Icon,
    step: f64,
    enabled: bool,
    press: Press,
}

impl StepButton {
    pub fn new(icon: Icon, step: f64, enabled: bool) -> NewWidget<Self> {
        NewWidget::new(StepButton { icon, step, enabled, press: Press::default() })
    }

    pub fn set_enabled(this: &mut WidgetMut<'_, Self>, enabled: bool) {
        if this.widget.enabled != enabled {
            this.widget.enabled = enabled;
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }
}

impl Widget for StepButton {
    type Action = Changed;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.enabled && self.press.pointer(ctx, event) {
            ctx.submit_action::<Changed>(Changed::Step(self.step));
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::HoveredChanged(_)) {
            ctx.request_paint_only();
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, _axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(size::SIZE_20)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let hovered = self.enabled && (ctx.is_hovered() || self.press.down);
        if hovered {
            painter.fill(RoundedRect::from_rect(bounds, size::ROUNDED_LG), t.icon_button_bg_hover).draw();
        }
        let color = if hovered { t.icon_button_icon_hover } else { t.icon_button_icon };
        let color = if self.enabled { color } else { color.multiply_alpha(0.35) };
        let glyph = size::SIZE_16;
        self.icon.draw(painter, bounds.center() - Vec2::new(glyph / 2.0, glyph / 2.0), glyph, 1.6, color);
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        if !self.enabled {
            node.set_disabled();
        }
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub struct Swatch {
    color: Option<masonry::peniko::Color>,
}

impl Swatch {
    pub fn new(color: Option<masonry::peniko::Color>) -> NewWidget<Self> {
        NewWidget::new(Swatch { color })
    }
}

impl Widget for Swatch {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, _axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(size::SIZE_20)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let shape = RoundedRect::from_rect(ctx.content_box(), size::RADIUS_MD);
        if let Some(color) = self.color {
            painter.fill(shape, color).draw();
        }
        ring(painter, shape, t.border_default);
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

#[derive(Debug)]
pub struct Committed;

pub struct InputBox<W: Widget> {
    child: WidgetPod<W>,
    width: f64,
}

impl<W: Widget> InputBox<W> {
    pub fn new(child: NewWidget<W>, width: f64) -> NewWidget<Self> {
        NewWidget::new(InputBox { child: child.to_pod(), width })
    }
}

impl<W: Widget> Widget for InputBox<W> {
    type Action = Committed;

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if let Update::ChildFocusChanged(focused) = event {
            ctx.request_paint_only();
            if !focused {
                ctx.submit_action::<Committed>(Committed);
            }
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        match axis {
            Axis::Horizontal => Length::px(self.width),
            Axis::Vertical => {
                let inner = self.width - size::INPUT_PADDING_X * 2.0;
                let context = LayoutSize::maybe(Axis::Horizontal, Some(Length::px(inner)));
                let height = ctx.compute_length(&mut self.child, LenReq::MaxContent.into(), context, Axis::Vertical, Some(Length::px(inner)));
                Length::px(height.get().max(size::INPUT_CONTENT_HEIGHT) + size::INPUT_PADDING_Y * 2.0)
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let inner = Size::new(size_.width - size::INPUT_PADDING_X * 2.0, size_.height - size::INPUT_PADDING_Y * 2.0);
        let child = ctx.compute_size(&mut self.child, SizeDef::new(LenDef::FitContent(Length::px(inner.width)), LenDef::MaxContent), inner.into());
        let child = Size::new(inner.width, child.height);
        ctx.run_layout(&mut self.child, child);
        let y = (size_.height - child.height) / 2.0;
        ctx.place_child(&mut self.child, Point::new(size::INPUT_PADDING_X, y));
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let shape = RoundedRect::from_rect(ctx.content_box(), size::RADIUS_XL);
        painter.fill(shape, t.bg_surface).draw();
        if ctx.has_focus_target() {
            let outer = RoundedRect::from_rect(ctx.content_box().inflate(2.5, 2.5), size::RADIUS_XL + 2.5);
            painter.stroke(&outer, &Stroke::new(3.0), t.border_focus_ring).draw();
            ring(painter, shape, t.border_focus);
        } else {
            ring(painter, shape, t.border_default);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}

fn ring(painter: &mut Painter<'_>, shape: RoundedRect, color: masonry::peniko::Color) {
    let outer = RoundedRect::from_rect(shape.rect().inflate(0.5, 0.5), shape.radii().top_left + 0.5);
    painter.stroke(&outer, &Stroke::new(1.0), color).draw();
}
