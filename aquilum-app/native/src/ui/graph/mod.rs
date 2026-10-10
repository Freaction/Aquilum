use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, BrushIndex, ChildrenIds, CursorIcon, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerButton, PointerEvent, PointerScrollEvent,
    PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, ScrollDelta, Update, UpdateCtx, Widget, WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Vec2};
use masonry::layout::{LenReq, Length};
use masonry::parley::{FontContext, FontWeight, Layout, LayoutContext, StyleProperty};
use masonry::peniko::{Color, ImageBrush};

use super::text::{Line, Style};
use super::theme;
use super::tokens::{number, size};
use crate::graph::camera::{Camera, approach};
use crate::graph::raster::{self, Disc, Edge, Rgb, Sprite, Stamp};
use crate::graph::{self, Graph};
use crate::i18n::{plural, t, t_with};

mod animate;
mod input;
mod labels;
mod overlay;
mod scene;
#[cfg(test)]
mod tests;

use labels::Label;

const DRAG_THRESHOLD: f64 = 2.0;

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
    settings: Option<WidgetPod<dyn Widget>>,
    panel: Rect,
    animating: bool,
    scale_factor: f64,
    surfaces: crate::render::Surfaces,
    draft: bool,
    cost: Duration,
    screen: Vec<[f32; 2]>,
}

impl GraphView {
    pub fn new(graph: Option<Arc<Graph>>, status: Status, display: Display, camera: Option<(f64, f64, f64)>, settings: Option<NewWidget<dyn Widget>>) -> NewWidget<Self> {
        NewWidget::new(Self::build(graph, status, display, camera, settings.map(NewWidget::to_pod)))
    }

    fn build(graph: Option<Arc<Graph>>, status: Status, display: Display, camera: Option<(f64, f64, f64)>, settings: Option<WidgetPod<dyn Widget>>) -> Self {
        let mut view = GraphView {
            graph: None,
            positions: Vec::new(),
            morph: None,
            camera: Camera::default(),
            stored: camera,
            display,
            status,
            size: Size::ZERO,
            fitted: false,
            marks: Vec::new(),
            intensity: Vec::new(),
            lit: Vec::new(),
            moving: Vec::new(),
            dimming: 0.0,
            dimming_target: 0.0,
            reveal: Vec::new(),
            revealing: false,
            visible: 0,
            hovered: None,
            pointer: None,
            drag: None,
            fades: HashMap::new(),
            shown_labels: Vec::new(),
            labels: HashMap::new(),
            sprites: HashMap::new(),
            sprite_scale: 0.0,
            moved_at: None,
            status_lines: Vec::new(),
            notice: [Line::default(), Line::default()],
            settings,
            panel: Rect::ZERO,
            animating: false,
            scale_factor: 1.0,
            surfaces: crate::render::Surfaces::default(),
            draft: false,
            cost: Duration::ZERO,
            screen: Vec::new(),
        };
        if let Some(graph) = graph {
            view.adopt(graph, false);
        }
        view
    }

    fn adopt(&mut self, graph: Arc<Graph>, keep: bool) {
        let morphs = keep && self.graph.as_ref().is_some_and(|g| g.len() == graph.len()) && graph.len() <= 100_000;
        let previous = std::mem::take(&mut self.positions);
        self.positions = if morphs { previous.clone() } else { graph.positions.clone() };
        self.morph = morphs.then_some((previous, 0.0));
        let count = graph.len();
        self.marks = vec![0; count];
        self.intensity = vec![0.0; count];
        self.lit.clear();
        self.moving.clear();
        self.dimming = 0.0;
        self.dimming_target = 0.0;
        self.hovered = None;
        self.fades.clear();
        self.shown_labels.clear();
        self.labels.clear();
        self.sprites.clear();
        self.reveal = (0..count).map(|n| if self.shown(&graph, n) { 1.0 } else { 0.0 }).collect();
        self.visible = self.reveal.iter().filter(|&&r| r > 0.0).count();
        self.revealing = false;
        self.fitted = keep && self.fitted;
        self.graph = Some(graph);
        self.status = Status::Ready;
        self.animating = true;
    }

    pub fn set_graph(this: &mut WidgetMut<'_, Self>, graph: Arc<Graph>, keep: bool) {
        this.widget.adopt(graph, keep);
        this.ctx.request_layout();
        this.ctx.request_anim_frame();
    }

    pub fn set_status(this: &mut WidgetMut<'_, Self>, status: Status) {
        this.widget.status = status;
        this.ctx.request_layout();
    }

    pub fn set_display(this: &mut WidgetMut<'_, Self>, display: Display) {
        let current = this.widget.display;
        if current == display {
            return;
        }
        this.widget.display = display;
        if display.spread != current.spread {
            this.widget.camera.rescale_world(display.spread / current.spread);
        }
        if display.created_from != current.created_from
            && let Some(graph) = this.widget.graph.clone()
        {
            this.widget.visible = (0..graph.len()).filter(|&n| this.widget.shown(&graph, n)).count();
            this.widget.revealing = true;
        }
        if display.depth != current.depth {
            this.widget.relight();
        }
        if !display.labels {
            this.widget.fades.clear();
            this.widget.shown_labels.clear();
        }
        this.widget.animating = true;
        this.ctx.request_anim_frame();
    }

    fn shown(&self, graph: &Graph, node: usize) -> bool {
        !(f64::from(graph.created[node]) < self.display.created_from)
    }

    fn bounds(&self) -> graph::Bounds {
        let b = self.graph.as_ref().map(|g| g.bounds).unwrap_or_default();
        let s = self.display.spread;
        graph::Bounds { min_x: b.min_x * s, min_y: b.min_y * s, max_x: b.max_x * s, max_y: b.max_y * s }
    }

    fn glide(&mut self) {
        if self.graph.is_some() {
            self.camera.glide_to(self.bounds(), self.size.width, self.size.height);
            self.moved();
        }
    }

    fn screen(&self, node: usize) -> Point {
        let [x, y] = self.positions[node];
        let s = self.display.spread;
        Point::new(
            (f64::from(x) * s - self.camera.center_x) * self.camera.scale + self.size.width / 2.0,
            self.size.height / 2.0 - (f64::from(y) * s - self.camera.center_y) * self.camera.scale,
        )
    }

    fn radius(&self, graph: &Graph, node: usize) -> f64 {
        graph::radius_pixels(graph.degrees[node], self.camera.scale, self.display.node_size, self.display.spread)
    }

    fn node_at(&self, p: Point) -> Option<usize> {
        let graph = self.graph.as_ref()?;
        let (x, y) = self.camera.to_world(p.x - self.size.width / 2.0, p.y - self.size.height / 2.0);
        graph.pick(x, y, self.camera.scale, self.display.node_size, self.display.spread, |n| self.shown(graph, n))
    }

    fn relight(&mut self) {
        let Some(graph) = self.graph.clone() else { return };
        for node in std::mem::take(&mut self.lit) {
            self.marks[node] = 0;
            self.moving.push(node);
        }
        self.dimming_target = if self.hovered.is_some() { 1.0 } else { 0.0 };
        if let Some(start) = self.hovered {
            let (marks, lit, moving) = (&mut self.marks, &mut self.lit, &mut self.moving);
            graph.levels(start, self.display.depth, |node, level| {
                marks[node] = level as u8 + 1;
                lit.push(node);
                moving.push(node);
            });
        }
        self.moving.sort_unstable();
        self.moving.dedup();
        self.animating = true;
    }

    fn hover(&mut self, node: Option<usize>) -> bool {
        if node == self.hovered {
            return false;
        }
        self.hovered = node;
        self.relight();
        true
    }

    fn moved(&mut self) {
        self.moved_at = Some(Instant::now());
        self.animating = true;
    }

}

impl Widget for GraphView {
    type Action = GraphAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        self.pointer(ctx, event);
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let now = Instant::now();
        let seconds = interval as f64 / 1e9;
        self.scale_factor = ctx.scale_factor();
        let mut busy = self.step(seconds);
        let (fcx, lcx) = ctx.text_contexts();
        busy |= self.step_labels(seconds, fcx, lcx);
        ctx.request_paint_only();
        if let Some(at) = self.moved_at
            && now.duration_since(at) >= CAMERA_SETTLE
        {
            self.moved_at = None;
            let s = self.display.spread;
            ctx.submit_action::<GraphAction>(GraphAction::Camera(self.camera.center_x / s, self.camera.center_y / s, self.camera.scale));
        }
        if let (Some(p), None, None) = (self.pointer, &self.drag, &self.morph) {
            let node = self.node_at(p);
            busy |= self.hover(node);
        }
        let moving = busy || self.drag.as_ref().is_some_and(|d| d.travelled > DRAG_THRESHOLD);
        self.draft = moving && (self.draft || self.cost > DRAFT_BUDGET);
        if busy || self.moved_at.is_some() || std::mem::take(&mut self.animating) {
            ctx.request_anim_frame();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::FontsChanged) {
            self.labels.clear();
            self.sprites.clear();
            self.status_lines.clear();
            self.notice = [Line::default(), Line::default()];
            ctx.request_anim_frame();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(settings) = &mut self.settings {
            ctx.register_child(settings);
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let resized = self.size != size;
        self.size = size;
        ctx.set_clip_path(size.to_rect());
        if self.graph.is_some() && size.width > 0.0 && size.height > 0.0 {
            if let Some((x, y, scale)) = self.stored.take() {
                let s = self.display.spread;
                self.camera.move_to(x * s, y * s, scale);
                self.fitted = true;
            } else if !self.fitted {
                self.camera.jump_to(self.bounds(), size.width, size.height);
                self.fitted = true;
            }
            if resized {
                let (fcx, lcx) = ctx.text_contexts();
                self.step_labels(0.0, fcx, lcx);
            }
        }
        if let Some(settings) = &mut self.settings {
            let show = !matches!(self.status, Status::Failed(_) | Status::NoWorkspace);
            ctx.set_stashed(settings, !show);
            self.panel = Rect::ZERO;
            if show {
                let panel = Rect::new(size.width - size::SPACE_12 - SETTINGS_WIDTH, size::SPACE_12, size.width - size::SPACE_12, (size.height - size::SPACE_12).max(size::SPACE_12));
                ctx.run_layout(settings, panel.size());
                ctx.place_child(settings, panel.origin());
                self.panel = panel;
            }
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let bounds = ctx.content_box();
        match &self.status {
            Status::NoWorkspace => {
                painter.fill(bounds, tm.graph_backdrop).draw();
                return self.paint_notice(ctx, painter, &t("graph.noWorkspace"), None);
            }
            Status::Failed(error) => {
                painter.fill(bounds, tm.graph_backdrop).draw();
                let error = error.clone();
                return self.paint_notice(ctx, painter, &t("graph.notBuilt"), Some(&error));
            }
            Status::Loading if self.graph.is_none() => {
                painter.fill(bounds, tm.graph_backdrop).draw();
                self.pill(ctx, painter, &[t("graph.loading")]);
                return;
            }
            _ => {}
        }
        self.scale_factor = ctx.scale_factor();
        let Some(graph) = self.graph.clone() else { return };
        let started = Instant::now();
        if let Some((image, scale)) = self.scene() {
            painter.draw_image(&image, Affine::scale(1.0 / scale));
        }
        self.cost = started.elapsed();
        self.paint_status(ctx, painter, &graph);
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        if self.drag.as_ref().is_some_and(|d| d.travelled > DRAG_THRESHOLD) {
            CursorIcon::Grabbing
        } else if self.hovered.is_some() {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Canvas
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(t("tabs.graph"));
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&self.settings.iter().map(WidgetPod::id).collect::<Vec<_>>())
    }
}
