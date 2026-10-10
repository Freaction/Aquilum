use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerButton,
    PointerEvent, PointerScrollEvent, PropertiesMut, PropertiesRef, PropertySet, RegisterCtx, ScrollDelta, Update,
    UpdateCtx, Widget, WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use crate::ui::editor::TextEditor;
use masonry::kurbo::{Affine, Axis, BezPath, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};
use masonry::parley::LineHeight;
use masonry::parley::style::StyleProperty;
use masonry::properties::{CaretColor, ContentColor, SelectionColor};

use uuid::Uuid;

use super::handle::Handle;
use super::icons;
use super::widgets::Press;
use super::tokens::size;
use super::{text, theme};

#[derive(Clone, Debug, PartialEq)]
pub enum TitlebarAction {
    Select(Uuid),
    Close(Uuid),
    Menu(Uuid, Point),
    Reorder(usize, usize),
    Drag,
    ToggleMaximize,
    Minimize,
    CloseWindow,
}

const CLOSE_SIZE: f64 = 24.0;
const DRAG_THRESHOLD: f64 = 4.0;
const EDGE: f64 = 48.0;
const EDGE_STEP: f64 = 14.0;

fn tab_style() -> text::Style {
    text::Style { size: size::TABS_FONT_SIZE as f32, weight: 400.0, line_height: size::TAB_CONTENT_HEIGHT }
}

fn tab_width(ctx: &mut MeasureCtx<'_>, title: &str, icon: bool) -> f64 {
    let title = text::measure(ctx, title, tab_style()).ceil();
    let icon = if icon { size::SIZE_16 + size::TAB_GAP } else { 0.0 };
    (size::SPACE_12 * 2.0 + icon + title + size::SIZE_32).clamp(size::TAB_MIN_WIDTH, size::TAB_MAX_WIDTH)
}

pub struct TabButton {
    id: Uuid,
    title: String,
    active: bool,
    document: bool,
    icon: Option<icons::Icon>,
    line: text::Line,
    close_hovered: bool,
    down: Option<(PointerButton, f64)>,
}

impl TabButton {
    pub fn new(id: Uuid, title: &str, active: bool, document: bool, icon: Option<icons::Icon>) -> NewWidget<Self> {
        NewWidget::new(TabButton {
            id,
            title: title.to_owned(),
            active,
            document,
            icon,
            line: text::Line::default(),
            close_hovered: false,
            down: None,
        })
    }

    fn close_rect(&self, bounds: Rect) -> Rect {
        let x = bounds.x1 - size::SPACE_12 - CLOSE_SIZE;
        let y = bounds.center().y - CLOSE_SIZE / 2.0;
        Rect::new(x, y, x + CLOSE_SIZE, y + CLOSE_SIZE)
    }

    fn close_visible(&self, hovered: bool) -> bool {
        self.active || hovered
    }
}

impl Widget for TabButton {
    type Action = TitlebarAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        let bounds = ctx.content_box();
        match event {
            PointerEvent::Move(update) => {
                let over = self.close_rect(bounds).contains(ctx.local_position(update.current.position));
                if over != self.close_hovered {
                    self.close_hovered = over;
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Leave(_) => {
                self.close_hovered = false;
                ctx.request_paint_only();
            }
            PointerEvent::Down(e) => {
                let at = ctx.local_position(e.state.position);
                match e.button {
                    Some(button @ (PointerButton::Primary | PointerButton::Auxiliary)) => {
                        self.down = Some((button, at.x));
                        ctx.capture_pointer();
                        ctx.request_paint_only();
                    }
                    Some(PointerButton::Secondary) if self.document => {
                        ctx.set_handled();
                        ctx.submit_action::<TitlebarAction>(TitlebarAction::Menu(self.id, ctx.to_window(at)));
                    }
                    _ => {}
                }
            }
            PointerEvent::Up(e) if self.down.is_some_and(|(button, _)| e.button == Some(button)) => {
                let Some((button, start)) = self.down.take() else { return };
                ctx.request_paint_only();
                let at = ctx.local_position(e.state.position);
                if !ctx.is_hovered() || (at.x - start).abs() >= DRAG_THRESHOLD {
                    return;
                }
                let close = button == PointerButton::Auxiliary
                    || (self.close_visible(true) && self.close_rect(bounds).contains(at));
                let action = if close { TitlebarAction::Close(self.id) } else { TitlebarAction::Select(self.id) };
                ctx.submit_action::<TitlebarAction>(action);
            }
            PointerEvent::Cancel(_) => self.down = None,
            _ => {}
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            ctx.submit_action::<TitlebarAction>(TitlebarAction::Select(self.id));
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
            Axis::Vertical => Length::px(size::TABS_HEIGHT),
            Axis::Horizontal => Length::px(tab_width(ctx, &self.title, self.icon.is_some())),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let hovered = ctx.is_hovered();
        paint_tab(painter, bounds, self.active, hovered);
        let style = tab_style();
        let mut left = bounds.x0 + size::SPACE_12;
        if let Some(icon) = self.icon {
            let glyph = size::SIZE_16;
            let color = if self.active { t.icon_primary } else { t.icon_secondary };
            icon.draw(painter, Point::new(left, bounds.center().y - glyph / 2.0), glyph, 1.5, color);
            left += glyph + size::TAB_GAP;
        }
        let width = (bounds.x1 - left - size::SPACE_12 - size::SIZE_32).max(0.0);
        let color = if self.active { t.tab_text_active } else { t.tab_text };
        let layout = self.line.layout(ctx, &self.title, width, style);
        let origin = Point::new(left, bounds.center().y - style.line_height / 2.0);
        render_text(painter, Affine::translate(origin.to_vec2()), layout, &[color.into()], true);

        if self.close_visible(hovered) {
            let rect = self.close_rect(bounds);
            if self.close_hovered {
                painter.fill(RoundedRect::from_rect(rect, size::ROUNDED_LG), t.icon_button_bg_hover).draw();
            }
            let glyph = size::SIZE_16;
            let icon_color = if self.close_hovered { t.icon_button_icon_hover } else { t.icon_button_icon };
            icons::X.draw(painter, rect.center() - Vec2::new(glyph / 2.0, glyph / 2.0), glyph, 1.5, icon_color);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Tab
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.title.clone());
        node.set_selected(self.active);
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

fn paint_tab(painter: &mut Painter<'_>, bounds: Rect, active: bool, hovered: bool) {
    let t = theme::current();
    let body = if active { Rect::new(bounds.x0, bounds.y0, bounds.x1, bounds.y1 + size::BORDER_1) } else { bounds };
    let bg = if active {
        t.tab_bg_active
    } else if hovered {
        t.tab_bg_hover
    } else {
        t.tab_bg
    };
    painter.fill(body, bg).draw();
    painter.fill(Rect::new(bounds.x1 - size::BORDER_1, bounds.y0, bounds.x1, bounds.y1), t.tab_border).draw();
}

pub struct TabRename {
    title: String,
    field: WidgetPod<TextEditor>,
}

impl TabRename {
    pub fn new(title: &str) -> (NewWidget<Self>, Handle<TextEditor>) {
        let t = theme::current();
        let font = theme::ui_font_settings();
        let field = TextEditor::single_line(title)
            .with_style(StyleProperty::FontFamily(theme::ui_font(&font.family)))
            .with_style(StyleProperty::FontSize(size::TABS_FONT_SIZE as f32))
            .with_style(StyleProperty::LineHeight(LineHeight::Absolute(size::TAB_CONTENT_HEIGHT as f32)));
        let field = NewWidget::new(field).with_props(
            PropertySet::new()
                .with(ContentColor::new(t.tab_text_active))
                .with(CaretColor { color: t.text_primary })
                .with(SelectionColor { color: t.selection_bg }),
        );
        let handle = Handle::of(&field);
        (NewWidget::new(TabRename { title: title.to_owned(), field: field.to_pod() }), handle)
    }
}

impl Widget for TabRename {
    type Action = masonry::core::NoAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if matches!(event, PointerEvent::Down(_)) {
            ctx.set_handled();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.field);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        match axis {
            Axis::Vertical => Length::px(size::TABS_HEIGHT),
            Axis::Horizontal => Length::px(tab_width(ctx, &self.title, false)),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let width = (size_.width - size::SPACE_12 * 2.0).max(0.0);
        let auto = SizeDef::new(LenDef::FitContent(Length::px(width)), LenDef::MaxContent);
        let child = ctx.compute_size(&mut self.field, auto, Size::new(width, size_.height).into());
        let child = Size::new(width, child.height);
        ctx.run_layout(&mut self.field, child);
        ctx.place_child(&mut self.field, Point::new(size::SPACE_12, ((size_.height - child.height) / 2.0).round()));
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        paint_tab(painter, ctx.content_box(), true, false);
    }

    fn accessibility_role(&self) -> Role {
        Role::Tab
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.field.id()])
    }
}

fn drag_or_maximize(ctx: &mut EventCtx<'_>, event: &PointerEvent) {
    if let PointerEvent::Down(e) = event
        && e.button == Some(PointerButton::Primary)
    {
        let action = if e.state.count >= 2 { TitlebarAction::ToggleMaximize } else { TitlebarAction::Drag };
        ctx.submit_action::<TitlebarAction>(action);
    }
}

pub fn reorder_layout(boxes: &[(f64, f64)], from: usize, offset: f64) -> (usize, Vec<f64>) {
    let mut shift = vec![0.0; boxes.len()];
    let Some(&(left, width)) = boxes.get(from) else { return (from, shift) };
    let left = left + offset;
    let right = left + width;
    let mut target = from;
    for (index, &(box_left, box_width)) in boxes.iter().enumerate() {
        let middle = box_left + box_width / 2.0;
        if index < from && left < middle {
            shift[index] = width;
            target = target.min(index);
        } else if index > from && right > middle {
            shift[index] = -width;
            target = target.max(index);
        }
    }
    shift[from] = offset;
    (target, shift)
}

struct Reorder {
    from: usize,
    start: f64,
    dragging: bool,
    target: usize,
    shift: Vec<f64>,
}

pub struct TabStrip {
    tabs: Vec<WidgetPod<dyn Widget>>,
    offset: f64,
    content: f64,
    width: f64,
    boxes: Vec<(f64, f64)>,
    reorder: Option<Reorder>,
}

impl TabStrip {
    pub fn new(tabs: Vec<NewWidget<dyn Widget>>) -> NewWidget<Self> {
        NewWidget::new(TabStrip {
            tabs: tabs.into_iter().map(NewWidget::to_pod).collect(),
            offset: 0.0,
            content: 0.0,
            width: 0.0,
            boxes: Vec::new(),
            reorder: None,
        })
    }

    pub fn set_tabs(this: &mut WidgetMut<'_, Self>, tabs: Vec<NewWidget<dyn Widget>>) {
        for old in std::mem::take(&mut this.widget.tabs) {
            this.ctx.remove_child(old);
        }
        this.widget.tabs = tabs.into_iter().map(NewWidget::to_pod).collect();
        this.widget.reorder = None;
        this.ctx.children_changed();
    }

    fn clamp(&mut self) {
        self.offset = self.offset.clamp(0.0, (self.content - self.width).max(0.0));
    }

    fn drag_move(&mut self, ctx: &mut EventCtx<'_>, x: f64) {
        let Some(reorder) = &self.reorder else { return };
        if !reorder.dragging && (x + self.offset - reorder.start).abs() < DRAG_THRESHOLD {
            return;
        }
        if x < EDGE {
            self.offset -= EDGE_STEP;
        } else if x > self.width - EDGE {
            self.offset += EDGE_STEP;
        }
        self.clamp();
        let Some(reorder) = &mut self.reorder else { return };
        reorder.dragging = true;
        let (target, shift) = reorder_layout(&self.boxes, reorder.from, x + self.offset - reorder.start);
        reorder.target = target;
        reorder.shift = shift;
        ctx.request_layout();
    }
}

impl Widget for TabStrip {
    type Action = TitlebarAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                let d = match *delta {
                    ScrollDelta::LineDelta(x, y) => -f64::from(if x != 0.0 { x } else { y }) * 40.0,
                    ScrollDelta::PixelDelta(p) => -(if p.x != 0.0 { p.x } else { p.y }) / ctx.scale_factor(),
                    ScrollDelta::PageDelta(x, y) => -f64::from(if x != 0.0 { x } else { y }) * self.width,
                };
                let before = self.offset;
                self.offset += d;
                self.clamp();
                if self.offset != before {
                    ctx.request_layout();
                    ctx.set_handled();
                }
            }
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                let x = ctx.local_position(e.state.position).x + self.offset;
                match self.boxes.iter().position(|&(left, width)| x >= left && x < left + width) {
                    Some(from) => {
                        self.reorder = Some(Reorder { from, start: x, dragging: false, target: from, shift: Vec::new() });
                    }
                    None => drag_or_maximize(ctx, event),
                }
            }
            PointerEvent::Move(update) if self.reorder.is_some() => {
                let x = ctx.local_position(update.current.position).x;
                self.drag_move(ctx, x);
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => {
                if let Some(reorder) = self.reorder.take()
                    && reorder.dragging
                {
                    if reorder.target != reorder.from && matches!(event, PointerEvent::Up(_)) {
                        ctx.submit_action::<TitlebarAction>(TitlebarAction::Reorder(reorder.from, reorder.target));
                    }
                    ctx.request_layout();
                }
            }
            _ => drag_or_maximize(ctx, event),
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for tab in &mut self.tabs {
            ctx.register_child(tab);
        }
    }

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(size::TABS_HEIGHT),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::ZERO,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let mut widths = Vec::with_capacity(self.tabs.len());
        for tab in &mut self.tabs {
            let context = LayoutSize::maybe(Axis::Vertical, Some(Length::px(size_.height)));
            let width = ctx.compute_length(tab, LenReq::MaxContent.into(), context, Axis::Horizontal, Some(Length::px(size_.height))).get();
            widths.push(width);
        }
        self.content = widths.iter().sum();
        self.width = size_.width;
        self.clamp();
        let shift = self.reorder.as_ref().filter(|r| r.dragging).map(|r| r.shift.clone()).unwrap_or_default();
        self.boxes.clear();
        let mut x = 0.0;
        for (index, (tab, width)) in self.tabs.iter_mut().zip(widths).enumerate() {
            self.boxes.push((x, width));
            ctx.run_layout(tab, Size::new(width, size_.height));
            let moved = shift.get(index).copied().unwrap_or(0.0);
            ctx.place_child(tab, Point::new(x - self.offset + moved, 0.0));
            x += width;
        }
        ctx.set_clip_path(Rect::new(0.0, 0.0, size_.width, size_.height + size::BORDER_1));
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::TabList
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        self.tabs.iter().map(WidgetPod::id).collect()
    }
}

pub struct DragArea;

impl DragArea {
    pub fn new() -> NewWidget<Self> {
        NewWidget::new(DragArea)
    }
}

impl Widget for DragArea {
    type Action = TitlebarAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        drag_or_maximize(ctx, event);
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, _axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match len_req {
            LenReq::FitContent(space) => space,
            _ => Length::ZERO,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum WindowButtonKind {
    Minimize,
    Maximize,
    Close,
}

pub struct WindowButton {
    kind: WindowButtonKind,
    maximized: bool,
    press: Press,
}

impl WindowButton {
    pub fn new(kind: WindowButtonKind, maximized: bool) -> NewWidget<Self> {
        NewWidget::new(WindowButton { kind, maximized, press: Press::default() })
    }

    pub fn set_maximized(this: &mut WidgetMut<'_, Self>, maximized: bool) {
        if this.widget.maximized != maximized {
            this.widget.maximized = maximized;
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }

    fn label(&self) -> String {
        crate::i18n::t(match self.kind {
            WindowButtonKind::Minimize => "window.minimize",
            WindowButtonKind::Maximize => "window.maximize",
            WindowButtonKind::Close => "window.close",
        })
    }

    fn action(&self) -> TitlebarAction {
        match self.kind {
            WindowButtonKind::Minimize => TitlebarAction::Minimize,
            WindowButtonKind::Maximize => TitlebarAction::ToggleMaximize,
            WindowButtonKind::Close => TitlebarAction::CloseWindow,
        }
    }
}

impl Widget for WindowButton {
    type Action = TitlebarAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.press.pointer(ctx, event) {
            ctx.submit_action::<TitlebarAction>(self.action());
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            ctx.submit_action::<TitlebarAction>(self.action());
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::HoveredChanged(_)) {
            ctx.request_paint_only();
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(match axis {
            Axis::Horizontal => size::WINDOW_CONTROL_WIDTH,
            Axis::Vertical => size::TABS_HEIGHT,
        })
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let active = ctx.is_hovered() || self.press.down;
        let close = self.kind == WindowButtonKind::Close;
        let (bg, color) = match (active, close) {
            (true, true) => (Some(if self.press.down { t.window_close_bg_active } else { t.window_close_bg_hover }), t.white),
            (true, false) => (Some(t.icon_button_bg_hover), t.icon_button_icon_hover),
            (false, _) => (None, t.icon_button_icon),
        };
        if let Some(bg) = bg {
            painter.fill(bounds, bg).draw();
        }
        let origin = bounds.center() - Vec2::new(6.0, 6.0);
        let at = |x: f64, y: f64| Point::new(origin.x + x, origin.y + y);
        match (self.kind, self.maximized) {
            (WindowButtonKind::Minimize, _) => {
                painter.fill(Rect::from_points(at(1.0, 6.0), at(11.0, 7.0)), color).draw();
            }
            (WindowButtonKind::Maximize, false) => {
                painter.stroke(Rect::from_points(at(1.5, 1.5), at(10.5, 10.5)), &Stroke::new(1.0), color).draw();
            }
            (WindowButtonKind::Maximize, true) => {
                painter.stroke(Rect::from_points(at(1.5, 3.5), at(8.5, 10.5)), &Stroke::new(1.0), color).draw();
                let mut back = BezPath::new();
                back.move_to(at(3.5, 3.0));
                back.line_to(at(3.5, 1.5));
                back.line_to(at(10.5, 1.5));
                back.line_to(at(10.5, 8.5));
                back.line_to(at(9.0, 8.5));
                painter.stroke(&back, &Stroke::new(1.0), color).draw();
            }
            (WindowButtonKind::Close, _) => {
                let mut cross = BezPath::new();
                cross.move_to(at(1.5, 1.5));
                cross.line_to(at(10.5, 10.5));
                cross.move_to(at(10.5, 1.5));
                cross.line_to(at(1.5, 10.5));
                painter.stroke(&cross, &Stroke::new(1.2), color).draw();
            }
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label());
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub fn window_controls(maximized: bool) -> (NewWidget<impl Widget>, Handle<WindowButton>) {
    let max = WindowButton::new(WindowButtonKind::Maximize, maximized);
    let handle = Handle::of(&max);
    let row = masonry::widgets::Flex::row()
        .with_fixed(WindowButton::new(WindowButtonKind::Minimize, false))
        .with_fixed(max)
        .with_fixed(WindowButton::new(WindowButtonKind::Close, false));
    let underlined = super::widgets::Underlined::new(NewWidget::new(row), || theme::current().tabs_bg, || theme::current().tabs_border);
    (underlined, handle)
}

#[cfg(test)]
mod tests {
    use super::reorder_layout;

    #[test]
    fn dragging_past_middle_moves_target() {
        let boxes = [(0.0, 100.0), (100.0, 100.0), (200.0, 100.0)];
        assert_eq!(reorder_layout(&boxes, 0, 30.0).0, 0);
        let (target, shift) = reorder_layout(&boxes, 0, 60.0);
        assert_eq!(target, 1);
        assert_eq!(shift, [60.0, -100.0, 0.0]);
        assert_eq!(reorder_layout(&boxes, 0, 160.0).0, 2);
        assert_eq!(reorder_layout(&boxes, 2, -160.0).0, 0);
    }
}
