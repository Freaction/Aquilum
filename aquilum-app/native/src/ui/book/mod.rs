use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::KeyState;
use masonry::core::{
    AccessCtx, ChildrenIds, CursorIcon, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, PropertySet, QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut, WidgetPod,
};
use masonry::imaging::record::{Scene, replay_transformed};
use masonry::imaging::{ClipRef, Composite, Filter, GroupRef, PaintSink, Painter};
use masonry::kurbo::{Affine, Axis, BezPath, Point, Rect, RoundedRect, Shape, Size, Stroke, Vec2};
use masonry::layout::{LenDef, LenReq, Length, SizeDef};
use masonry::peniko::{BlendMode, Color, ColorStop, Compose, Fill, Gradient, ImageBrush, ImageQuality, Mix};
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::Gap;
use masonry::widgets::Flex;

use super::handle::Handle;
use super::icons;
use super::menu::box_shadow;
use super::theme;
use super::tokens::size;
use super::widgets::{ButtonSize, IconButton, TextButton, Variant};
use crate::cover;
use crate::i18n::t;

mod action_bar;
mod draw;
mod hero_widget;
mod page_cover;
mod star;
mod thumb;

pub use draw::forget as forget_art;
pub use hero_widget::{Hero, hero};
pub use page_cover::PageCover;
pub use star::Star;
pub use thumb::Thumbnail;
use action_bar::ActionBar;
use draw::*;

const CONTENT_GAP: f64 = 56.0;
const CONTROLS_TOP: f64 = 16.0;
const CONTROLS_RIGHT: f64 = 12.0;
const RAYS_WIDTH: f64 = 311.0;

pub enum Visual {
    Pattern(&'static str),
    Image(ImageBrush),
}

pub struct BookView {
    pub cover: ImageBrush,
    pub has_file: bool,
}

pub struct HeroView {
    pub cover: Option<Visual>,
    pub position: f64,
    pub book: Option<BookView>,
    pub viewport: f64,
}

pub struct HeroIds {
    pub cover: Option<Handle<PageCover>>,
    pub replace: Option<WidgetId>,
    pub random: Option<WidgetId>,
    pub reposition: Option<Handle<IconButton>>,
    pub remove: Option<WidgetId>,
    pub thumbnail: Option<Handle<Thumbnail>>,
    pub replace_book: Option<WidgetId>,
    pub book_button: Option<Handle<TextButton>>,
}

#[derive(Debug)]
pub enum ThumbAction {
    Remove,
    Paste,
}

pub fn cover_height(viewport: f64) -> f64 {
    (viewport * 0.207).clamp(186.0, 240.0)
}

fn thumb_height(cover: f64) -> f64 {
    184.0 + (cover - 186.0) * 36.0 / 54.0
}
