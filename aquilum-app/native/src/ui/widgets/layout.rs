use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesRef, RegisterCtx, Widget, WidgetMut, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Rect, Size};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};
use masonry::peniko::Color;

use crate::ui::tokens::size;

/// Граница с одной стороны панели: полоса толщиной 1 px поперёк `axis`, растянутая вдоль
/// (у `BorderWidth` в Masonry одна ширина на все стороны, у Flex нет выравнивания «растянуть»).
pub struct Rule {
    /// Направление полосы: `Vertical` — вертикальная линия между колонками.
    axis: Axis,
    color: fn() -> Color,
    base: Option<fn() -> Color>,
}

impl Rule {
    pub fn new(axis: Axis, color: fn() -> Color) -> NewWidget<Self> {
        NewWidget::new(Rule { axis, color, base: None })
    }

    pub fn on(axis: Axis, color: fn() -> Color, base: fn() -> Color) -> NewWidget<Self> {
        NewWidget::new(Rule { axis, color, base: Some(base) })
    }
}

impl Widget for Rule {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis == self.axis, len_req) {
            (true, LenReq::FitContent(space)) => space,
            (true, _) => Length::ZERO,
            (false, _) => Length::px(size::BORDER_1),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if let Some(base) = self.base {
            painter.fill(ctx.content_box(), base()).draw();
        }
        painter.fill(ctx.content_box(), (self.color)()).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
/// `max-width` по центру: ребёнок шириной с доступное место, но не шире `max`.
pub struct MaxWidth<W: Widget> {
    child: WidgetPod<W>,
    max: f64,
}

impl<W: Widget> MaxWidth<W> {
    pub fn new(child: NewWidget<W>, max: f64) -> NewWidget<Self> {
        NewWidget::new(MaxWidth { child: child.to_pod(), max })
    }
}

impl<W: Widget> Widget for MaxWidth<W> {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        match axis {
            Axis::Horizontal => match len_req {
                LenReq::MinContent => Length::ZERO,
                LenReq::MaxContent => Length::px(self.max),
                LenReq::FitContent(space) => space,
            },
            Axis::Vertical => {
                let inner = Length::px(cross.map_or(self.max, |c| c.get().min(self.max)));
                let context = LayoutSize::maybe(Axis::Horizontal, Some(inner));
                ctx.compute_length(&mut self.child, LenReq::MaxContent.into(), context, Axis::Vertical, Some(inner))
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let inner = size.width.min(self.max);
        let auto = SizeDef::new(LenDef::FitContent(Length::px(inner)), LenDef::MaxContent);
        let child = ctx.compute_size(&mut self.child, auto, Size::new(inner, size.height).into());
        let child = Size::new(inner, child.height);
        ctx.run_layout(&mut self.child, child);
        ctx.place_child(&mut self.child, Point::new(((size.width - inner) / 2.0).round(), 0.0));
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
/// Гнездо на всё доступное место с заменяемым содержимым: так окно пересобирает интерфейс
/// целиком (смена темы, языка, шрифта), не пересоздавая `RenderRoot`.
pub struct Slot {
    child: WidgetPod<dyn Widget>,
    fit: bool,
}

impl Slot {
    pub fn new(child: NewWidget<dyn Widget>) -> NewWidget<Self> {
        NewWidget::new(Slot { child: child.to_pod(), fit: false })
    }

    pub fn fit(child: NewWidget<dyn Widget>) -> NewWidget<Self> {
        NewWidget::new(Slot { child: child.to_pod(), fit: true })
    }

    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<dyn Widget>) {
        this.ctx.remove_child(std::mem::replace(&mut this.widget.child, child.to_pod()));
        this.ctx.children_changed();
    }
}

impl Widget for Slot {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        if self.fit {
            let context = ctx.context_size();
            return ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross);
        }
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
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
pub struct Wrap {
    children: Vec<WidgetPod<dyn Widget>>,
    gap: f64,
}

impl Wrap {
    pub fn new(children: Vec<NewWidget<dyn Widget>>, gap: f64) -> NewWidget<Self> {
        NewWidget::new(Wrap { children: children.into_iter().map(NewWidget::to_pod).collect(), gap })
    }

    fn sizes(&mut self, ctx: &mut MeasureCtx<'_>) -> Vec<Size> {
        self.children
            .iter_mut()
            .map(|child| {
                let width = ctx.compute_length(child, LenReq::MaxContent.into(), LayoutSize::NONE, Axis::Horizontal, None).get();
                let context = LayoutSize::maybe(Axis::Horizontal, Some(Length::px(width)));
                let height = ctx.compute_length(child, LenReq::MaxContent.into(), context, Axis::Vertical, Some(Length::px(width))).get();
                Size::new(width, height)
            })
            .collect()
    }

    fn rows(sizes: &[Size], width: f64, gap: f64) -> Vec<(usize, Point)> {
        let (mut x, mut y, mut line) = (0.0, 0.0, 0.0_f64);
        let mut placed = Vec::with_capacity(sizes.len());
        for (index, size) in sizes.iter().enumerate() {
            if x > 0.0 && x + size.width > width {
                x = 0.0;
                y += line + gap;
                line = 0.0;
            }
            placed.push((index, Point::new(x, y)));
            x += size.width + gap;
            line = line.max(size.height);
        }
        placed
    }
}

impl Widget for Wrap {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for child in &mut self.children {
            ctx.register_child(child);
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let sizes = self.sizes(ctx);
        match axis {
            Axis::Horizontal => match len_req {
                LenReq::FitContent(space) => space,
                LenReq::MinContent => Length::px(sizes.iter().map(|s| s.width).fold(0.0, f64::max)),
                LenReq::MaxContent => {
                    Length::px(sizes.iter().map(|s| s.width).sum::<f64>() + self.gap * sizes.len().saturating_sub(1) as f64)
                }
            },
            Axis::Vertical => {
                let width = cross.map_or(f64::INFINITY, |c| c.get());
                let placed = Self::rows(&sizes, width, self.gap);
                let bottom = placed.iter().map(|(i, p)| p.y + sizes[*i].height).fold(0.0, f64::max);
                Length::px(bottom)
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let mut sizes = Vec::with_capacity(self.children.len());
        for child in &mut self.children {
            let auto = SizeDef::new(LenDef::MaxContent, LenDef::MaxContent);
            sizes.push(ctx.compute_size(child, auto, LayoutSize::NONE));
        }
        for (index, origin) in Self::rows(&sizes, size.width, self.gap) {
            let child = &mut self.children[index];
            ctx.run_layout(child, sizes[index]);
            ctx.place_child(child, origin);
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        self.children.iter().map(WidgetPod::id).collect()
    }
}

pub struct Underlined {
    child: WidgetPod<dyn Widget>,
    base: fn() -> Color,
    line: fn() -> Color,
}

impl Underlined {
    pub fn new(child: NewWidget<impl Widget + ?Sized>, base: fn() -> Color, line: fn() -> Color) -> NewWidget<Self> {
        NewWidget::new(Underlined { child: child.erased().to_pod(), base, line })
    }
}

impl Widget for Underlined {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let context = masonry::layout::LayoutSize::maybe(axis.cross(), cross);
        let inner = ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross);
        if axis == Axis::Vertical { Length::px(inner.get() + size::BORDER_1) } else { inner }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let inner = Size::new(size.width, (size.height - size::BORDER_1).max(0.0));
        ctx.run_layout(&mut self.child, inner);
        ctx.place_child(&mut self.child, Point::ZERO);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let bounds = ctx.content_box();
        painter.fill(bounds, (self.base)()).draw();
        painter.fill(Rect::new(bounds.x0, bounds.y1 - size::BORDER_1, bounds.x1, bounds.y1), (self.line)()).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
