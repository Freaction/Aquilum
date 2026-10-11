mod slider;
mod switch;
mod segmented;
mod buttons;
mod input;
mod picker;

pub use slider::*;
pub use switch::*;
pub use segmented::*;
pub use buttons::*;
pub use input::*;
pub use picker::{ColorPicker, Picked};

use masonry::accesskit::{Action, Node, Role, Toggled};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx,
    PointerButton, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget,
    WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Circle, Point, Rect, RoundedRect, Size, Stroke};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};

use crate::ui::icons;
use crate::ui::menu::box_shadow;
use crate::ui::tokens::{Shadow, size};
use crate::ui::widgets::Press;
use crate::ui::{text, theme};

#[derive(Clone, Debug, PartialEq)]
pub enum Changed {
    Bool(bool),
    Choice(usize),
    OpenMenu(Rect),
    OpenColor(Rect),
    Value(f64),
}

pub(super) fn ring(painter: &mut Painter<'_>, shape: RoundedRect, color: masonry::peniko::Color) {
    let outer = RoundedRect::from_rect(shape.rect().inflate(0.5, 0.5), shape.radii().top_left + 0.5);
    painter.stroke(&outer, &Stroke::new(1.0), color).draw();
}
