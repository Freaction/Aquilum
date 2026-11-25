use masonry::core::{LayoutCtx, MeasureCtx, NewWidget, WidgetPod};
use masonry::kurbo::{Axis, Point, Rect, Size};
use masonry::peniko::Color;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::Flex;

use super::item::MenuItem;
use crate::ui::tokens::size;
use crate::ui::widgets::{MaxContent, spaced};

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
    spaced(column, size::GAP_XS, size::PADDING_2XS)
}

impl Sub {
    pub(super) fn measure(&mut self, ctx: &mut MeasureCtx<'_>) -> Size {
        let width = ctx.max_content(&mut self.panel, Axis::Horizontal, None);
        let height = ctx.max_content(&mut self.panel, Axis::Vertical, Some(width));
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
