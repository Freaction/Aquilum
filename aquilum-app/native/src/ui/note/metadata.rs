use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, CursorIcon, BrushIndex, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget,
    PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, Update, UpdateCtx, Widget,
    render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Size};
use masonry::layout::{LenReq, Length};
use masonry::parley::FontWeight;
use masonry::widgets::{Flex, SizedBox};

use crate::i18n::t;
use crate::ui::icons;
use crate::ui::{text, theme};
use crate::ui::tokens::size;
use crate::ui::widgets::Press;

pub fn metadata_widget(expanded: Option<bool>) -> NewWidget<dyn Widget> {
    match expanded {
        Some(expanded) => NewWidget::new(Flex::row().with_fixed(NewWidget::new(MetadataToggle {
            expanded,
            label: t("editor.metadata"),
            press: Default::default(),
            layout: None,
        }))).erased(),
        None => NewWidget::new(SizedBox::empty().height(Length::ZERO)).erased(),
    }
}

#[derive(Debug)]
pub struct MetadataToggled;

pub struct MetadataToggle {
    expanded: bool,
    label: String,
    press: Press,
    layout: Option<masonry::parley::Layout<BrushIndex>>,
}

const TOGGLE_LINE: f64 = 1.45;

impl MetadataToggle {
    fn layout(
        &mut self,
        font_cx: &mut masonry::parley::FontContext,
        layout_cx: &mut masonry::parley::LayoutContext<BrushIndex>,
    ) -> &masonry::parley::Layout<BrushIndex> {
        self.layout.get_or_insert_with(|| {
            let font = theme::editor_font_settings();
            let style = text::Style { size: font.size, weight: FontWeight::NORMAL.value(), line_height: f64::from(font.size * TOGGLE_LINE as f32) };
            let mut layout = text::builder(font_cx, layout_cx, &self.label, theme::editor_font(&font.family), style).build(&self.label);
            layout.break_all_lines(None);
            layout
        })
    }
}

impl Widget for MetadataToggle {
    type Action = MetadataToggled;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.press.pointer(ctx, event) {
            ctx.submit_action::<MetadataToggled>(MetadataToggled);
        }
        ctx.set_handled();
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::FontsChanged) {
            self.layout = None;
            ctx.request_layout();
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            ctx.submit_action::<MetadataToggled>(MetadataToggled);
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        _cross: Option<Length>,
    ) -> Length {
        let font = theme::editor_font_settings();
        match axis {
            Axis::Vertical => Length::px((f64::from(font.size) * TOGGLE_LINE).max(size::SIZE_20) + size::SPACE_4),
            Axis::Horizontal => {
                let (font_cx, layout_cx) = ctx.text_contexts();
                Length::px((size::SIZE_20 + size::SPACE_4 + f64::from(self.layout(font_cx, layout_cx).width())).ceil())
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
        let (font_cx, layout_cx) = ctx.text_contexts();
        let layout = self.layout(font_cx, layout_cx);
        let x = bounds.x0 + size::SIZE_20 + size::SPACE_4;
        render_text(painter, Affine::translate((x, center - line / 2.0)), layout, &[tm.text_tertiary.into()], true);
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        CursorIcon::Pointer
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
