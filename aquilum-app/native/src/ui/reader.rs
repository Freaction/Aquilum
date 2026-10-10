use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::{Code, Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PointerScrollEvent, PropertiesMut, PropertiesRef,
    RegisterCtx, ScrollDelta, TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Line as Segment, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};

use super::icons;
use super::menu::box_shadow;
use super::text::{Line, Style};
use super::theme;
use super::tokens::{number, size};
use super::widgets::{ButtonSize, IconButton, TextButton, Variant};
use crate::i18n::t;
use crate::reader::page::{Item, Settings};
use crate::reader::session::{Session, Spot};

const POPOVER_WIDTH: f64 = 352.0;
const PROGRESS: f64 = 3.0;
const WHEEL_STEP: f64 = 120.0;
const TOOLBAR_WIDTH: f64 = 576.0;
const MARK_SCALE: f64 = 0.6;

#[derive(Debug)]
pub enum ReaderAction {
    Close,
    Settings,
    Moved { cfi: String, fraction: f64 },
    QuoteRef(String),
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Prev,
    Next,
}

pub struct ReaderView {
    session: Session,
    settings: Settings,
    title: String,
    total: u32,
    fraction: f64,
    report: bool,
    wheel: f64,
    settings_button: WidgetPod<IconButton>,
    close: WidgetPod<IconButton>,
    popover: Option<WidgetPod<dyn Widget>>,
    title_line: Line,
    pages_line: Line,
    header: f64,
    body: Rect,
    popover_rect: Rect,
    hover: Option<Option<Side>>,
    selection: Option<(Spot, Spot)>,
    selecting: bool,
    quotes: Vec<(String, String)>,
    marks: Option<(usize, Vec<(Spot, Spot, String, String)>)>,
    labels: Vec<(Rect, String)>,
    quote_button: Option<WidgetPod<TextButton>>,
    toolbar: Rect,
    preview: Rect,
    preview_line: Line,
    mark_lines: Vec<Line>,
}

pub struct ReaderIds {
    pub settings: WidgetId,
    pub close: WidgetId,
    pub quote: Option<WidgetId>,
}

fn nav_width(width: f64) -> f64 {
    (width * 0.08).clamp(40.0, 72.0)
}

impl ReaderView {
    pub fn new(session: Session, settings: Settings, title: String, total: u32, can_quote: bool) -> (NewWidget<Self>, ReaderIds) {
        let settings_button = NewWidget::new(IconButton::new(icons::SETTINGS_2, t("reader.settings")));
        let close = NewWidget::new(IconButton::new(icons::X, t("reader.close")));
        let quote_button = can_quote.then(|| NewWidget::new(TextButton::new(t("reader.quote"), Variant::Primary).size(ButtonSize::S)));
        let ids = ReaderIds { settings: settings_button.id(), close: close.id(), quote: quote_button.as_ref().map(NewWidget::id) };
        let view = ReaderView {
            session,
            settings,
            title,
            total: total.max(1),
            fraction: 0.0,
            report: true,
            wheel: 0.0,
            settings_button: settings_button.to_pod(),
            close: close.to_pod(),
            popover: None,
            title_line: Line::default(),
            pages_line: Line::default(),
            header: size::SPACE_48,
            body: Rect::ZERO,
            popover_rect: Rect::ZERO,
            hover: None,
            selection: None,
            selecting: false,
            quotes: Vec::new(),
            marks: None,
            labels: Vec::new(),
            quote_button: quote_button.map(NewWidget::to_pod),
            toolbar: Rect::ZERO,
            preview: Rect::ZERO,
            preview_line: Line::default(),
            mark_lines: Vec::new(),
        };
        (NewWidget::new(view), ids)
    }

    pub fn set_settings(this: &mut WidgetMut<'_, Self>, settings: Settings) {
        if this.widget.settings != settings {
            this.widget.settings = settings;
            this.widget.report = true;
            this.ctx.request_layout();
        }
    }

    pub fn set_popover(this: &mut WidgetMut<'_, Self>, popover: Option<NewWidget<dyn Widget>>) {
        if let Some(old) = this.widget.popover.take() {
            this.ctx.remove_child(old);
        }
        this.widget.popover = popover.map(NewWidget::to_pod);
        this.ctx.children_changed();
        this.ctx.request_layout();
    }

    pub fn set_quotes(this: &mut WidgetMut<'_, Self>, quotes: Vec<(String, String)>) {
        this.widget.quotes = quotes;
        this.widget.marks = None;
        this.ctx.request_paint_only();
    }

    pub fn clear_selection(this: &mut WidgetMut<'_, Self>) {
        if this.widget.selection.take().is_some() {
            this.ctx.request_layout();
        }
    }

    pub fn quote(&self) -> Option<(String, String)> {
        let (from, to) = self.ordered()?;
        let text = self.session.text(from, to);
        let cfi = self.session.range_cfi(from, to)?;
        (!text.is_empty()).then_some((text, cfi))
    }

    fn ordered(&self) -> Option<(Spot, Spot)> {
        let (a, b) = self.selection?;
        (a != b).then(|| if a < b { (a, b) } else { (b, a) })
    }

    fn marks(&mut self) -> &[(Spot, Spot, String, String)] {
        let section = self.session.section();
        if self.marks.as_ref().is_none_or(|(s, _)| *s != section) {
            let resolved = self.quotes.iter().filter_map(|(cfi, label)| self.session.resolve(cfi).map(|(a, b)| (a, b, cfi.clone(), label.clone()))).collect();
            self.marks = Some((section, resolved));
        }
        &self.marks.as_ref().expect("метки посчитаны выше").1
    }

    #[cfg(test)]
    pub fn label_rects(&self) -> Vec<Rect> {
        self.labels.iter().map(|(r, _)| *r).collect()
    }

    pub fn has_popover(&self) -> bool {
        self.popover.is_some()
    }

    fn current(&self) -> u32 {
        ((self.fraction.clamp(0.0, 1.0) * f64::from(self.total)).round() as u32).min(self.total)
    }

    fn moved(&mut self, ctx: &mut EventCtx<'_>, changed: bool) {
        if !changed {
            return;
        }
        self.report = true;
        if self.selection.take().is_some() {
            ctx.request_layout();
        }
        if self.session.needs_layout() {
            ctx.request_layout();
        } else {
            self.emit(ctx);
            ctx.request_paint_only();
        }
    }

    fn emit(&mut self, ctx: &mut impl Submit) {
        if !std::mem::take(&mut self.report) {
            return;
        }
        if let Some((cfi, fraction)) = self.session.location() {
            self.fraction = fraction;
            ctx.submit(ReaderAction::Moved { cfi, fraction });
        }
    }

    fn side(&self, p: Point) -> Option<Side> {
        if !self.body.contains(p) {
            return None;
        }
        let nav = nav_width(self.body.width());
        if p.x < self.body.x0 + nav {
            Some(Side::Prev)
        } else if p.x > self.body.x1 - nav {
            Some(Side::Next)
        } else {
            None
        }
    }

    fn turn(&mut self, ctx: &mut EventCtx<'_>, side: Side) {
        let changed = match side {
            Side::Prev => self.session.prev(),
            Side::Next => self.session.next(),
        };
        self.moved(ctx, changed);
    }

    fn line_height() -> f64 {
        (f64::from(theme::ui_font_settings().size) * number::FONT_LINE_HEIGHT_NORMAL).round()
    }
}

trait Submit {
    fn submit(&mut self, action: ReaderAction);
}

impl Submit for EventCtx<'_> {
    fn submit(&mut self, action: ReaderAction) {
        self.submit_action::<ReaderAction>(action);
    }
}

impl Submit for LayoutCtx<'_> {
    fn submit(&mut self, action: ReaderAction) {
        self.submit_action::<ReaderAction>(action);
    }
}

impl Widget for ReaderView {
    type Action = ReaderAction;

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        let TextEvent::Keyboard(k) = event else { return };
        if k.state != KeyState::Down {
            return;
        }
        if k.code == Code::KeyC && (k.modifiers.ctrl() || k.modifiers.meta()) && self.popover.is_none() {
            if let Some((from, to)) = self.ordered() {
                let text = self.session.text(from, to);
                if let Err(error) = arboard::Clipboard::new().and_then(|mut c| c.set_text(text)) {
                    eprintln!("не скопировано: {error}");
                }
                ctx.set_handled();
            }
            return;
        }
        match &k.key {
            Key::Named(NamedKey::Escape) => {
                ctx.submit_action::<ReaderAction>(if self.popover.is_some() { ReaderAction::Settings } else { ReaderAction::Close });
            }
            _ if self.popover.is_some() => return,
            Key::Named(NamedKey::ArrowRight | NamedKey::PageDown) => self.turn(ctx, Side::Next),
            Key::Character(c) if c == " " => self.turn(ctx, Side::Next),
            Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) => self.turn(ctx, Side::Prev),
            _ => return,
        }
        ctx.set_handled();
    }

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Move(update) => {
                let p = ctx.local_position(update.current.position);
                if self.selecting {
                    if let Some((_, focus)) = self.session.hit(p - self.body.origin().to_vec2(), false)
                        && let Some((anchor, old)) = self.selection
                    {
                        if focus != old {
                            self.selection = Some((anchor, focus));
                            ctx.request_paint_only();
                        }
                    }
                    return;
                }
                let hover = self.body.contains(p).then(|| self.side(p));
                if hover != self.hover {
                    self.hover = hover;
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Leave(_) => {
                if self.hover.take().is_some() {
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Down(down) => {
                let p = ctx.local_position(down.state.position);
                if self.popover.is_some() {
                    if self.body.contains(p) && !self.popover_rect.contains(p) {
                        ctx.submit_action::<ReaderAction>(ReaderAction::Settings);
                    }
                    return;
                }
                if self.quote_button.is_some() && self.selection.is_some() && self.toolbar.contains(p) {
                    return;
                }
                if let Some((_, cfi)) = self.labels.iter().find(|(r, _)| r.contains(p)) {
                    ctx.submit_action::<ReaderAction>(ReaderAction::QuoteRef(cfi.clone()));
                    return;
                }
                if let Some(side) = self.side(p) {
                    self.turn(ctx, side);
                    return;
                }
                if !self.body.contains(p) {
                    return;
                }
                let had = self.selection.is_some();
                self.selection = self.session.hit(p - self.body.origin().to_vec2(), down.state.count == 2);
                self.selecting = true;
                ctx.capture_pointer();
                if had != self.selection.is_some() {
                    ctx.request_layout();
                }
                ctx.request_paint_only();
            }
            PointerEvent::Up(_) => {
                if !std::mem::take(&mut self.selecting) {
                    return;
                }
                if self.ordered().is_none() {
                    self.selection = None;
                }
                ctx.request_layout();
            }
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                if self.popover.is_some() {
                    return;
                }
                let (dx, dy) = match delta {
                    ScrollDelta::PixelDelta(d) => (d.x, d.y),
                    ScrollDelta::LineDelta(x, y) => (f64::from(*x) * 40.0, f64::from(*y) * 40.0),
                    _ => (0.0, 0.0),
                };
                if self.settings.scrolled {
                    let changed = self.session.scroll_by(-dy);
                    self.moved(ctx, changed);
                    return;
                }
                self.wheel += if dx.abs() > dy.abs() { -dx } else { -dy };
                if self.wheel.abs() >= WHEEL_STEP {
                    let side = if self.wheel > 0.0 { Side::Next } else { Side::Prev };
                    self.wheel = 0.0;
                    self.turn(ctx, side);
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::FontsChanged) {
            self.title_line.clear();
            self.pages_line.clear();
            ctx.request_layout();
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.settings_button);
        ctx.register_child(&mut self.close);
        if let Some(popover) = &mut self.popover {
            ctx.register_child(popover);
        }
        if let Some(button) = &mut self.quote_button {
            ctx.register_child(button);
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let line = Self::line_height();
        self.header = (size::SPACE_8 * 2.0 + line * 2.0 + size::SPACE_2).max(size::SPACE_48);
        let button = Size::new(size::SIZE_32, size::SIZE_32);
        let y = ((self.header - size::SIZE_32) / 2.0).round();
        let close_x = size.width - size::SPACE_12 - size::SIZE_32;
        ctx.run_layout(&mut self.close, button);
        ctx.place_child(&mut self.close, Point::new(close_x, y));
        ctx.run_layout(&mut self.settings_button, button);
        ctx.place_child(&mut self.settings_button, Point::new(close_x - size::SPACE_4 - size::SIZE_32, y));
        self.body = Rect::new(0.0, self.header + size::BORDER_1, size.width, size.height - PROGRESS - size::BORDER_1);
        let (fcx, lcx) = ctx.text_contexts();
        self.session.ensure(self.body.size(), &self.settings, fcx, lcx);
        if let Some(popover) = &mut self.popover {
            let width = POPOVER_WIDTH.min(size.width - size::SPACE_24);
            let auto = SizeDef::new(LenDef::FitContent(Length::px(width)), LenDef::MaxContent);
            let measured = ctx.compute_size(popover, auto, LayoutSize::maybe(Axis::Horizontal, Some(Length::px(width))));
            ctx.run_layout(popover, Size::new(width, measured.height));
            let origin = Point::new(size.width - size::SPACE_12 - width, self.header + size::GAP_XS);
            ctx.place_child(popover, origin);
            self.popover_rect = Rect::from_origin_size(origin, Size::new(width, measured.height));
        }
        let selected = self.ordered().is_some() && !self.selecting;
        if let Some(button) = &mut self.quote_button {
            ctx.set_stashed(button, !selected);
            if selected {
                let width = TOOLBAR_WIDTH.min(size.width - size::SPACE_24);
                let auto = SizeDef::new(LenDef::MaxContent, LenDef::MaxContent);
                let measured = ctx.compute_size(button, auto, LayoutSize::maybe(Axis::Horizontal, Some(Length::px(width))));
                ctx.run_layout(button, measured);
                let height = measured.height + size::SPACE_8 * 2.0;
                let left = ((size.width - width) / 2.0).round();
                self.toolbar = Rect::new(left, self.body.y1 - size::SPACE_16 - height, left + width, self.body.y1 - size::SPACE_16);
                let button_x = self.toolbar.x1 - size::SPACE_12 - measured.width;
                ctx.place_child(button, Point::new(button_x, self.toolbar.y0 + size::SPACE_8));
                self.preview = Rect::new(self.toolbar.x0 + size::SPACE_12, self.toolbar.y0, button_x - size::GAP_MD, self.toolbar.y1);
            }
        }
        self.emit(ctx);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let window = ctx.content_box();
        painter.fill(window, tm.bg_canvas).draw();
        let header = Rect::new(0.0, 0.0, window.width(), self.header);
        painter.fill(header, tm.bg_surface).draw();
        let border = self.header + size::BORDER_1 / 2.0;
        painter.stroke(&Segment::new((0.0, border), (window.width(), border)), &Stroke::new(size::BORDER_1), tm.border_subtle).draw();

        let line = Self::line_height();
        let side = size::SPACE_12 + size::SIZE_32 * 2.0 + size::SPACE_4 + size::SPACE_12;
        let inner = (window.width() - side * 2.0).max(1.0);
        let top = ((self.header - line * 2.0 - size::SPACE_2) / 2.0).round();
        let title = if self.title.is_empty() { t("reader.book") } else { self.title.clone() };
        let layout = self.title_line.layout(ctx, &title, inner, Style::ui(line));
        render_text(painter, Affine::translate(((window.width() - f64::from(layout.width())) / 2.0, top)), layout, &[tm.text_primary.into()], true);
        let pages = format!("{}/{}", self.current(), self.total);
        let layout = self.pages_line.layout(ctx, &pages, inner, Style::ui(line));
        render_text(painter, Affine::translate(((window.width() - f64::from(layout.width())) / 2.0, top + line + size::SPACE_2)), layout, &[tm.text_secondary.into()], true);

        let body = self.body;
        let offset = body.origin().to_vec2();
        let marks: Vec<(Vec<Rect>, String, String)> = self.marks().to_vec().into_iter().map(|(a, b, cfi, label)| (self.session.rects(a, b), cfi, label)).filter(|(r, ..)| !r.is_empty()).collect();
        let selection = self.ordered().map(|(a, b)| self.session.rects(a, b)).unwrap_or_default();
        painter.with_fill_clip(body, |painter| {
            for (rects, ..) in &marks {
                for r in rects {
                    painter.fill(*r + offset, tm.text_accent.multiply_alpha(0.3)).draw();
                }
            }
            for r in &selection {
                painter.fill(*r + offset, tm.selection_bg).draw();
            }
        });
        let palette = [tm.text_primary.into(), tm.text_secondary.into()];
        let items = self.session.items();
        painter.with_fill_clip(body, |painter| {
            for (rect, y0, column) in self.session.visible() {
                for piece in &column.pieces {
                    let band = Rect::new(rect.x0 - size::SPACE_24, body.y0 + y0 + piece.y, rect.x1 + size::SPACE_24, body.y0 + y0 + piece.y + (piece.to - piece.from));
                    if band.y1 < body.y0 || band.y0 > body.y1 {
                        continue;
                    }
                    let origin = Vec2::new(rect.x0, band.y0 - piece.from);
                    match &items[piece.item] {
                        Item::Text { layout, x, .. } => {
                            let at = Affine::translate(origin + Vec2::new(*x, 0.0));
                            let whole = piece.from <= 0.5 && piece.to >= f64::from(layout.height()) - 0.5;
                            if whole {
                                render_text(painter, at, layout, &palette, true);
                            } else {
                                painter.with_fill_clip(band, |painter| render_text(painter, at, layout, &palette, true));
                            }
                        }
                        Item::Image { image: Some(image), size: shown, .. } => {
                            let k = shown.width / f64::from(image.image.width);
                            let x = rect.x0 + ((rect.width() - shown.width) / 2.0).max(0.0);
                            painter.with_fill_clip(band, |painter| painter.draw_image(image, Affine::translate((x, band.y0 - piece.from)) * Affine::scale(k)));
                        }
                        Item::Image { .. } => {}
                        Item::Rule => {
                            let y = band.y0 + 0.5;
                            painter.stroke(&Segment::new((rect.x0, y), (rect.x1, y)), &Stroke::new(size::BORDER_1), tm.border_subtle).draw();
                        }
                    }
                }
            }
        });

        self.labels.clear();
        self.mark_lines.resize_with(marks.len(), Line::default);
        let mark_size = f64::from(self.settings.size) * MARK_SCALE;
        for ((rects, cfi, label), line) in marks.iter().zip(&mut self.mark_lines) {
            let Some(last) = rects.last().map(|r| *r + offset) else { continue };
            let style = Style { size: mark_size as f32, weight: self.settings.weight, line_height: mark_size };
            let layout = line.layout(ctx, label, f64::MAX, style);
            let at = Point::new(last.x1 + f64::from(self.settings.size) * 0.15, last.y0 + (last.height() - f64::from(self.settings.size)) / 2.0);
            render_text(painter, Affine::translate(at.to_vec2()), layout, &[tm.text_accent.into()], true);
            self.labels.push((Rect::from_origin_size(at, Size::new(f64::from(layout.width()), mark_size)).inflate(4.0, 4.0), cfi.clone()));
        }

        if let Some(hover) = self.hover {
            let nav = nav_width(body.width());
            for side in [Side::Prev, Side::Next] {
                let zone = match side {
                    Side::Prev => Rect::new(body.x0, body.y0, body.x0 + nav, body.y1),
                    Side::Next => Rect::new(body.x1 - nav, body.y0, body.x1, body.y1),
                };
                let active = hover == Some(side);
                let color = if active { tm.text_primary } else { tm.text_secondary.multiply_alpha(0.7) };
                let icon = if side == Side::Prev { icons::CHEVRON_LEFT } else { icons::CHEVRON_RIGHT };
                let glyph = size::SIZE_24;
                icon.draw(painter, zone.center() - Vec2::new(glyph / 2.0, glyph / 2.0), glyph, 2.0, color);
            }
        }

        let footer = window.height() - PROGRESS - size::BORDER_1;
        painter.fill(Rect::new(0.0, footer, window.width(), window.height()), tm.bg_surface).draw();
        painter.stroke(&Segment::new((0.0, footer + 0.5), (window.width(), footer + 0.5)), &Stroke::new(size::BORDER_1), tm.border_subtle).draw();
        let bar = Rect::new(0.0, window.height() - PROGRESS, window.width(), window.height());
        painter.fill(bar, tm.bg_surface_subtle).draw();
        let share = f64::from(self.current()) / f64::from(self.total);
        painter.fill(Rect::new(0.0, bar.y0, bar.width() * share, bar.y1), tm.text_primary.multiply_alpha(0.45)).draw();

        if self.quote_button.is_some() && self.ordered().is_some() && !self.selecting {
            let shape = RoundedRect::from_rect(self.toolbar, size::RADIUS_XL);
            box_shadow(painter, shape, tm.elevation_md);
            painter.fill(shape, tm.bg_surface).draw();
            painter.stroke(&RoundedRect::from_rect(self.toolbar.inflate(-0.5, -0.5), size::RADIUS_XL - 0.5), &Stroke::new(size::BORDER_1), tm.border_subtle).draw();
            let (from, to) = self.ordered().expect("выделение проверено выше");
            let text = self.session.text(from, to);
            let line = Self::line_height();
            let layout = self.preview_line.layout(ctx, &text, self.preview.width().max(1.0), Style::ui(line));
            render_text(painter, Affine::translate((self.preview.x0, self.preview.y0 + ((self.preview.height() - line) / 2.0).round())), layout, &[tm.text_secondary.into()], true);
        }

        if self.popover.is_some() {
            let shape = RoundedRect::from_rect(self.popover_rect, size::RADIUS_XL);
            box_shadow(painter, shape, tm.elevation_float);
            box_shadow(painter, shape, tm.shadow_ring);
            painter.fill(shape, tm.dialog_bg).draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Dialog
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(if self.title.is_empty() { t("reader.title") } else { self.title.clone() });
        node.set_modal();
    }

    fn children_ids(&self) -> ChildrenIds {
        let mut ids = vec![self.settings_button.id(), self.close.id()];
        ids.extend(self.popover.as_ref().map(WidgetPod::id));
        ids.extend(self.quote_button.as_ref().map(WidgetPod::id));
        ChildrenIds::from_slice(&ids)
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for ReaderView {
    fn capture_pointer_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &PointerEvent) {}
}
