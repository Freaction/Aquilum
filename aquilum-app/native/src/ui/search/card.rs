use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, BrushIndex, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef,
    RegisterCtx, Update, UpdateCtx, Widget, WidgetMut, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Stroke};
use masonry::layout::{LenReq, Length};
use masonry::parley::style::StyleProperty;
use masonry::parley::{Affinity, Cursor, FontWeight, Layout, LineHeight, Selection};
use masonry::peniko::Color;

use crate::ui::icons;
use crate::ui::theme;
use crate::ui::tokens::{number, size};
use crate::ui::widgets::Press;

pub type Marks = Vec<(usize, usize)>;

pub struct CardData {
    pub title: String,
    pub extension: String,
    pub snippet: String,
    pub count: String,
    pub title_marks: Marks,
    pub extension_marks: Marks,
    pub snippet_marks: Marks,
}

#[derive(Debug)]
pub enum CardAction {
    Hover,
    Open { new_tab: bool },
}

pub struct ResultCard {
    data: CardData,
    selected: bool,
    press: Press,
}

impl ResultCard {
    pub fn new(data: CardData, selected: bool) -> NewWidget<Self> {
        NewWidget::new(ResultCard { data, selected, press: Press::default() })
    }

    pub fn set_selected(this: &mut WidgetMut<'_, Self>, selected: bool) {
        if this.widget.selected != selected {
            this.widget.selected = selected;
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }
}

pub(super) fn marked_layout(ctx: &mut PaintCtx<'_>, text: &str, marks: &[(usize, usize)], line_height: f32, width: Option<f32>) -> Layout<BrushIndex> {
    let font = theme::ui_font_settings();
    let (font_cx, layout_cx) = ctx.text_contexts();
    let mut builder = layout_cx.ranged_builder(font_cx, text, 1.0, true);
    builder.push_default(StyleProperty::FontFamily(theme::ui_font(&font.family)));
    builder.push_default(StyleProperty::FontSize(size::FONT_SIZE_UI_BASE as f32));
    builder.push_default(StyleProperty::FontWeight(FontWeight::new(font.weight)));
    builder.push_default(StyleProperty::LineHeight(LineHeight::Absolute(line_height)));
    for &(start, end) in marks {
        builder.push(StyleProperty::Brush(BrushIndex(1)), start..end);
    }
    let mut layout = builder.build(text);
    layout.break_all_lines(width);
    layout
}

fn paint_marked(painter: &mut Painter<'_>, layout: &Layout<BrushIndex>, marks: &[(usize, usize)], origin: Point, color: Color) {
    let t = theme::current();
    for &(start, end) in marks {
        let selection = Selection::new(
            Cursor::from_byte_index(layout, start, Affinity::Downstream),
            Cursor::from_byte_index(layout, end, Affinity::Upstream),
        );
        for (rect, _) in selection.geometry(layout) {
            let rect = Rect::new(rect.x0, rect.y0, rect.x1, rect.y1) + origin.to_vec2();
            painter.fill(RoundedRect::from_rect(rect, size::ROUNDED_SM), t.search_match_bg).draw();
        }
    }
    render_text(painter, Affine::translate(origin.to_vec2()), layout, &[color.into(), t.text_primary.into()], true);
}

impl Widget for ResultCard {
    type Action = CardAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if let Some(e) = self.press.click(ctx, event) {
            let m = e.state.modifiers;
            ctx.submit_action::<CardAction>(CardAction::Open { new_tab: m.ctrl() || m.meta() });
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            ctx.submit_action::<CardAction>(CardAction::Open { new_tab: false });
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(true) => ctx.submit_action::<CardAction>(CardAction::Hover),
            Update::HoveredChanged(false) => ctx.request_paint_only(),
            Update::FontsChanged => ctx.request_layout(),
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(size::SEARCH_RESULT_HEIGHT),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::ZERO,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let shape = RoundedRect::from_rect(bounds.inflate(-0.5, -0.5), size::ROUNDED_XL);
        let bg = if self.selected || ctx.is_hovered() { t.search_result_bg_hover } else { t.search_result_bg };
        painter.fill(shape, bg).draw();
        painter.stroke(&shape, &Stroke::new(size::BORDER_1), t.search_result_border).draw();

        let pad = size::SPACE_12;
        let top = bounds.y0 + pad;
        icons::FILE.draw(painter, Point::new(bounds.x0 + pad, top), size::SIZE_20, 1.2, t.icon_secondary);

        let line = size::SIZE_20 as f32;
        let count = marked_layout(ctx, &self.data.count, &[], line, None);
        let count_x = bounds.x1 - pad - f64::from(count.width());
        render_text(painter, Affine::translate((count_x, top)), &count, &[t.text_tertiary.into()], true);

        let name_x = bounds.x0 + pad + size::SIZE_20 + size::SPACE_6;
        let name_right = count_x - size::SPACE_12;
        painter.push_fill_clip(Rect::new(name_x, top, name_right.max(name_x), top + size::SIZE_20));
        let title = marked_layout(ctx, &self.data.title, &self.data.title_marks, line, None);
        let title_width = f64::from(title.width());
        paint_marked(painter, &title, &self.data.title_marks, Point::new(name_x, top), t.text_secondary);
        if !self.data.extension.is_empty() {
            let extension = format!(".{}", self.data.extension);
            let marks: Vec<_> = self.data.extension_marks.iter().map(|(s, e)| (s + 1, e + 1)).collect();
            let layout = marked_layout(ctx, &extension, &marks, line, None);
            paint_marked(painter, &layout, &marks, Point::new(name_x + title_width, top), t.text_tertiary);
        }
        painter.pop_clip();

        let snippet_x = bounds.x0 + pad + size::SIZE_28;
        let snippet_y = top + size::SIZE_20 + size::SPACE_6;
        let line_height = (size::FONT_SIZE_UI_BASE * number::FONT_LINE_HEIGHT_SNUG) as f32;
        let width = (bounds.x1 - pad - snippet_x).max(0.0) as f32;
        let snippet = marked_layout(ctx, &self.data.snippet, &self.data.snippet_marks, line_height, Some(width));
        painter.push_fill_clip(Rect::new(snippet_x, snippet_y, bounds.x1 - pad, snippet_y + f64::from(line_height) * 3.0));
        paint_marked(painter, &snippet, &self.data.snippet_marks, Point::new(snippet_x, snippet_y), t.text_tertiary);
        painter.pop_clip();
    }

    fn accessibility_role(&self) -> Role {
        Role::ListBoxOption
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.data.title.clone());
        node.set_selected(self.selected);
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
