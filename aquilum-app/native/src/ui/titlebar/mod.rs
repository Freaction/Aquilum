use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerButton,
    PointerEvent, PointerScrollEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update,
    UpdateCtx, Widget, WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use crate::ui::editor::TextEditor;
use masonry::kurbo::{Affine, Arc, Axis, BezPath, Insets, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};

use uuid::Uuid;

use super::handle::Handle;
use super::icons;
use super::keymap::pointer;
use super::widgets::{Press, frame_line, paint_glyph};
use super::tokens::{number, size};
use super::{text, theme};

mod rename;
mod shape;
mod strip;
mod tab;
#[cfg(test)]
mod tests;
mod window;

pub use rename::*;
use shape::*;
pub use strip::*;
pub use tab::*;
pub use window::*;

#[derive(Clone, Debug, PartialEq)]
pub enum TitlebarAction {
    Select(Uuid),
    Close(Uuid),
    Menu(Uuid, Point),
    Reorder(usize, usize),
    Drag,
    ToggleMaximize,
    Minimize,
    CloseWindow,
}

const CLOSE_SIZE: f64 = size::BUTTON_ICON_SIZE + size::PADDING_4XS * 2.0;
const EDGE: f64 = 48.0;
const EDGE_STEP: f64 = 14.0;

pub(super) fn drag_or_maximize(ctx: &mut EventCtx<'_>, event: &PointerEvent) {
    if WINDOW_CHROME
        && let PointerEvent::Down(e) = event
        && pointer::press(e.button)
    {
        let action = if e.state.count >= 2 { TitlebarAction::ToggleMaximize } else { TitlebarAction::Drag };
        ctx.submit_action::<TitlebarAction>(action);
    }
}
