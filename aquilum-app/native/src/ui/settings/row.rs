use masonry::accesskit::{Node, Role};
use masonry::core::{AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesRef, RegisterCtx, Widget, WidgetPod};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LayoutSize, LenReq, Length};

use crate::ui::tokens::size;

const CONTROL_SHARE: f64 = 0.4;

#[derive(Clone, Copy)]
enum Part {
    Text,
    Control,
}

struct Arrangement {
    beside: bool,
    text: Size,
    control: Size,
}

impl Arrangement {
    fn new(width: f64, mut length: impl FnMut(Part, Axis, Option<f64>) -> f64) -> Self {
        let natural = length(Part::Control, Axis::Horizontal, None);
        let beside = natural <= width * CONTROL_SHARE;
        let control = if beside { natural } else { width };
        let control = Size::new(control, length(Part::Control, Axis::Vertical, Some(control)));
        let text = if beside { (width - control.width - size::GAP_XL).max(0.0) } else { width };
        let height = length(Part::Text, Axis::Vertical, Some(text));
        let height = if beside { height.max(size::SETTINGS_ROW_TEXT_MIN_HEIGHT) } else { height };
        Arrangement { beside, text: Size::new(text, height), control }
    }

    fn height(&self) -> f64 {
        if self.beside { self.text.height.max(self.control.height) } else { self.text.height + size::GAP_LG + self.control.height }
    }
}

pub struct SettingRow {
    text: WidgetPod<dyn Widget>,
    control: WidgetPod<dyn Widget>,
}

impl SettingRow {
    pub fn new(text: NewWidget<impl Widget + ?Sized>, control: NewWidget<dyn Widget>) -> NewWidget<Self> {
        NewWidget::new(SettingRow { text: text.erased().to_pod(), control: control.to_pod() })
    }

    fn pod(&mut self, part: Part) -> &mut WidgetPod<dyn Widget> {
        match part {
            Part::Text => &mut self.text,
            Part::Control => &mut self.control,
        }
    }
}

fn request(cross: Option<f64>) -> (LayoutSize, Option<Length>) {
    let cross = cross.map(Length::px);
    (LayoutSize::maybe(Axis::Horizontal, cross), cross)
}

impl Widget for SettingRow {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.text);
        ctx.register_child(&mut self.control);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let mut length = |part, axis, cross| {
            let (context, cross) = request(cross);
            ctx.compute_length(self.pod(part), LenReq::MaxContent.into(), context, axis, cross).get()
        };
        if let (Axis::Horizontal, LenReq::FitContent(space)) = (axis, len_req) {
            return space;
        }
        let natural = length(Part::Text, Axis::Horizontal, None) + size::GAP_XL + length(Part::Control, Axis::Horizontal, None);
        match axis {
            Axis::Horizontal => Length::px(natural),
            Axis::Vertical => Length::px(Arrangement::new(cross.map_or(natural, Length::get), length).height()),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let at = Arrangement::new(size.width, |part, axis, cross| {
            let (context, cross) = request(cross);
            ctx.compute_length(self.pod(part), LenReq::MaxContent.into(), context, axis, cross).get()
        });
        let (text, control) = if at.beside {
            let middle = |height: f64| ((size.height - height) / 2.0).max(0.0);
            (Point::new(0.0, middle(at.text.height)), Point::new(size.width - at.control.width, middle(at.control.height)))
        } else {
            (Point::ORIGIN, Point::new(0.0, at.text.height + size::GAP_LG))
        };
        ctx.run_layout(&mut self.text, at.text);
        ctx.place_child(&mut self.text, text);
        ctx.run_layout(&mut self.control, at.control);
        ctx.place_child(&mut self.control, control);
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.text.id(), self.control.id()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arrange(width: f64, control: f64) -> bool {
        Arrangement::new(width, |part, axis, _| match (part, axis) {
            (Part::Control, Axis::Horizontal) => control,
            _ => 20.0,
        })
        .beside
    }

    #[test]
    fn wide_controls_move_under_the_text_on_a_narrow_row() {
        assert!(arrange(320.0, 36.0) && arrange(320.0, 32.0) && arrange(320.0, 90.0), "переключатель, значок и короткая кнопка справа");
        assert!(!arrange(320.0, size::INPUT_WIDTH), "поле ввода под текстом");
        assert!(arrange(640.0, size::INPUT_WIDTH), "на широкой строке поле справа");
    }
}
