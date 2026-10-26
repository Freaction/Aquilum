use std::ops::Range;

use masonry::accesskit::{Node, Role};
pub(super) use masonry::core::keyboard::{Key, KeyState};
pub(super) use masonry::core::Ime;
pub(super) use masonry::imaging::Painter;
pub(super) use masonry::kurbo::{Affine, Axis, BezPath, Cap, Circle, Join, Point, Rect, RoundedRect, Size, Stroke, Vec2};
pub(super) use masonry::parley::{Affinity, BoundingBox, Cursor, Layout, Selection};
pub(super) use masonry::core::BrushIndex;
pub(super) use super::markdown;
pub(super) use super::document::{Fold, Paragraph};
pub(super) use super::markdown::{Block, Callout};
pub(super) use super::history::Kind;
pub(super) use crate::ui::tokens::size;
use masonry::core::{
    AccessCtx, ChildrenIds, ComposeCtx, CursorIcon, EventCtx, LayoutCtx, MeasureCtx, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, StyleProperty, StyleSet, TextEvent, Update,
    UpdateCtx, Widget, WidgetMut,
};
use masonry::layout::{LenReq, Length};
use crate::ui::editor::TextAction;

use super::document::Document;
use super::markdown::Fonts;
use super::history::History;
use crate::ui::theme;

mod api;
mod build;
mod caret;
mod code;
mod edit;
mod find;
mod matching;
pub use find::Status;
pub use matching::Search;
mod format;
mod geometry;
use geometry::{rect, visible};
mod ime;
mod keys;
mod lifecycle;
mod links;
mod media;
mod marks;
mod multicursor;
mod outline;
mod pages;
mod pairs;
mod paste;
mod pointer;
pub use paste::Smart;
mod select;
mod selection_highlight;
mod state;
pub use state::TextEditor;
mod suggest;
mod table;
#[cfg(test)]
mod testing;
mod widget_events;
mod widget_impl;
mod widget_layout;
mod widget_paint;
