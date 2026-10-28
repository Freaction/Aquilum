use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, BrushIndex, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Size};
use masonry::layout::{LenDef, LenReq, Length, SizeDef};
use masonry::parley::Layout;

use crate::i18n::t;
use crate::ui::editor::TextEditor;
use crate::ui::tokens::{number, size};
use crate::ui::{text, theme};

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
            let hint = t("editor.untitled");
            let (font_cx, layout_cx) = ctx.text_contexts();
            let size = size::FONT_SIZE_24 as f32;
            let style = text::Style { size, weight: number::EDITOR_HEADING_WEIGHT as f32, line_height: f64::from(size * font.line_height) };
            let mut layout = text::builder(font_cx, layout_cx, &hint, theme::editor_font(&font.family), style).build(&hint);
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
