use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerButton,
    PointerEvent, PointerScrollEvent, PropertiesMut, PropertiesRef, RegisterCtx, ScrollDelta, Update,
    UpdateCtx, Widget, WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use crate::ui::editor::TextEditor;
use masonry::kurbo::{Affine, Axis, BezPath, Point, Rect, Size, Stroke, Vec2};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};

use uuid::Uuid;

use super::handle::Handle;
use super::icons;
use super::widgets::Press;
use super::tokens::size;
use super::{text, theme};

mod rename;
mod strip;
mod tab;
mod window;

pub use rename::*;
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

const CLOSE_SIZE: f64 = 24.0;
const DRAG_THRESHOLD: f64 = 4.0;
const EDGE: f64 = 48.0;
const EDGE_STEP: f64 = 14.0;

pub(super) fn drag_or_maximize(ctx: &mut EventCtx<'_>, event: &PointerEvent) {
    if let PointerEvent::Down(e) = event
        && e.button == Some(PointerButton::Primary)
    {
        let action = if e.state.count >= 2 { TitlebarAction::ToggleMaximize } else { TitlebarAction::Drag };
        ctx.submit_action::<TitlebarAction>(action);
    }
}
