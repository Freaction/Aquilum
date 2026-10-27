mod actions;
mod load;
mod panel;

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use masonry::app::RenderRoot;
use masonry::core::WidgetId;

use super::App;
#[cfg(test)]
use super::Popup;
use crate::graph::Graph;
use crate::i18n::t;
use crate::ui::graph::{Display, GraphView, Heat, Status};
use crate::ui::handle::Handle;
use crate::ui::widgets::Slot;

struct Controls {
    reset: WidgetId,
    toggle: WidgetId,
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
            panel: prefs.panel,
        }
    }

    fn push_display(&self, root: &mut RenderRoot) {
        let display = self.graph_display();
        if let Some(view) = self.graph.view {
            view.edit(root, |mut v| GraphView::set_display(&mut v, display));
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
        if self.graph_active() || (self.graph.view.is_none() && self.graph.data.is_none()) {
            return;
        }
        self.graph = GraphState { generation: self.graph.generation + 1, ..GraphState::default() };
        if let Some(chrome) = &self.chrome {
            chrome.graph_page.edit(root, |mut slot| Slot::clear(&mut slot));
        }
    }

    pub(super) fn drop_graph_frames(&mut self, root: &mut RenderRoot) {
        if let Some(view) = self.graph.view {
            view.edit(root, |mut view| GraphView::drop_frames(&mut view));
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

    #[cfg(test)]
    pub(super) fn graph_loaded(&self) -> Option<usize> {
        self.graph.data.as_ref().map(|g| g.len())
    }

    #[cfg(test)]
    pub(super) fn graph_released(&self) -> bool {
        self.graph.data.is_none() && self.graph.view.is_none()
    }

    #[cfg(test)]
    pub(super) fn graph_reset_button(&self) -> Option<WidgetId> {
        self.graph.controls.as_ref().map(|c| c.reset)
    }

    #[cfg(test)]
    pub(super) fn reset_graph_dialog_layer(&self) -> Option<WidgetId> {
        match &self.popup {
            Some(Popup::ResetGraph(dialog)) => Some(dialog.layer),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(super) fn graph_share(&self) -> f64 {
        self.graph.share
    }

    #[cfg(test)]
    pub(super) fn set_graph_share(&mut self, share: f64) {
        self.graph.share = share;
    }

    #[cfg(not(feature = "installed"))]
    pub fn graph_view(&self) -> Option<WidgetId> {
        self.graph.view.map(|v| v.id())
    }

    #[cfg(not(feature = "installed"))]
    pub fn graph_ready(&self) -> bool {
        self.graph_active() && self.graph.data.is_some()
    }

    #[cfg(not(feature = "installed"))]
    pub fn open_graph_for_replay(&mut self, root: &mut RenderRoot) {
        if !self.graph_active() {
            self.open_graph(root);
        }
    }
}
