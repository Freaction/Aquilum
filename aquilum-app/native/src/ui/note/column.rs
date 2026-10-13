use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Size};
use masonry::layout::{LenDef, LenReq, Length, SizeDef};
use masonry::parley::style::StyleProperty;
use masonry::widgets::Flex;

use crate::ui::motion::{self, EASE_IN_OUT, approach};
use crate::ui::text;
use crate::ui::theme::{self, EditorFont};
use crate::ui::widgets::bezier;
use crate::ui::tokens::size;

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

const WIDE_MARGIN: f64 = size::SPACE_32 - size::EDITOR_PADDING_X;

pub struct EditorColumn {
    child: WidgetPod<Flex>,
    ch: Option<(EditorFont, f64)>,
    inset: f64,
    wide: bool,
    blend: f64,
}

impl EditorColumn {
    pub fn new(child: NewWidget<Flex>, inset: f64, wide: bool) -> NewWidget<Self> {
        NewWidget::new(EditorColumn { child: child.to_pod(), ch: None, inset, wide, blend: if wide { 1.0 } else { 0.0 } })
    }

    pub fn set_wide(this: &mut WidgetMut<'_, Self>, wide: bool) {
        if this.widget.wide != wide {
            this.widget.wide = wide;
            this.ctx.request_anim_frame();
        }
    }

    fn inner(&self, width: f64, max: f64) -> f64 {
        let narrow = width.min(max);
        let wide = (width - WIDE_MARGIN * 2.0).max(narrow);
        narrow + (wide - narrow) * bezier(EASE_IN_OUT, self.blend)
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

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let goal = if self.wide { 1.0 } else { 0.0 };
        self.blend = approach(self.blend, goal, motion::step(interval as f64 / 1e9, motion::NORMAL));
        ctx.request_layout();
        if self.blend != goal {
            ctx.request_anim_frame();
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
                let inner = Length::px(cross.map_or(max, |c| self.inner(c.get(), max)));
                let context = masonry::layout::LayoutSize::maybe(Axis::Horizontal, Some(inner));
                ctx.compute_length(&mut self.child, LenReq::MaxContent.into(), context, Axis::Vertical, Some(inner))
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let max = self.ch.as_ref().map_or(size_.width, |(_, w)| *w);
        let inner = self.inner(size_.width, max);
        crate::images::show(inner * ctx.scale_factor().max(1.0));
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
