use std::collections::HashMap;
use std::sync::Arc;

use masonry::core::{NewWidget, Widget, WidgetMut, WidgetPod};
use masonry::kurbo::{Rect, Size};

use crate::ui::text::Line;
use super::{Display, GraphView, Status};
use crate::graph::Graph;
use crate::graph::camera::Camera;

impl GraphView {
    pub fn new(graph: Option<Arc<Graph>>, status: Status, display: Display, camera: Option<(f64, f64, f64)>, settings: Option<NewWidget<dyn Widget>>) -> NewWidget<Self> {
        NewWidget::new(Self::build(graph, status, display, camera, settings.map(NewWidget::to_pod)))
    }

    pub(super) fn build(graph: Option<Arc<Graph>>, status: Status, display: Display, camera: Option<(f64, f64, f64)>, settings: Option<WidgetPod<dyn Widget>>) -> Self {
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
            action_rect: None,
            action_line: Line::default(),
            settings,
            panel: Rect::ZERO,
            animating: false,
            scale_factor: 1.0,
            surfaces: crate::render::Surfaces::default(),
            draft: false,
            cost: std::time::Duration::ZERO,
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

    #[cfg(test)]
    pub fn retained(&self) -> (usize, usize) {
        (self.sprites.len(), self.surfaces.pool_len())
    }

    pub fn drop_frames(this: &mut WidgetMut<'_, Self>) {
        this.widget.surfaces = crate::render::Surfaces::default();
        this.widget.sprites.clear();
        this.ctx.request_paint_only();
        this.ctx.request_anim_frame();
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
}
