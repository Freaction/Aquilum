use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesRef, PropertySet, RegisterCtx, Widget, WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Size};
use masonry::layout::{LenDef, LenReq, Length, SizeDef};
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::marked::marked_layout;
use crate::i18n::t;
use crate::ui::editor::TextEditor;
use crate::ui::handle::Handle;
use crate::ui::tokens::size;
use crate::ui::settings::controls::InputBox;
use crate::ui::widgets::{IconButton, IconView, Slot};
use crate::ui::{icons, style, theme};

pub struct Placeholder {
    field: WidgetPod<TextEditor>,
    hint: String,
    empty: bool,
    height: f64,
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
            (Axis::Vertical, _) => Length::px(self.height),
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

pub fn field(query: &str, hint: String, height: f64) -> (NewWidget<Placeholder>, Handle<TextEditor>, Handle<Placeholder>) {
    let field = NewWidget::new(TextEditor::single_line(query).styled(|| style::ui_text(size::SIZE_20 as f32))).with_class(style::FIELD);
    let field_handle = Handle::of(&field);
    let placeholder = NewWidget::new(Placeholder { field: field.to_pod(), hint, empty: query.is_empty(), height });
    let placeholder_handle = Handle::of(&placeholder);
    (placeholder, field_handle, placeholder_handle)
}

fn padded(child: NewWidget<impl Widget>, left: f64, right: f64) -> NewWidget<Flex> {
    let padding = Padding { top: Length::ZERO, bottom: Length::ZERO, left: Length::px(left), right: Length::px(right) };
    NewWidget::new(Flex::row().with_fixed(child)).with_props(PropertySet::new().with(padding))
}

pub fn after_gap(child: NewWidget<impl Widget>) -> NewWidget<Flex> {
    padded(child, size::INPUT_GAP, 0.0)
}

pub fn input_row(icon: NewWidget<IconView>, placeholder: NewWidget<Placeholder>, trailing: Option<NewWidget<dyn Widget>>) -> NewWidget<Flex> {
    let mut row = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(padded(icon, 0.0, size::INPUT_GAP)).with(placeholder, 1.0);
    if let Some(trailing) = trailing {
        row = row.with_fixed(trailing);
    }
    NewWidget::new(row)
}

pub struct FilterBox {
    pub widget: NewWidget<InputBox<Flex>>,
    pub field: Handle<TextEditor>,
    pub placeholder: Handle<Placeholder>,
    pub clear: Handle<IconButton>,
    pub clear_slot: Handle<Slot>,
}

pub fn filter_box(query: &str, hint: String) -> FilterBox {
    let (placeholder, field, placeholder_handle) = field(query, hint, size::INPUT_CONTENT_HEIGHT);
    let icon = IconView::new(icons::SEARCH, size::INPUT_ICON_SIZE, size::SIDEBAR_ICON_SIZE, 1.4, || theme::current().icon_faint);
    let clear = NewWidget::new(IconButton::new(icons::X, t("search.clear")).xs());
    let clear_handle = Handle::of(&clear);
    let clear_slot = Slot::shown(after_gap(clear), !query.is_empty());
    let clear_slot_handle = Handle::of(&clear_slot);
    let widget = InputBox::unfilled(InputBox::compact(input_row(icon, placeholder, Some(clear_slot.erased()))));
    FilterBox { widget, field, placeholder: placeholder_handle, clear: clear_handle, clear_slot: clear_slot_handle }
}

pub struct FindBox {
    pub widget: NewWidget<Flex>,
    pub field: Handle<TextEditor>,
    pub placeholder: Handle<Placeholder>,
}

fn row(query: &str, hint: String, clear: Option<NewWidget<IconButton>>) -> FindBox {
    let (placeholder, field, placeholder_handle) = field(query, hint, size::SIZE_24);
    let has_clear = clear.is_some();
    let mut row = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(IconView::new(icons::SEARCH, size::SIZE_20, size::SIZE_20, 1.4, || theme::current().icon_secondary))
        .with(placeholder, 1.0);
    if let Some(clear) = clear {
        row = row.with_fixed(clear);
    }
    let content = if has_clear { size::SIZE_32 } else { size::SIZE_24 };
    let widget = NewWidget::new(row).with_props(
        PropertySet::new()
            .with(Gap::new(Length::px(size::SPACE_6)))
            .with(Padding::from_vh(Length::px((size::SEARCH_BOX_HEIGHT - content) / 2.0), Length::px(size::SPACE_16))),
    );
    FindBox { widget, field, placeholder: placeholder_handle }
}

pub fn find_box(hint: String) -> FindBox {
    row("", hint, None)
}

pub fn search_box(query: &str) -> SearchBox {
    let clear = NewWidget::new(IconButton::new(icons::X, t("search.clear")));
    let clear_handle = Handle::of(&clear);
    let FindBox { widget, field, placeholder } = row(query, t("search.title"), Some(clear));
    SearchBox { widget, field, placeholder, clear: clear_handle }
}
