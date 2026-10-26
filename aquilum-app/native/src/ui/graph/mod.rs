use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use masonry::core::{BrushIndex, EventCtx, PaintCtx, PointerEvent, PointerScrollEvent, ScrollDelta, Widget, WidgetPod, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Point, Rect, RoundedRect, Size, Vec2};
use masonry::parley::{FontContext, FontWeight, Layout, LayoutContext, StyleProperty};
use masonry::peniko::{Color, ImageBrush};

use super::text::{Line, Style};
use super::theme;
use super::tokens::{number, size};
use super::widgets::{ButtonSize, Look, Variant, paint_button};
use crate::graph::camera::{Camera, approach};
use crate::graph::raster::{self, Sprite, Stamp};
use crate::graph::{self, Graph};
use crate::i18n::{plural, t_with};

mod animate;
mod input;
mod labels;
mod overlay;
mod scene;
mod state;
#[cfg(test)]
mod tests;
mod view;
mod widget;

use labels::Label;

const LABEL_GAP: f64 = 4.0;
const MAX_EDGES: usize = 600_000;
const FADE: f64 = 0.07;
const SETTLED: f64 = 0.02;
const CAMERA_SETTLE: Duration = Duration::from_millis(400);
const SETTINGS_WIDTH: f64 = 240.0;
const DRAFT_BUDGET: Duration = Duration::from_millis(12);
const DRAFT_SCALE: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Heat {
    None,
    Modified,
    Created,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Display {
    pub node_size: f64,
    pub spread: f64,
    pub depth: usize,
    pub labels: bool,
    pub heat: Heat,
    pub created_from: f64,
}

impl Default for Display {
    fn default() -> Self {
        Display { node_size: 1.0, spread: 1.0, depth: 1, labels: true, heat: Heat::None, created_from: f64::NEG_INFINITY }
    }
}

#[derive(Debug)]
pub enum GraphAction {
    Open(String),
    Camera(f64, f64, f64),
    ShowAll,
    Retry,
}

pub enum Status {
    Loading,
    Ready,
    Failed(String),
    NoWorkspace,
}

struct Drag {
    last: Point,
    travelled: f64,
    slop: f64,
}

impl Drag {
    fn panning(&self) -> bool {
        self.travelled > self.slop
    }
}

pub struct GraphView {
    graph: Option<Arc<Graph>>,
    positions: Vec<[f32; 2]>,
    morph: Option<(Vec<[f32; 2]>, f64)>,
    camera: Camera,
    stored: Option<(f64, f64, f64)>,
    display: Display,
    status: Status,
    size: Size,
    fitted: bool,
    marks: Vec<u8>,
    intensity: Vec<f32>,
    lit: Vec<usize>,
    moving: Vec<usize>,
    dimming: f64,
    dimming_target: f64,
    reveal: Vec<f32>,
    revealing: bool,
    visible: usize,
    hovered: Option<usize>,
    pointer: Option<Point>,
    drag: Option<Drag>,
    fades: HashMap<usize, f64>,
    shown_labels: Vec<(usize, f64, bool)>,
    labels: HashMap<usize, Label>,
    sprites: HashMap<usize, Sprite>,
    sprite_scale: f64,
    moved_at: Option<Instant>,
    status_lines: Vec<Line>,
    notice: [Line; 2],
    pub(super) action_rect: Option<Rect>,
    over_action: bool,
    action_line: Line,
    settings: Option<WidgetPod<dyn Widget>>,
    panel: Rect,
    animating: bool,
    scale_factor: f64,
    surfaces: crate::render::Surfaces,
    draft: bool,
    cost: Duration,
    screen: Vec<[f32; 2]>,
}
