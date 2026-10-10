use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, BrushIndex, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerButton,
    PointerEvent, PropertiesMut, PropertiesRef, PropertySet, RegisterCtx, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
    WidgetPod, render_text,
};
use masonry::imaging::Painter;
use crate::ui::editor::TextEditor;
use masonry::kurbo::{Affine, Axis, Point, Size};
use masonry::layout::{LenDef, LenReq, Length, SizeDef};
use masonry::parley::style::StyleProperty;
use masonry::parley::{FontWeight, Layout, LineHeight};
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Background, ContentColor, Gap, Padding, SelectionColor, CaretColor};
use masonry::widgets::{Flex, SizedBox};

use super::handle::Handle;
use super::icons;
use super::scroll::ScrollArea;
use super::theme::{self, EditorFont};
use super::tokens::{number, size};
use super::widgets::{ButtonSize, IconButton, Slot, TextButton, Variant};
use super::text;
use crate::i18n::t;

pub struct NoteView<'a> {
    pub file_name: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub metadata: Option<bool>,
    pub can_back: bool,
    pub can_forward: bool,
    pub focus_mode: bool,
    pub hero: Option<super::book::HeroView>,
    pub resolver: Option<std::sync::Arc<dyn crate::ui::editor::resolve::Resolver>>,
}

pub struct NoteIds {
    pub back: WidgetId,
    pub forward: WidgetId,
    pub focus: WidgetId,
    pub name: Handle<Caption>,
    pub title_box: Handle<TitleField>,
    pub title: Handle<TextEditor>,
    pub metadata: Handle<Slot>,
    pub body: Handle<crate::ui::editor::TextEditor>,
    pub hero: Option<(Handle<super::book::Hero>, super::book::HeroIds)>,
    pub hero_slot: Handle<Slot>,
    pub spacer: Handle<SizedBox>,
    pub column: Handle<EditorColumn>,
    pub scroll: Handle<ScrollArea<Flex>>,
}

pub struct HeroParts {
    pub widget: NewWidget<dyn Widget>,
    pub hero: Option<(Handle<super::book::Hero>, super::book::HeroIds)>,
    pub top: f64,
    pub inset: f64,
}

pub fn hero_parts(view: Option<super::book::HeroView>) -> HeroParts {
    match view {
        Some(view) => {
            let (hero, ids) = super::book::hero(view);
            let handle = Handle::of(&hero);
            HeroParts { widget: hero.erased(), hero: Some((handle, ids)), top: size::SPACE_24, inset: size::EDITOR_PADDING_X * 2.0 }
        }
        None => HeroParts {
            widget: NewWidget::new(SizedBox::empty().height(Length::ZERO)).erased(),
            hero: None,
            top: size::EDITOR_PADDING_Y,
            inset: 0.0,
        },
    }
}

pub fn page(view: NoteView<'_>) -> (NewWidget<Flex>, NoteIds) {
    let tm = theme::current();
    let font = theme::editor_font_settings();
    let back = NewWidget::new(IconButton::new(icons::ARROW_LEFT, t("editor.back"))).disabled(!view.can_back);
    let forward = NewWidget::new(IconButton::new(icons::ARROW_RIGHT, t("editor.forward"))).disabled(!view.can_forward);
    let (back_id, forward_id) = (back.id(), forward.id());
    let focus = NewWidget::new(TextButton::new(t("editor.focusMode"), Variant::Ghost).size(ButtonSize::Xs).pressed(view.focus_mode));
    let focus_id = focus.id();
    let name = NewWidget::new(Caption::new(view.file_name));
    let name_handle = Handle::of(&name);
    let mut left = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
    if !view.focus_mode {
        left = left.with_fixed(back).with_fixed(forward);
    }
    let left = NewWidget::new(left).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_4))));
    let mut toolbar = Flex::row().main_axis_alignment(MainAxisAlignment::SpaceBetween).cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(left);
    toolbar = if view.focus_mode { toolbar.with_spacer(1.0) } else { toolbar.with(name, 1.0) };
    let toolbar = toolbar.with_fixed(focus);
    let toolbar = NewWidget::new(SizedBox::new(NewWidget::new(toolbar)).height(Length::px(size::SIZE_40))).with_props(
        PropertySet::new()
            .with(Background::Color(tm.editor_bg))
            .with(Padding::all(Length::px(size::SPACE_4)))
            .with(Gap::new(Length::px(size::SPACE_8))),
    );

    let title_area = TextEditor::single_line(view.title)
        .with_style(StyleProperty::FontFamily(theme::editor_font(&font.family)))
        .with_style(StyleProperty::FontSize(size::FONT_SIZE_24 as f32))
        .with_style(StyleProperty::FontWeight(FontWeight::new(number::EDITOR_HEADING_WEIGHT as f32)))
        .with_style(StyleProperty::LineHeight(LineHeight::FontSizeRelative(font.line_height)));
    let title_area = NewWidget::new(title_area).with_props(
        PropertySet::new().with(ContentColor::new(tm.editor_text)).with(SelectionColor { color: tm.selection_bg }),
    );
    let title = Handle::of(&title_area);
    let title_field = TitleField::new(title_area, view.title.is_empty());
    let title_box = Handle::of(&title_field);

    let metadata = Slot::fit(metadata_widget(view.metadata));
    let metadata_handle = Handle::of(&metadata);

    let body_area = crate::ui::editor::TextEditor::new(view.body)
        .with_style(StyleProperty::FontFamily(theme::editor_font(&font.family)))
        .with_style(StyleProperty::FontSize(font.size))
        .with_style(StyleProperty::FontWeight(FontWeight::new(font.weight)))
        .with_style(StyleProperty::LineHeight(LineHeight::FontSizeRelative(font.line_height)))
        .markdown()
        .with_resolver(view.resolver.clone());
    let body_area = NewWidget::new(body_area).with_props(
        PropertySet::new()
            .with(ContentColor::new(tm.editor_text))
            .with(SelectionColor { color: tm.selection_bg })
            .with(CaretColor { color: tm.editor_text }),
    );
    let body = Handle::of(&body_area);

    let content = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(title_field)
        .with_fixed(metadata)
        .with_fixed(body_area);
    let parts = hero_parts(view.hero);
    let hero = parts.hero;
    let hero_slot = Slot::fit(parts.widget);
    let hero_slot_handle = Handle::of(&hero_slot);
    let spacer = NewWidget::new(SizedBox::empty().height(Length::px(parts.top)));
    let spacer_handle = Handle::of(&spacer);
    let column = EditorColumn::new(NewWidget::new(content), parts.inset);
    let column_handle = Handle::of(&column);
    let wrapper = NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(spacer).with_fixed(column)).with_props(
        PropertySet::new().with(Padding {
            top: Length::ZERO,
            bottom: Length::px(size::EDITOR_PADDING_Y + size::EDITOR_SCROLL_PAST_END),
            left: Length::px(size::EDITOR_PADDING_X),
            right: Length::px(size::EDITOR_PADDING_X),
        }),
    );
    let scroller = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(hero_slot).with_fixed(wrapper);
    let scroller = NewWidget::new(scroller).with_props(PropertySet::new().with(Background::Color(tm.editor_bg)));
    let scroll = NewWidget::new(ScrollArea::new(scroller));
    let scroll_handle = Handle::of(&scroll);
    let page = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(toolbar)
        .with(scroll, 1.0);
    let page = NewWidget::new(page).with_props(PropertySet::new().with(Background::Color(tm.editor_bg)));
    let ids = NoteIds {
        back: back_id,
        forward: forward_id,
        focus: focus_id,
        name: name_handle,
        title_box,
        title,
        metadata: metadata_handle,
        body,
        hero,
        hero_slot: hero_slot_handle,
        spacer: spacer_handle,
        column: column_handle,
        scroll: scroll_handle,
    };
    (page, ids)
}

pub fn metadata_widget(expanded: Option<bool>) -> NewWidget<dyn Widget> {
    match expanded {
        Some(expanded) => NewWidget::new(Flex::row().with_fixed(NewWidget::new(MetadataToggle { expanded, label: t("editor.metadata") }))).erased(),
        None => NewWidget::new(SizedBox::empty().height(Length::ZERO)).erased(),
    }
}

#[derive(Debug)]
pub enum TitleAction {
    Blurred,
}

pub struct TitleField {
    field: WidgetPod<TextEditor>,
    empty: bool,
    hint: Option<Layout<BrushIndex>>,
}

impl TitleField {
    pub fn new(field: NewWidget<TextEditor>, empty: bool) -> NewWidget<Self> {
        NewWidget::new(TitleField { field: field.to_pod(), empty, hint: None })
    }

    pub fn set_empty(this: &mut WidgetMut<'_, Self>, empty: bool) {
        if this.widget.empty != empty {
            this.widget.empty = empty;
            this.ctx.request_paint_only();
        }
    }
}

impl Widget for TitleField {
    type Action = TitleAction;

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::ChildFocusChanged(false) => ctx.submit_action::<TitleAction>(TitleAction::Blurred),
            Update::FontsChanged => {
                self.hint = None;
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.field);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        match axis {
            Axis::Horizontal => match len_req {
                LenReq::FitContent(space) => space,
                _ => Length::ZERO,
            },
            Axis::Vertical => {
                let context = masonry::layout::LayoutSize::maybe(Axis::Horizontal, cross);
                let inner = ctx.compute_length(&mut self.field, LenReq::MaxContent.into(), context, Axis::Vertical, cross);
                Length::px(inner.get() + size::SPACE_12 * 2.0)
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let auto = SizeDef::new(LenDef::FitContent(Length::px(size_.width)), LenDef::MaxContent);
        let child = ctx.compute_size(&mut self.field, auto, size_.into());
        ctx.run_layout(&mut self.field, Size::new(size_.width, child.height));
        ctx.place_child(&mut self.field, Point::new(0.0, size::SPACE_12));
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if !self.empty {
            return;
        }
        let font = theme::editor_font_settings();
        let hint = self.hint.get_or_insert_with(|| {
            let text = t("editor.untitled");
            let (font_cx, layout_cx) = ctx.text_contexts();
            let mut builder = layout_cx.ranged_builder(font_cx, &text, 1.0, true);
            builder.push_default(StyleProperty::FontFamily(theme::editor_font(&font.family)));
            builder.push_default(StyleProperty::FontSize(size::FONT_SIZE_24 as f32));
            builder.push_default(StyleProperty::FontWeight(FontWeight::new(number::EDITOR_HEADING_WEIGHT as f32)));
            builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(font.line_height)));
            let mut layout = builder.build(&text);
            layout.break_all_lines(None);
            layout
        });
        let origin = ctx.content_box().origin() + masonry::kurbo::Vec2::new(0.0, size::SPACE_12);
        render_text(painter, Affine::translate(origin.to_vec2()), hint, &[theme::current().editor_placeholder_color.into()], true);
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(t("editor.titleAria"));
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.field.id()])
    }
}

#[derive(Debug)]
pub struct MetadataToggled;

pub struct MetadataToggle {
    expanded: bool,
    label: String,
}

const TOGGLE_LINE: f64 = 1.45;

impl Widget for MetadataToggle {
    type Action = MetadataToggled;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if let PointerEvent::Down(e) = event
            && e.button == Some(PointerButton::Primary)
        {
            ctx.submit_action::<MetadataToggled>(MetadataToggled);
            ctx.set_handled();
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            ctx.submit_action::<MetadataToggled>(MetadataToggled);
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        let font = theme::editor_font_settings();
        match axis {
            Axis::Vertical => Length::px((f64::from(font.size) * TOGGLE_LINE).max(size::SIZE_20) + size::SPACE_4),
            Axis::Horizontal => {
                let label = &self.label;
                let (font_cx, layout_cx) = ctx.text_contexts();
                let mut builder = layout_cx.ranged_builder(font_cx, label, 1.0, true);
                builder.push_default(StyleProperty::FontFamily(theme::editor_font(&font.family)));
                builder.push_default(StyleProperty::FontSize(font.size));
                let mut layout = builder.build(label);
                layout.break_all_lines(None);
                Length::px((size::SIZE_20 + size::SPACE_4 + f64::from(layout.width())).ceil())
            }
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let font = theme::editor_font_settings();
        let bounds = ctx.content_box();
        let line = f64::from(font.size) * TOGGLE_LINE;
        let row = line.max(size::SIZE_20);
        let center = bounds.y0 + row / 2.0;
        let icon = if self.expanded { icons::CHEVRON_DOWN } else { icons::CHEVRON_RIGHT };
        icon.draw(painter, Point::new(bounds.x0, center - size::SIZE_20 / 2.0), size::SIZE_20, 1.5, tm.icon_tertiary);
        let label = &self.label;
        let (font_cx, layout_cx) = ctx.text_contexts();
        let mut builder = layout_cx.ranged_builder(font_cx, label, 1.0, true);
        builder.push_default(StyleProperty::FontFamily(theme::editor_font(&font.family)));
        builder.push_default(StyleProperty::FontSize(font.size));
        builder.push_default(StyleProperty::LineHeight(LineHeight::Absolute(line as f32)));
        let mut layout = builder.build(label);
        layout.break_all_lines(None);
        let x = bounds.x0 + size::SIZE_20 + size::SPACE_4;
        render_text(painter, Affine::translate((x, center - line / 2.0)), &layout, &[tm.text_tertiary.into()], true);
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
        node.set_expanded(self.expanded);
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub struct Caption {
    text: String,
    line: text::Line,
}

impl Caption {
    pub fn new(text: &str) -> Self {
        Caption { text: text.to_owned(), line: text::Line::default() }
    }

    pub fn set_text(this: &mut WidgetMut<'_, Self>, text: &str) {
        if this.widget.text != text {
            this.widget.text = text.to_owned();
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }
}

impl Widget for Caption {
    type Action = masonry::core::NoAction;

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::FontsChanged) {
            self.line.clear();
            ctx.request_paint_only();
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(size::SIZE_20),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::ZERO,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let bounds = ctx.content_box();
        let style = text::Style::ui(size::SIZE_20);
        let layout = self.line.layout(ctx, &self.text, bounds.width(), style);
        let x = bounds.x0 + ((bounds.width() - f64::from(layout.width())) / 2.0).max(0.0);
        let y = bounds.center().y - size::SIZE_20 / 2.0;
        render_text(painter, Affine::translate((x, y)), layout, &[theme::current().text_tertiary.into()], true);
    }

    fn accessibility_role(&self) -> Role {
        Role::Label
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_value(self.text.clone());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub fn ch_width(ctx: &mut MeasureCtx<'_>) -> f64 {
    let font = theme::editor_font_settings();
    let (font_cx, layout_cx) = ctx.text_contexts();
    let mut builder = layout_cx.ranged_builder(font_cx, "0", 1.0, true);
    builder.push_default(StyleProperty::FontFamily(theme::editor_font(&font.family)));
    builder.push_default(StyleProperty::FontSize(font.size));
    let mut layout = builder.build("0");
    layout.break_all_lines(None);
    (f64::from(layout.width()) * f64::from(font.max_width_ch)).round()
}

pub struct EditorColumn {
    child: WidgetPod<Flex>,
    ch: Option<(EditorFont, f64)>,
    inset: f64,
}

impl EditorColumn {
    fn new(child: NewWidget<Flex>, inset: f64) -> NewWidget<Self> {
        NewWidget::new(EditorColumn { child: child.to_pod(), ch: None, inset })
    }

    pub fn set_inset(this: &mut WidgetMut<'_, Self>, inset: f64) {
        if this.widget.inset != inset {
            this.widget.inset = inset;
            this.widget.ch = None;
            this.ctx.request_layout();
        }
    }

    fn max(&mut self, ctx: &mut MeasureCtx<'_>) -> f64 {
        let font = theme::editor_font_settings();
        if let Some((cached, width)) = &self.ch
            && *cached == *font
        {
            return *width;
        }
        let width = ch_width(ctx) - self.inset;
        self.ch = Some(((*font).clone(), width));
        width
    }
}

impl Widget for EditorColumn {
    type Action = masonry::core::NoAction;

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::FontsChanged) {
            self.ch = None;
            ctx.request_layout();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let max = self.max(ctx);
        match axis {
            Axis::Horizontal => match len_req {
                LenReq::MinContent => Length::ZERO,
                LenReq::MaxContent => Length::px(max),
                LenReq::FitContent(space) => space,
            },
            Axis::Vertical => {
                let inner = Length::px(cross.map_or(max, |c| c.get().min(max)));
                let context = masonry::layout::LayoutSize::maybe(Axis::Horizontal, Some(inner));
                ctx.compute_length(&mut self.child, LenReq::MaxContent.into(), context, Axis::Vertical, Some(inner))
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let max = self.ch.as_ref().map_or(size_.width, |(_, w)| *w);
        let inner = size_.width.min(max);
        let auto = SizeDef::new(LenDef::Fixed(Length::px(inner)), LenDef::MaxContent);
        let child = ctx.compute_size(&mut self.child, auto, Size::new(inner, size_.height).into());
        ctx.run_layout(&mut self.child, Size::new(inner, child.height));
        ctx.place_child(&mut self.child, Point::new(((size_.width - inner) / 2.0).round(), 0.0));
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
