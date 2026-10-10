use std::ops::Range;

use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, BrushIndex, ChildrenIds, ComposeCtx, CursorIcon, EventCtx, Ime, LayoutCtx, MeasureCtx, PaintCtx, PointerButton, PointerButtonEvent,
    PointerEvent, PointerUpdate, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, StyleProperty, StyleSet, TextEvent, Update,
    UpdateCtx, Widget, WidgetMut, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, BezPath, Cap, Circle, Join, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LenReq, Length};
use masonry::parley::{Affinity, BoundingBox, Cursor, Layout, Selection};
use masonry::properties::{ContentColor, SelectionColor};
use masonry::widgets::TextAction;

use super::document::{Document, Fold, Paragraph};
use super::markdown::{self, Block, Callout, Fonts};
use super::history::{Edit, History, Kind};
use crate::ui::theme;
use crate::ui::tokens::size;

const BLINK_MS: u64 = 1000;
const CARET_LINE_SCALE: f64 = 1.0;
const LINE_SCROLL: f64 = 40.0;
const BLINK_TIMEOUT_MS: u64 = 10_000;

mod editing;
mod keys;
mod links;
mod media;
mod marks;
mod outline;
mod suggest;
mod table;

pub struct TextEditor {
    doc: Document,
    history: History,
    styles: StyleSet,
    anchor: usize,
    focus: usize,
    affinity: Affinity,
    column: Option<f64>,
    compose: Option<(String, Option<(usize, usize)>)>,
    width: f64,
    caret_visible: bool,
    pub(super) painted: (f64, f64),
    blink_phase: u64,
    blink_elapsed: u64,
    single_line: bool,
    centered: bool,
    scroll_x: f64,
    focused: bool,
    table: Option<super::table::Session>,
    hovered_table: Option<usize>,
    table_scrolls: std::collections::HashMap<usize, f64>,
    media: media::MediaState,
    videos: crate::video::Owned,
    pointer_hover: bool,
    hovered_link: Option<(usize, usize)>,
    unit: Option<(u8, Range<usize>)>,
    suggest: Option<suggest::Suggest>,
    suggest_shown: Option<crate::ui::editor::resolve::Suggestions>,
}

fn visible(transform: Affine, window: Size) -> (f64, f64) {
    let r = transform.inverse().transform_rect_bbox(window.to_rect());
    (r.y0, r.y1)
}

fn rect(b: BoundingBox, top: f64) -> Rect {
    Rect::new(b.x0, b.y0 + top, b.x1, b.y1 + top)
}

impl TextEditor {
    pub fn new(text: &str) -> Self {
        let mut styles = StyleSet::new(16.0);
        styles.insert(StyleProperty::LineHeight(masonry::parley::LineHeight::FontSizeRelative(1.2)));
        TextEditor {
            doc: Document::new(text),
            history: History::default(),
            styles,
            anchor: 0,
            focus: 0,
            affinity: Affinity::Downstream,
            column: None,
            compose: None,
            width: 0.0,
            caret_visible: true,
            painted: (0.0, 0.0),
            blink_phase: 0,
            blink_elapsed: 0,
            single_line: false,
            centered: false,
            scroll_x: 0.0,
            focused: false,
            table: None,
            hovered_table: None,
            table_scrolls: std::collections::HashMap::new(),
            media: media::MediaState::default(),
            videos: crate::video::Owned::default(),
            pointer_hover: false,
            hovered_link: None,
            unit: None,
            suggest: None,
            suggest_shown: None,
        }
    }

    pub fn single_line(text: &str) -> Self {
        let text = text.replace(['\r', '\n'], " ");
        TextEditor { single_line: true, ..TextEditor::new(&text) }
    }

    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    pub fn markdown(mut self) -> Self {
        let size = self.styles.inner().values().find_map(|p| if let StyleProperty::FontSize(s) = p { Some(*s) } else { None }).unwrap_or(16.0);
        self.doc.set_markdown(Some(Fonts { size, mono: theme::editor_font("iA Writer Mono"), ch: 0.0 }));
        self
    }

    pub fn with_resolver(mut self, resolver: Option<std::sync::Arc<dyn super::resolve::Resolver>>) -> Self {
        self.doc.resolver = resolver;
        self
    }

    pub fn refresh_embeds(this: &mut WidgetMut<'_, Self>) {
        let id = this.ctx.widget_id();
        this.widget.poll_suggest(id);
        this.ctx.request_layout();
        this.ctx.request_render();
    }

    pub fn with_style(mut self, property: impl Into<StyleProperty>) -> Self {
        self.styles.insert(property.into());
        self
    }

    pub fn text(&self) -> &str {
        self.doc.text()
    }

    pub fn reset_text(this: &mut WidgetMut<'_, Self>, text: &str) {
        let widget = &mut *this.widget;
        widget.videos.stop();
        widget.doc.set_text(text);
        widget.history.clear();
        widget.table_scrolls.clear();
        widget.compose = None;
        let end = widget.doc.len();
        widget.anchor = text.floor_char_boundary(widget.anchor.min(end));
        widget.focus = text.floor_char_boundary(widget.focus.min(end));
        this.ctx.request_layout();
        this.ctx.request_render();
    }

    pub fn select_byte_range(this: &mut WidgetMut<'_, Self>, start: usize, end: usize) {
        let widget = &mut *this.widget;
        let len = widget.doc.len();
        widget.anchor = widget.doc.text().floor_char_boundary(start.min(len));
        widget.focus = widget.doc.text().floor_char_boundary(end.min(len));
        widget.column = None;
        this.ctx.request_layout();
        this.ctx.request_render();
    }

    pub fn anchor_at(&self, y: f64) -> Option<(usize, f64)> {
        if !self.laid_out() {
            return None;
        }
        let p = self.doc.paragraph(self.doc.at_y(y.max(0.0)));
        Some((p.start, y - p.top))
    }

    pub fn top_of(&self, offset: usize) -> Option<f64> {
        self.laid_out().then(|| self.doc.paragraph(self.doc.at_offset(offset.min(self.doc.len()))).top)
    }
}

impl Widget for TextEditor {
    type Action = TextAction;

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let (changed, playing) = self.videos.tick();
        if changed {
            ctx.request_post_paint();
        }
        if playing {
            ctx.request_anim_frame();
        }
        if !(ctx.is_window_focused() && ctx.is_focus_target()) {
            return;
        }
        if self.blink_elapsed >= BLINK_TIMEOUT_MS {
            if !self.caret_visible {
                self.caret_visible = true;
                ctx.request_post_paint();
            }
            return;
        }
        let ms = interval / 1_000_000;
        self.blink_elapsed += ms;
        self.blink_phase = (self.blink_phase + ms) % BLINK_MS;
        ctx.request_anim_frame();
        let visible = self.blink_phase < BLINK_MS / 2;
        if visible != self.caret_visible {
            self.caret_visible = visible;
            ctx.request_post_paint();
        }
    }

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.compose.is_some() || !self.laid_out() {
            return;
        }
        match event {
            PointerEvent::Down(PointerButtonEvent { button: None | Some(PointerButton::Primary), state, .. }) => {
                self.close_suggest();
                let point = ctx.local_position(state.position);
                let new_tab = state.modifiers.ctrl() || state.modifiers.meta();
                if self.table_press(ctx, point, state.modifiers.shift(), new_tab)
                    || self.media_press(ctx, point, state.count)
                    || self.card_click(ctx, point, new_tab)
                    || self.toggle_task(ctx, point)
                    || self.open_link(ctx, point, new_tab)
                {
                    return;
                }
                self.select_at(point, state.count, state.modifiers.shift());
                ctx.request_focus();
                ctx.capture_pointer();
                self.restart_blink(ctx);
                self.refresh(ctx);
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                let point = ctx.local_position(current.position);
                self.hover_text(ctx, point);
                if self.media_move(ctx, point) || self.table_move(ctx, point) || !ctx.is_active() || self.table_active() {
                    return;
                }
                self.drag_select(point);
                self.refresh(ctx);
            }
            PointerEvent::Down(PointerButtonEvent { button: Some(PointerButton::Secondary), state, .. }) => {
                let point = ctx.local_position(state.position);
                self.table_context(ctx, point);
            }
            PointerEvent::Up(_) => {
                self.media_release(ctx);
                self.table_release(ctx);
            }
            PointerEvent::Leave(_) => {
                self.media_leave(ctx);
                if self.hovered_link.take().is_some() {
                    ctx.request_render();
                }
                if self.hovered_table.take().is_some() {
                    ctx.request_render();
                }
            }
            PointerEvent::Scroll(scroll) => {
                let point = ctx.local_position(scroll.state.position);
                let (dx, dy) = match scroll.delta {
                    masonry::core::ScrollDelta::PixelDelta(p) => (p.x / ctx.scale_factor(), p.y / ctx.scale_factor()),
                    masonry::core::ScrollDelta::LineDelta(x, y) => (f64::from(x) * LINE_SCROLL, f64::from(y) * LINE_SCROLL),
                    _ => (0.0, 0.0),
                };
                let dx = if scroll.state.modifiers.shift() && dx == 0.0 { dy } else { dx };
                if dx != 0.0 && self.table_scroll(ctx, point, -dx) {
                    ctx.set_handled();
                }
            }
            _ => {}
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        self.restart_blink(ctx);
        match event {
            TextEvent::Keyboard(key) if self.laid_out() => {
                if !self.media_key(ctx, key) && !self.table_key(ctx, key) {
                    self.keyboard(ctx, key);
                }
            }
            TextEvent::Ime(Ime::Commit(text)) if self.table_text(ctx, text) => {}
            TextEvent::Ime(ime) => self.ime(ctx, ime),
            TextEvent::ClipboardPaste(text) if self.table_text(ctx, text) => {}
            TextEvent::ClipboardPaste(text) => {
                self.insert(text, Kind::Other);
                ctx.set_handled();
                self.changed(ctx);
            }
            TextEvent::WindowFocusChange(_) => ctx.request_render(),
            _ => {}
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn accepts_text_input(&self) -> bool {
        true
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FontsChanged => {
                self.doc.invalidate_all();
                ctx.request_layout();
            }
            Update::FocusChanged(focused) => {
                self.focused = *focused;
                if !*focused {
                    self.hide_suggest();
                }
                ctx.request_layout();
                ctx.request_render();
                ctx.request_anim_frame();
            }
            _ => {}
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        match axis {
            Axis::Horizontal => match len_req {
                LenReq::MinContent => Length::ZERO,
                LenReq::MaxContent => Length::px(self.width),
                LenReq::FitContent(space) => space,
            },
            Axis::Vertical => {
                let width = cross.map_or(self.width, Length::get);
                let (fcx, lcx) = ctx.text_contexts();
                self.ensure_layout(fcx, lcx, width);
                Length::px(self.doc.height())
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let (fcx, lcx) = ctx.text_contexts();
        self.ensure_layout(fcx, lcx, size.width);
        self.reveal_caret();
        if self.suggest.is_some() {
            self.publish_suggest(ctx.widget_id());
        }
        if self.single_line {
            ctx.set_clip_path(size.to_rect());
        }
    }

    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {
        let (top, bottom) = visible(ctx.window_transform(), ctx.window_size());
        if top < self.painted.0 || bottom > self.painted.1 {
            ctx.request_paint_only();
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let _zone = aq_trace::zone("редактор: отрисовка текста");
        let (top, bottom) = visible(ctx.window_transform(), ctx.window_size());
        let margin = bottom - top;
        self.painted = (top - margin, bottom + margin);
        let (first, last) = (self.doc.at_y(self.painted.0), self.doc.at_y(self.painted.1));
        let shown = &self.doc.paragraphs()[first..=last];
        let range = self.selection();
        if !range.is_empty() && ctx.is_focus_target() {
            let color = props.get::<SelectionColor>(ctx.property_cache()).color;
            for p in shown.iter().filter(|p| p.fold == Fold::None && p.layout.is_some() && p.start <= range.end && p.end() >= range.start) {
                let layout = p.layout();
                let a = range.start.max(p.start) - p.start;
                let b = range.end.min(p.end()) - p.start;
                let selection = Selection::new(Cursor::from_byte_index(layout, a, Affinity::Downstream), Cursor::from_byte_index(layout, b, Affinity::Upstream));
                for (bounds, _) in selection.geometry(layout) {
                    painter.fill(rect(bounds, 0.0) + p.origin() + self.shift(), color).draw();
                }
                if range.end > p.end() && b == p.len {
                    let end = Cursor::from_byte_index(layout, p.len, Affinity::Upstream).geometry(layout, 1.0);
                    painter.fill(rect(BoundingBox::new(end.x0, end.y0, end.x0 + 8.0, end.y1), 0.0) + p.origin() + self.shift(), color).draw();
                }
            }
        }
        let color = props.get::<ContentColor>(ctx.property_cache()).color;
        let tm = theme::current();
        let palette = markdown::palette(tm, color);
        let markdown = self.doc.markdown.is_some();
        if markdown {
            self.paint_blocks(painter, first, last);
        }
        for p in shown {
            match (p.fold, &p.embed) {
                (Fold::Member, _) => continue,
                (Fold::Head | Fold::Above, Some(embed)) => {
                    let overlay = match &self.table {
                        Some(session) if session.start == p.start => session.overlay(self.hovered_table == Some(p.start)),
                        _ => super::table::Overlay { hovered: self.hovered_table == Some(p.start), scroll_x: self.table_scroll_x(p.start), ..Default::default() },
                    };
                    let media = self.media_overlay(p.start);
                    if media.video.is_none() {
                        let hover = self.hovered_link.filter(|(start, _)| *start == p.start).map(|(_, hit)| hit);
                        embed.view.paint(painter, self.shift().x, p.top, &palette, &overlay, &media, hover);
                    }
                    self.paint_link_hover(painter, p);
                    if p.fold == Fold::Head {
                        continue;
                    }
                }
                _ => {}
            }
            if let Some(layout) = &p.layout {
                if markdown {
                    self.paint_marks(painter, p);
                }
                render_text(painter, Affine::translate(self.shift() + p.origin()), layout, &palette, true);
            }
        }
    }

    fn post_paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if !self.videos.is_empty() && self.laid_out() {
            self.paint_videos(painter);
        }
        if !(ctx.is_focus_target() && ctx.is_window_focused() && self.laid_out()) {
            return;
        }
        if self.table_active() {
            self.paint_table_caret(painter);
            return;
        }
        if !self.caret_visible {
            return;
        }
        painter.fill(RoundedRect::from_rect(self.caret(), size::CARET_RADIUS), theme::current().caret_color).draw();
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        self.media_cursor()
            .or_else(|| self.table_cursor().filter(|_| self.hovered_table.is_some()))
            .or(self.pointer_hover.then_some(CursorIcon::Pointer))
            .unwrap_or(CursorIcon::Text)
    }

    fn accessibility_role(&self) -> Role {
        if self.single_line { Role::TextInput } else { Role::MultilineTextInput }
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_value(self.doc.text());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}


impl TextEditor {
    fn restart_blink(&mut self, ctx: &mut EventCtx<'_>) {
        self.blink_phase = 0;
        self.blink_elapsed = 0;
        self.caret_visible = true;
        ctx.request_anim_frame();
    }
}

