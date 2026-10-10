use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, NewWidget, PropertySet, Widget, WidgetId};
use masonry::layout::Length;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Background, BorderColor, BorderWidth, CornerRadius, Gap, Padding};
use masonry::widgets::{Flex, SizedBox};

use super::App;
use crate::graph::{self, Graph};
use crate::i18n::t;
use crate::interface::GraphPrefs;
use crate::ui::graph::{Display, GraphAction, GraphView, Heat, Status};
use crate::ui::handle::Handle;
use crate::ui::settings::controls::{Changed, Segmented, Slider, Switch};
use crate::ui::text::label;
use crate::ui::tokens::{number, size};
use crate::ui::widgets::{IconButton, Pressed, Slot};
use crate::ui::{icons, theme};

struct Controls {
    refresh: WidgetId,
    node_size: WidgetId,
    spread: WidgetId,
    depth: WidgetId,
    created: WidgetId,
    labels: WidgetId,
    heat: WidgetId,
}

pub(super) struct GraphState {
    data: Option<Arc<Graph>>,
    failed: Option<String>,
    loading: bool,
    stale: bool,
    generation: u64,
    share: f64,
    tx: Sender<(u64, Result<Graph, String>)>,
    rx: Receiver<(u64, Result<Graph, String>)>,
    view: Option<Handle<GraphView>>,
    controls: Option<Controls>,
}

impl Default for GraphState {
    fn default() -> Self {
        let (tx, rx) = channel();
        GraphState { data: None, failed: None, loading: false, stale: false, generation: 0, share: 0.0, tx, rx, view: None, controls: None }
    }
}

const HEATS: [Heat; 3] = [Heat::None, Heat::Modified, Heat::Created];

fn depth_hint(depth: usize) -> String {
    t(&format!("graph.depth{}", depth.clamp(1, 3)))
}

fn day_hint(day: f64) -> String {
    if !day.is_finite() || day <= 0.0 {
        return t("graph.all");
    }
    chrono::DateTime::from_timestamp((day * 86_400.0) as i64, 0).map_or_else(|| t("graph.all"), |d| d.with_timezone(&chrono::Local).format("%d.%m.%y").to_string())
}

impl App {
    fn graph_display(&self) -> Display {
        let prefs = self.interface.graph();
        let range = self.graph.data.as_ref().map(|g| g.created_range).unwrap_or_default();
        let share = self.graph.share;
        Display {
            node_size: prefs.node_size,
            spread: prefs.spread,
            depth: prefs.highlight_depth.clamp(1, 3),
            labels: prefs.labels,
            heat: HEATS[usize::from(prefs.heatmap_axis).min(2)],
            created_from: if share == 0.0 { f64::NEG_INFINITY } else { range.oldest + (range.newest - range.oldest) * share },
        }
    }

    fn created_hint(&self) -> String {
        let display = self.graph_display();
        if self.graph.share == 0.0 { t("graph.all") } else { day_hint(display.created_from) }
    }

    fn graph_status(&self) -> Status {
        if self.tree.root().is_none() {
            Status::NoWorkspace
        } else if self.graph.data.is_some() {
            Status::Ready
        } else if let Some(error) = &self.graph.failed {
            Status::Failed(error.clone())
        } else {
            Status::Loading
        }
    }

    pub(super) fn graph_widget(&mut self) -> Option<NewWidget<dyn Widget>> {
        if !self.tabs.has_graph() {
            self.graph.view = None;
            self.graph.controls = None;
            return None;
        }
        let tm = theme::current();
        let prefs = self.interface.graph();
        let line = (theme::ui_font_settings().size * number::FONT_LINE_HEIGHT_NORMAL as f32).round();
        let title = label(&t("graph.title"), tm.text_primary, Some(number::FONT_WEIGHT_UI_STRONG as f32), line);
        let refresh = NewWidget::new(IconButton::new(icons::ROTATE_CCW, t("graph.refresh")));
        let head = Flex::row().main_axis_alignment(MainAxisAlignment::SpaceBetween).cross_axis_alignment(CrossAxisAlignment::Center);
        let refresh_id = refresh.id();
        let head = head.with(title, 1.0).with_fixed(refresh);
        let node_size = Slider::new(&t("graph.nodeSize"), prefs.node_size, 0.4, 2.5, 0.05, &format!("{:.2}×", prefs.node_size));
        let spread = Slider::new(&t("graph.spread"), prefs.spread, graph::MIN_SPREAD, 2.5, 0.05, &format!("{:.2}×", prefs.spread));
        let depth = Slider::new(&t("graph.depth"), prefs.highlight_depth as f64, 1.0, 3.0, 1.0, &depth_hint(prefs.highlight_depth));
        let created = Slider::new(&t("graph.createdAfter"), self.graph.share, 0.0, 1.0, 0.005, &self.created_hint());
        let labels = Switch::new(prefs.labels, &t("graph.labels"));
        let toggle = Flex::row()
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .with(label(&t("graph.labels"), tm.text_secondary, None, line), 1.0);
        let labels_id = labels.id();
        let toggle = toggle.with_fixed(labels);
        let options = vec![t("graph.heatNone"), t("graph.heatModified"), t("graph.heatCreated")];
        let heat = Segmented::stretched(options, usize::from(prefs.heatmap_axis).min(2));
        let heat_id = heat.id();
        let choice = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(label(&t("graph.heatmap"), tm.text_secondary, None, line))
            .with_fixed(heat);
        let choice = NewWidget::new(choice).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_8))));
        let controls = Controls {
            refresh: refresh_id,
            node_size: node_size.id(),
            spread: spread.id(),
            depth: depth.id(),
            created: created.id(),
            labels: labels_id,
            heat: heat_id,
        };
        let panel = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(NewWidget::new(head))
            .with_fixed(node_size)
            .with_fixed(spread)
            .with_fixed(depth)
            .with_fixed(created)
            .with_fixed(NewWidget::new(toggle))
            .with_fixed(choice);
        let panel = NewWidget::new(panel).with_props(
            PropertySet::new()
                .with(Background::Color(tm.graph_settings_bg))
                .with(BorderColor { color: tm.sidebar_border })
                .with(BorderWidth { width: Length::px(size::BORDER_1) })
                .with(CornerRadius { radius: Length::px(size::RADIUS_2XL) })
                .with(Padding::all(Length::px(size::SPACE_16)))
                .with(Gap::new(Length::px(size::SPACE_16))),
        );
        let camera = self.session.as_ref().and_then(|s| s.camera);
        let view = GraphView::new(self.graph.data.clone(), self.graph_status(), self.graph_display(), camera, Some(panel.erased()));
        self.graph.view = Some(Handle::of(&view));
        self.graph.controls = Some(controls);
        Some(view.erased())
    }

    pub(super) fn show_graph(&mut self, root: &mut RenderRoot) {
        if self.graph.view.is_none()
            && let Some(widget) = self.graph_widget()
            && let Some(chrome) = &self.chrome
        {
            chrome.graph_page.edit(root, |mut slot| Slot::set_child(&mut slot, widget));
        }
        if self.graph.data.is_none() || self.graph.stale {
            self.load_graph();
        }
    }

    pub(super) fn release_graph(&mut self, root: &mut RenderRoot) {
        if self.tabs.has_graph() || (self.graph.view.is_none() && self.graph.data.is_none()) {
            return;
        }
        self.graph.view = None;
        self.graph.controls = None;
        self.graph.data = None;
        self.graph.failed = None;
        self.graph.stale = false;
        self.graph.share = 0.0;
        self.graph.generation += 1;
        if let Some(chrome) = &self.chrome {
            chrome.graph_page.edit(root, |mut slot| Slot::set_child(&mut slot, NewWidget::new(SizedBox::empty()).erased()));
        }
    }

    pub(super) fn open_graph(&mut self, root: &mut RenderRoot) {
        self.tabs.open_graph();
        self.show_active(root);
    }

    pub(super) fn graph_active(&self) -> bool {
        self.tabs.active().graph
    }

    pub fn graph_shown(&self) -> bool {
        self.graph_active() && self.popup.is_none() && self.graph.view.is_some()
    }

    fn load_graph(&mut self) {
        let Some(workspace) = self.tree.root().map(PathBuf::from) else { return };
        if self.graph.loading {
            self.graph.stale = true;
            return;
        }
        self.graph.loading = true;
        self.graph.stale = false;
        let generation = self.graph.generation;
        let core = Arc::clone(&self.workspace.core);
        let tx = self.graph.tx.clone();
        let wake = Arc::clone(&self.workspace.wake);
        std::thread::spawn(move || {
            let result = graph::load(&core, &workspace);
            if tx.send((generation, result)).is_ok() {
                wake();
            }
        });
    }

    pub(super) fn graph_links_changed(&mut self) {
        if !self.tabs.has_graph() {
            return;
        }
        if self.graph_active() {
            self.load_graph();
        } else {
            self.graph.stale = true;
        }
    }

    #[cfg(test)]
    pub(super) fn graph_loaded(&self) -> Option<usize> {
        self.graph.data.as_ref().map(|g| g.len())
    }

    #[cfg(test)]
    pub(super) fn graph_released(&self) -> bool {
        self.graph.data.is_none() && self.graph.view.is_none()
    }

    pub fn graph_view(&self) -> Option<WidgetId> {
        self.graph.view.map(|v| v.id())
    }

    pub fn graph_ready(&self) -> bool {
        self.graph_active() && self.graph.data.is_some()
    }

    pub fn open_graph_for_replay(&mut self, root: &mut RenderRoot) {
        if !self.graph_active() {
            self.open_graph(root);
        }
    }

    pub(super) fn on_graph_results(&mut self, root: &mut RenderRoot) {
        while let Ok((generation, result)) = self.graph.rx.try_recv() {
            self.graph.loading = false;
            if generation != self.graph.generation || !self.tabs.has_graph() {
                continue;
            }
            match result {
                Ok(graph) => {
                    let keep = self.graph.data.is_some();
                    let graph = Arc::new(graph);
                    self.graph.data = Some(Arc::clone(&graph));
                    self.graph.failed = None;
                    let display = self.graph_display();
                    if let Some(view) = self.graph.view {
                        view.edit(root, |mut v| {
                            GraphView::set_display(&mut v, display);
                            GraphView::set_graph(&mut v, graph, keep);
                        });
                    }
                    self.refresh_created_hint(root);
                }
                Err(error) => {
                    if self.graph.data.is_none() {
                        self.graph.failed = Some(error);
                        let status = self.graph_status();
                        if let Some(view) = self.graph.view {
                            view.edit(root, |mut v| GraphView::set_status(&mut v, status));
                        }
                    }
                }
            }
            if self.graph.stale && self.graph_active() {
                self.load_graph();
            }
        }
    }

    fn refresh_created_hint(&mut self, root: &mut RenderRoot) {
        let hint = self.created_hint();
        let share = self.graph.share;
        if let Some(id) = self.graph.controls.as_ref().map(|c| c.created) {
            root.edit_widget(id, |mut w| Slider::set(&mut w.downcast(), share, &hint));
        }
    }

    pub(super) fn is_graph_widget(&self, id: WidgetId) -> bool {
        let Some(c) = &self.graph.controls else { return false };
        self.graph.view.is_some_and(|v| v.id() == id) || [c.refresh, c.node_size, c.spread, c.depth, c.created, c.labels, c.heat].contains(&id)
    }

    pub(super) fn on_graph_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> bool {
        if let Some(action) = action.downcast_ref::<GraphAction>() {
            match action {
                GraphAction::Open(path) => return self.open_in(root, PathBuf::from(path), true),
                GraphAction::Camera(x, y, scale) => {
                    if let Some(session) = &mut self.session {
                        session.save_camera(&self.workspace.core, (*x, *y, *scale));
                    }
                }
            }
            return false;
        }
        let Some(c) = &self.graph.controls else { return false };
        let (refresh, node_size, spread, depth, created, labels, heat) = (c.refresh, c.node_size, c.spread, c.depth, c.created, c.labels, c.heat);
        if id == refresh && action.downcast_ref::<Pressed>().is_some() {
            self.load_graph();
            return false;
        }
        let Some(changed) = action.downcast_ref::<Changed>() else { return false };
        let mut prefs: GraphPrefs = self.interface.graph();
        let mut hint = None;
        match changed {
            Changed::Value(v) if id == node_size => {
                prefs.node_size = *v;
                hint = Some(format!("{v:.2}×"));
            }
            Changed::Value(v) if id == spread => {
                prefs.spread = *v;
                hint = Some(format!("{v:.2}×"));
            }
            Changed::Value(v) if id == depth => {
                prefs.highlight_depth = v.round() as usize;
                hint = Some(depth_hint(prefs.highlight_depth));
            }
            Changed::Value(v) if id == created => self.graph.share = *v,
            Changed::Bool(on) if id == labels => prefs.labels = *on,
            Changed::Choice(index) if id == heat => prefs.heatmap_axis = (*index).min(2) as u8,
            _ => return false,
        }
        self.interface.set_graph(prefs);
        if let (Some(hint), Changed::Value(v)) = (hint, changed) {
            root.edit_widget(id, |mut w| Slider::set(&mut w.downcast(), *v, &hint));
        }
        if id == created {
            self.refresh_created_hint(root);
        }
        let display = self.graph_display();
        if let Some(view) = self.graph.view {
            view.edit(root, |mut v| GraphView::set_display(&mut v, display));
        }
        false
    }
}
