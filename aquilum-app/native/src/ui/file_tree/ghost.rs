//! Плашка перетаскивания (`.q-drag-ghost`): акцентный фон, текст на акценте, тень, прозрачность
//! 0.9. Слой над окном, указатель не перехватывает.

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Widget, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, RoundedRect, Size};
use masonry::layout::{LenReq, Length};

use crate::ui::menu::box_shadow;
use crate::ui::tokens::size;
use crate::ui::{text, theme};

/// Смещение плашки от указателя.
pub const OFFSET: f64 = 12.0;
/// Длиннее — обрезается многоточием, как `GHOST_LABEL_LIMIT`.
const LABEL_LIMIT: usize = 25;

pub struct DragGhost {
    label: String,
    line: text::Line,
}

impl DragGhost {
    pub fn new(label: &str) -> NewWidget<Self> {
        let label = if label.chars().count() > LABEL_LIMIT {
            label.chars().take(LABEL_LIMIT).collect::<String>() + "…"
        } else {
            label.to_owned()
        };
        NewWidget::new(DragGhost { label, line: text::Line::default() })
    }

    fn style() -> text::Style {
        text::Style::ui(size::SIDEBAR_ICON_SIZE)
    }
}

impl Widget for DragGhost {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(&mut self, ctx: &mut masonry::core::UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &masonry::core::Update) {
        if matches!(event, masonry::core::Update::FontsChanged) {
            self.line.clear();
            ctx.request_layout();
        }
    }

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        // `padding: 4px 12px`.
        match axis {
            Axis::Horizontal => Length::px(text::measure(ctx, &self.label, Self::style()).ceil() + size::SPACE_12 * 2.0),
            Axis::Vertical => Length::px(Self::style().line_height + size::SPACE_4 * 2.0),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let shape = RoundedRect::from_rect(bounds, size::RADIUS_LG);
        box_shadow(painter, shape, t.elevation_md);
        painter.fill(shape, t.bg_accent.multiply_alpha(0.9)).draw();
        let style = Self::style();
        let layout = self.line.layout(ctx, &self.label, bounds.width(), style);
        let origin = Affine::translate((bounds.x0 + size::SPACE_12, bounds.y0 + size::SPACE_4));
        render_text(painter, origin, layout, &[t.text_on_accent.multiply_alpha(0.9).into()], true);
    }

    fn accessibility_role(&self) -> Role {
        Role::Tooltip
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for DragGhost {
    fn capture_pointer_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &PointerEvent) {}
}
