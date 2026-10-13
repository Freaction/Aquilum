use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::{Code, Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PointerScrollEvent, PropertiesMut, PropertiesRef,
    RegisterCtx, ScrollDelta, TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Line as Segment, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};

use super::icons;
use super::menu::box_shadow;
use super::text::{Line, Style};
use super::theme;
use super::tokens::{number, size};
use super::widgets::{ButtonSize, IconButton, TextButton, Variant};
use crate::i18n::t;
use crate::reader::page::{Item, Settings};
use crate::reader::session::{Session, Spot};

mod arrange;
mod input;
mod motion;
mod paint;
mod view;
mod widget;

const POPOVER_WIDTH: f64 = 352.0;
const PROGRESS: f64 = 3.0;
const WHEEL_STEP: f64 = 120.0;
const TOOLBAR_WIDTH: f64 = 576.0;
const MARK_SCALE: f64 = 0.6;

#[derive(Debug)]
pub enum ReaderAction {
    Close,
    Settings,
    Moved { cfi: String, fraction: f64 },
    QuoteRef(String),
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Prev,
    Next,
}

pub struct ReaderView {
    intro: crate::ui::widgets::Intro,
    session: Session,
    settings: Settings,
    title: String,
    total: u32,
    fraction: f64,
    report: bool,
    wheel: f64,
    settings_button: WidgetPod<IconButton>,
    close: WidgetPod<IconButton>,
    popover: Option<WidgetPod<dyn Widget>>,
    title_line: Line,
    pages_line: Line,
    header: f64,
    body: Rect,
    popover_rect: Rect,
    hover: Option<Option<Side>>,
    zones: [crate::ui::widgets::Tween; 2],
    bar: crate::ui::widgets::Tween,
    selection: Option<(Spot, Spot)>,
    selecting: bool,
    quotes: Vec<(String, String)>,
    marks: Option<(usize, Vec<(Spot, Spot, String, String)>)>,
    active: Option<String>,
    hovered: Option<usize>,
    quote_rects: Vec<(String, Vec<Rect>)>,
    labels: Vec<(Rect, String)>,
    quote_button: Option<WidgetPod<TextButton>>,
    toolbar: Rect,
    preview: Rect,
    preview_line: Line,
    mark_lines: Vec<Line>,
}

pub struct ReaderIds {
    pub settings: WidgetId,
    pub close: WidgetId,
    pub quote: Option<WidgetId>,
}

fn nav_width(width: f64) -> f64 {
    (width * 0.08).clamp(40.0, 72.0)
}
