use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, BrushIndex, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerButton,
    PointerEvent, PropertiesMut, PropertiesRef, PropertySet, RegisterCtx, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
    WidgetPod, render_text,
};
use masonry::imaging::Painter;
use crate::ui::editor::TextEditor;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Stroke};
use masonry::layout::{LenDef, LenReq, Length, SizeDef};
use masonry::parley::style::StyleProperty;
use masonry::parley::{Affinity, Cursor, FontWeight, Layout, LineHeight, Selection};
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{CaretColor, ContentColor, Gap, Padding, SelectionColor};
use masonry::widgets::{Flex, Label};

use super::handle::Handle;
use super::icons::{self, Icon};
use super::tokens::{number, size};
use super::widgets::{IconButton, IconView};
use super::theme;
use crate::i18n::t;

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
    down: bool,
}

impl ResultCard {
    pub fn new(data: CardData, selected: bool) -> NewWidget<Self> {
        NewWidget::new(ResultCard { data, selected, down: false })
    }

    pub fn set_selected(this: &mut WidgetMut<'_, Self>, selected: bool) {
        if this.widget.selected != selected {
            this.widget.selected = selected;
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }
}

fn marked_layout(ctx: &mut PaintCtx<'_>, text: &str, marks: &[(usize, usize)], line_height: f32, width: Option<f32>) -> Layout<BrushIndex> {
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

fn paint_marked(painter: &mut Painter<'_>, layout: &Layout<BrushIndex>, marks: &[(usize, usize)], origin: Point, color: masonry::peniko::Color) {
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
        match event {
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                self.down = true;
                ctx.capture_pointer();
            }
            PointerEvent::Up(e) if e.button == Some(PointerButton::Primary) && self.down => {
                self.down = false;
                if ctx.is_hovered() {
                    let m = e.state.modifiers;
                    ctx.submit_action::<CardAction>(CardAction::Open { new_tab: m.ctrl() || m.meta() });
                }
            }
            PointerEvent::Cancel(_) => self.down = false,
            _ => {}
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

pub struct Placeholder {
    field: WidgetPod<TextEditor>,
    hint: String,
    empty: bool,
}

impl Placeholder {
    pub fn set_empty(this: &mut WidgetMut<'_, Self>, empty: bool) {
        if this.widget.empty != empty {
            this.widget.empty = empty;
            this.ctx.request_paint_only();
        }
    }
}

impl Widget for Placeholder {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.field);
    }

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(size::SIZE_24),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::ZERO,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let auto = SizeDef::new(LenDef::FitContent(Length::px(size_.width)), LenDef::MaxContent);
        let child = ctx.compute_size(&mut self.field, auto, size_.into());
        let child = Size::new(size_.width, child.height);
        ctx.run_layout(&mut self.field, child);
        ctx.place_child(&mut self.field, Point::new(0.0, ((size_.height - child.height) / 2.0).round()));
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if !self.empty {
            return;
        }
        let t = theme::current();
        let bounds = ctx.content_box();
        let layout = marked_layout(ctx, &self.hint, &[], size::SIZE_20 as f32, None);
        render_text(painter, Affine::translate((bounds.x0, bounds.center().y - size::SIZE_20 / 2.0)), &layout, &[t.text_tertiary.into()], true);
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.field.id()])
    }
}

pub struct SearchBox {
    pub widget: NewWidget<Flex>,
    pub field: Handle<TextEditor>,
    pub placeholder: Handle<Placeholder>,
    pub clear: Handle<IconButton>,
}

pub fn search_box(query: &str) -> SearchBox {
    let t = theme::current();
    let font = theme::ui_font_settings();
    let field = TextEditor::single_line(query)
        .with_style(StyleProperty::FontFamily(theme::ui_font(&font.family)))
        .with_style(StyleProperty::FontSize(size::FONT_SIZE_UI_BASE as f32))
        .with_style(StyleProperty::LineHeight(LineHeight::Absolute(size::SIZE_20 as f32)));
    let field = NewWidget::new(field).with_props(
        PropertySet::new()
            .with(ContentColor::new(t.text_primary))
            .with(CaretColor { color: t.text_primary })
            .with(SelectionColor { color: t.selection_bg }),
    );
    let field_handle = Handle::of(&field);
    let placeholder = NewWidget::new(Placeholder { field: field.to_pod(), hint: t_owned("search.title"), empty: query.is_empty() });
    let placeholder_handle = Handle::of(&placeholder);
    let clear = NewWidget::new(IconButton::new(icons::X, t_owned("search.clear")));
    let clear_handle = Handle::of(&clear);
    let row = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(IconView::new(icons::SEARCH, size::SIZE_20, size::SIZE_20, 1.4, || theme::current().icon_secondary))
        .with(placeholder, 1.0)
        .with_fixed(clear);
    let row = NewWidget::new(row).with_props(
        PropertySet::new()
            .with(Gap::new(Length::px(size::SPACE_6)))
            .with(Padding::from_vh(Length::px(size::SPACE_8), Length::px(size::SPACE_16))),
    );
    SearchBox { widget: row, field: field_handle, placeholder: placeholder_handle, clear: clear_handle }
}

fn t_owned(key: &str) -> String {
    t(key)
}

pub fn empty_state(icon: Icon, title: &str, description: &str) -> NewWidget<Flex> {
    empty_state_with(icon, title, Some(description), None)
}

pub fn empty_state_with(icon: Icon, title: &str, description: Option<&str>, action: Option<NewWidget<dyn Widget>>) -> NewWidget<Flex> {
    let t = theme::current();
    let line = (size::FONT_SIZE_UI_BASE * number::FONT_LINE_HEIGHT_SNUG) as f32;
    let mut copy = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(centered(title, t.text_secondary, line));
    if let Some(description) = description {
        copy = copy.with_fixed(centered(description, t.text_tertiary, line));
    }
    let copy = NewWidget::new(copy).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_8))));
    let mut column = Flex::column()
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(IconView::new(icon, size::EMPTY_STATE_ICON_CONTAINER, size::EMPTY_STATE_ICON_SIZE, 1.4, || theme::current().icon_muted))
        .with_fixed(super::widgets::MaxWidth::new(copy, size::EMPTY_STATE_WIDTH));
    if let Some(action) = action {
        column = column.with_fixed(action);
    }
    NewWidget::new(column).with_props(
        PropertySet::new()
            .with(Gap::new(Length::px(size::EMPTY_STATE_GAP)))
            .with(Padding::from_vh(Length::px(size::EMPTY_STATE_PADDING_Y), Length::ZERO)),
    )
}

fn centered(text: &str, color: masonry::peniko::Color, line: f32) -> NewWidget<Label> {
    let font = theme::ui_font_settings();
    let label = Label::new(text)
        .with_text_alignment(masonry::TextAlign::Center)
        .with_style(StyleProperty::FontFamily(theme::ui_font(&font.family)))
        .with_style(StyleProperty::FontSize(size::FONT_SIZE_UI_BASE as f32))
        .with_style(StyleProperty::LineHeight(LineHeight::Absolute(line)));
    NewWidget::new(label).with_props(PropertySet::new().with(ContentColor::new(color)).with(masonry::properties::LineBreaking::WordWrap))
}

fn hint(modifiers: &[&str], icon: Option<Icon>, text: &str) -> NewWidget<Flex> {
    let t = theme::current();
    let font = theme::ui_font_settings();
    let small = |text: &str, color, weight: f32| {
        let label = Label::new(text)
            .with_style(StyleProperty::FontFamily(theme::ui_font(&font.family)))
            .with_style(StyleProperty::FontSize(size::FONT_SIZE_UI_SM as f32))
            .with_style(StyleProperty::FontWeight(FontWeight::new(weight)))
            .with_style(StyleProperty::LineHeight(LineHeight::Absolute(size::SIZE_20 as f32)));
        NewWidget::new(label).with_props(PropertySet::new().with(ContentColor::new(color)))
    };
    let mut kbd = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
    for modifier in modifiers {
        kbd = kbd.with_fixed(small(modifier, t.search_key_hint_color, 500.0));
    }
    if let Some(icon) = icon {
        kbd = kbd.with_fixed(IconView::new(icon, size::FONT_SIZE_12, size::FONT_SIZE_12, 2.2, || theme::current().search_key_hint_color));
    }
    let kbd = NewWidget::new(kbd).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_2))));
    let row = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(kbd).with_fixed(small(text, t.text_tertiary, font.weight));
    NewWidget::new(row).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_4))))
}

pub fn footer() -> NewWidget<Flex> {
    let row = Flex::row()
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(hint(&[], Some(icons::ARROW_UP_DOWN), &t_owned("search.navigate")))
        .with_fixed(hint(&[], Some(icons::CORNER_DOWN_LEFT), &t_owned("search.open")))
        .with_fixed(hint(&["Ctrl"], Some(icons::CORNER_DOWN_LEFT), &t_owned("search.newPane")))
        .with_fixed(hint(&["Shift"], Some(icons::CORNER_DOWN_LEFT), &t_owned("search.create")));
    NewWidget::new(row).with_props(
        PropertySet::new().with(Gap::new(Length::px(size::SPACE_12))).with(Padding::all(Length::px(size::DIALOG_PADDING))),
    )
}

pub fn card_size(room: Size) -> Size {
    let window_height = room.height + size::SPACE_32;
    Size::new(room.width.min(size::SEARCH_WIDTH), (window_height * 0.7).min(room.height))
}

pub fn results_list(cards: Vec<NewWidget<ResultCard>>) -> (NewWidget<Flex>, Vec<WidgetId>) {
    let ids = cards.iter().map(NewWidget::id).collect();
    let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    for card in cards {
        column = column.with_fixed(card);
    }
    let column = NewWidget::new(column).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_6))).with(Padding {
        top: Length::px(size::SPACE_8),
        bottom: Length::px(size::SPACE_8),
        left: Length::px(size::SPACE_8),
        right: Length::px(size::SPACE_4),
    }));
    (column, ids)
}

pub fn offset_of(index: usize) -> (f64, f64) {
    let top = size::SPACE_8 + index as f64 * (size::SEARCH_RESULT_HEIGHT + size::SPACE_6);
    (top, top + size::SEARCH_RESULT_HEIGHT)
}

