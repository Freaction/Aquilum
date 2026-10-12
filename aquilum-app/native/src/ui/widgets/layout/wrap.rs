use masonry::accesskit::{Node, Role};
use masonry::core::{AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesRef, RegisterCtx, Widget, WidgetPod};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};

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
