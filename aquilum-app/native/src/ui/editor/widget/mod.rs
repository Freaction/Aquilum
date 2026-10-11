use std::ops::Range;

use masonry::accesskit::{Node, Role};
pub(super) use masonry::core::keyboard::{Key, KeyState, NamedKey};
pub(super) use masonry::core::Ime;
pub(super) use masonry::imaging::Painter;
pub(super) use masonry::kurbo::{Affine, Axis, BezPath, Cap, Circle, Join, Point, Rect, RoundedRect, Size, Stroke, Vec2};
pub(super) use masonry::parley::{Affinity, BoundingBox, Cursor, Layout, Selection};
pub(super) use masonry::core::BrushIndex;
pub(super) use super::markdown;
pub(super) use super::document::{Fold, Paragraph};
pub(super) use super::markdown::{Block, Callout};
pub(super) use super::history::{Edit, Kind};
pub(super) use crate::ui::tokens::size;
use masonry::core::{
    AccessCtx, ChildrenIds, ComposeCtx, CursorIcon, EventCtx, LayoutCtx, MeasureCtx, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, StyleProperty, StyleSet, TextEvent, Update,
    UpdateCtx, Widget, WidgetMut,
};
use masonry::layout::{LenReq, Length};
use crate::ui::editor::TextAction;

use super::document::Document;
use super::markdown::Fonts;
use super::history::History;
use crate::ui::theme;

mod editing;
mod find;
pub(crate) use find::find_all;
mod format;
mod keys;
mod links;
mod media;
mod marks;
mod multicursor;
mod outline;
mod pages;
mod pairs;
mod paste;
pub use paste::Smart;
mod selection_highlight;
mod suggest;
mod table;
mod widget_events;
mod widget_layout;
mod widget_paint;

pub struct TextEditor {
    doc: Document,
    history: History,
    styles: StyleSet,
    style: Option<fn() -> Vec<StyleProperty>>,
    anchor: usize,
    focus: usize,
    affinity: Affinity,
    column: Option<f64>,
    compose: Option<(String, Option<(usize, usize)>)>,
    width: f64,
    caret_visible: bool,
    pub(super) painted: (f64, f64),
    blink_start: std::time::Instant,
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
    find: find::Find,
    smart: Smart,
    suggest_shown: Option<crate::ui::editor::resolve::Suggestions>,
    hover_terms: Vec<String>,
    hover_marks: Vec<Range<usize>>,
    external_reveal: Option<(Range<usize>, std::time::Instant)>,
    extra_cursors: Vec<(usize, usize)>,
}

pub(super) fn visible(transform: Affine, window: Size) -> (f64, f64) {
    let r = transform.inverse().transform_rect_bbox(window.to_rect());
    (r.y0, r.y1)
}

pub(super) fn rect(b: BoundingBox, top: f64) -> Rect {
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
            style: None,
            anchor: 0,
            focus: 0,
            affinity: Affinity::Downstream,
            column: None,
            compose: None,
            width: 0.0,
            caret_visible: true,
            painted: (0.0, 0.0),
            blink_start: std::time::Instant::now(),
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
            find: find::Find::default(),
            smart: Smart::default(),
            suggest_shown: None,
            hover_terms: Vec::new(),
            hover_marks: Vec::new(),
            external_reveal: None,
            extra_cursors: Vec::new(),
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
        self.doc.set_markdown(Some(Fonts { size, mono: theme::editor_font("iA Writer Mono"), ch: 0.0, callouts: self.smart.callouts }));
        self
    }

    pub fn report_edits(mut self) -> Self {
        self.doc.report_edits();
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

    pub fn styled(mut self, style: fn() -> Vec<StyleProperty>) -> Self {
        for property in style() {
            self.styles.insert(property);
        }
        self.style = Some(style);
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
        widget.refind();
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

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, props: &mut PropertiesMut<'_>, interval: u64) {
        self.do_anim_frame(ctx, props, interval);
    }

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        self.do_pointer_event(ctx, props, event);
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, props: &mut PropertiesMut<'_>, event: &TextEvent) {
        self.do_text_event(ctx, props, event);
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn accepts_text_input(&self) -> bool {
        true
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, props: &mut PropertiesMut<'_>, event: &Update) {
        self.do_update(ctx, props, event);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        self.do_measure(ctx, props, axis, len_req, cross)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        self.do_layout(ctx, props, size);
    }

    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {
        self.do_compose(ctx);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, painter: &mut masonry::imaging::Painter<'_>) {
        self.do_paint(ctx, props, painter);
    }

    fn post_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, painter: &mut masonry::imaging::Painter<'_>) {
        self.do_post_paint(ctx, props, painter);
    }

    fn get_cursor(&self, ctx: &QueryCtx<'_>, pos: Point) -> CursorIcon {
        self.do_get_cursor(ctx, pos)
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
