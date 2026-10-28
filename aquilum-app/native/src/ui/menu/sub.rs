use masonry::core::{LayoutCtx, MeasureCtx, NewWidget, PropertySet, WidgetPod};
use masonry::kurbo::{Axis, Point, Rect, Size};
use masonry::layout::{LayoutSize, LenReq, Length};
use masonry::peniko::Color;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::item::MenuItem;
use crate::ui::tokens::size;

pub struct Swatch<'a> {
    pub label: &'a str,
    pub color: Option<Color>,
}

pub struct Submenu<'a> {
    pub at: usize,
    pub items: &'a [Swatch<'a>],
    pub checked: usize,
}

pub(super) struct Sub {
    pub(super) top: f64,
    pub(super) panel: WidgetPod<Flex>,
    pub(super) shown: bool,
    pub(super) rect: Rect,
}

pub(super) fn panel(column: Flex) -> NewWidget<Flex> {
    NewWidget::new(column).with_props(PropertySet::new().with(Gap::new(Length::px(size::GAP_XS))).with(Padding::all(Length::px(size::GAP_XS))))
}

impl Sub {
    pub(super) fn measure(&mut self, ctx: &mut MeasureCtx<'_>) -> Size {
        let width = ctx.compute_length(&mut self.panel, LenReq::MaxContent.into(), LayoutSize::NONE, Axis::Horizontal, None);
        let context = LayoutSize::maybe(Axis::Horizontal, Some(width));
        let height = ctx.compute_length(&mut self.panel, LenReq::MaxContent.into(), context, Axis::Vertical, Some(width));
        self.rect = Rect::from_origin_size(self.rect.origin(), Size::new(width.get(), height.get()));
        self.rect.size()
    }

    pub(super) fn place(&mut self, ctx: &mut LayoutCtx<'_>, origin: Point) {
        self.rect = Rect::from_origin_size(origin, self.rect.size());
        ctx.run_layout(&mut self.panel, self.rect.size());
        ctx.place_child(&mut self.panel, origin);
    }

    pub(super) fn new(spec: &Submenu<'_>, top: f64) -> Self {
        let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (index, swatch) in spec.items.iter().enumerate() {
            column = column.with_fixed(NewWidget::new(MenuItem::swatch(index, swatch.label, swatch.color, spec.checked == index)));
        }
        Sub { top, panel: panel(column).to_pod(), shown: false, rect: Rect::ZERO }
    }
}
