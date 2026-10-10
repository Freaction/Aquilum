use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update,
    UpdateCtx, Widget, WidgetMut, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size};
use masonry::layout::{LenReq, Length};

use super::menu::box_shadow;
use super::text::{Line, Style};
use super::theme;
use super::tokens::{number, size};

const MIN_WIDTH: f64 = 200.0;
const MAX_WIDTH: f64 = 420.0;

#[derive(Debug)]
pub struct SuggestPicked(pub usize);

pub struct SuggestList {
    items: Vec<String>,
    query: String,
    selected: usize,
    hovered: Option<usize>,
    lines: Vec<Line>,
}

impl SuggestList {
    pub fn new(items: Vec<String>, query: String, selected: usize) -> NewWidget<Self> {
        NewWidget::new(SuggestList { items, query, selected, hovered: None, lines: Vec::new() })
    }

    pub fn set(this: &mut WidgetMut<'_, Self>, items: Vec<String>, query: String, selected: usize) {
        let resized = this.widget.items.len() != items.len() || this.widget.items != items;
        this.widget.items = items;
        this.widget.query = query;
        this.widget.selected = selected;
        if resized {
            this.widget.hovered = None;
            this.ctx.request_layout();
        }
        this.ctx.request_paint_only();
    }

    pub fn size_for(items: &[String]) -> Size {
        let font = theme::ui_font_settings();
        let width = items.iter().map(|t| t.chars().count() as f64 * f64::from(font.size) * 0.6).fold(0.0, f64::max) + Self::row_pad() * 2.0 + size::GAP_XS * 2.0;
        Size::new(width.clamp(MIN_WIDTH, MAX_WIDTH).round(), items.len() as f64 * Self::row() + size::GAP_XS * 2.0)
    }

    fn row() -> f64 {
        size::SIZE_20 + size::PADDING_XS * 2.0
    }

    fn row_pad() -> f64 {
        size::PADDING_XS
    }

    fn index_at(&self, p: Point) -> Option<usize> {
        let i = ((p.y - size::GAP_XS) / Self::row()).floor();
        (i >= 0.0 && (i as usize) < self.items.len()).then_some(i as usize)
    }
}

impl Widget for SuggestList {
    type Action = SuggestPicked;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Move(update) => {
                let hovered = self.index_at(ctx.local_position(update.current.position));
                if hovered != self.hovered {
                    self.hovered = hovered;
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Leave(_) => {
                if self.hovered.take().is_some() {
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Down(down) => {
                if let Some(index) = self.index_at(ctx.local_position(down.state.position)) {
                    ctx.submit_action::<SuggestPicked>(SuggestPicked(index));
                }
                ctx.set_handled();
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::FontsChanged) {
            self.lines.clear();
            ctx.request_layout();
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(Self::size_for(&self.items).get_coord(axis))
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let bounds = ctx.content_box();
        let shape = RoundedRect::from_rect(bounds, size::RADIUS_XL);
        box_shadow(painter, shape, tm.elevation_float);
        box_shadow(painter, shape, tm.shadow_ring);
        painter.fill(shape, tm.dialog_bg).draw();
        let line = (f64::from(theme::ui_font_settings().size) * number::FONT_LINE_HEIGHT_NORMAL).round();
        let style = Style::ui(line);
        let query = self.query.to_lowercase();
        self.lines.resize_with(self.items.len(), Line::default);
        for (i, (title, cache)) in self.items.iter().zip(&mut self.lines).enumerate() {
            let row = Rect::new(bounds.x0 + size::GAP_XS, bounds.y0 + size::GAP_XS + i as f64 * Self::row(), bounds.x1 - size::GAP_XS, bounds.y0 + size::GAP_XS + (i + 1) as f64 * Self::row());
            let background = if i == self.selected { Some(tm.bg_surface_active) } else if Some(i) == self.hovered { Some(tm.bg_surface_overlay) } else { None };
            if let Some(color) = background {
                painter.fill(RoundedRect::from_rect(row, size::RADIUS_LG), color).draw();
            }
            let width = (row.width() - Self::row_pad() * 2.0).max(1.0);
            let layout = cache.layout(ctx, title, width, style);
            let origin = Point::new(row.x0 + Self::row_pad(), row.y0 + ((row.height() - line) / 2.0).round());
            if !query.is_empty()
                && let Some(at) = title.to_lowercase().find(&query)
                && title.is_char_boundary(at)
                && title.is_char_boundary((at + query.len()).min(title.len()))
            {
                let end = (at + query.len()).min(title.len());
                let start_x = masonry::parley::Cursor::from_byte_index(layout, at, masonry::parley::Affinity::Downstream).geometry(layout, 0.0).x0;
                let end_x = masonry::parley::Cursor::from_byte_index(layout, end, masonry::parley::Affinity::Upstream).geometry(layout, 0.0).x0;
                let mark = Rect::new(origin.x + start_x, origin.y, origin.x + end_x, origin.y + line);
                painter.fill(RoundedRect::from_rect(mark, size::ROUNDED_SM), tm.search_match_bg).draw();
            }
            render_text(painter, Affine::translate(origin.to_vec2()), layout, &[tm.text_primary.into()], true);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::ListBox
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        if let Some(title) = self.items.get(self.selected) {
            node.set_label(title.clone());
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for SuggestList {
    fn capture_pointer_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &PointerEvent) {}
}
