use std::path::PathBuf;
use std::sync::Arc;

use masonry::app::RenderRoot;

use super::App;
use crate::graph;
use crate::ui::graph::GraphView;
use crate::ui::settings::controls::Slider;

impl App {
    pub(super) fn load_graph(&mut self) {
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

    pub(in crate::app) fn graph_links_changed(&mut self) {
        if !self.tabs.has_graph() {
            return;
        }
        if self.graph_active() {
            self.load_graph();
        } else {
            self.graph.stale = true;
        }
    }

    pub(in crate::app) fn on_graph_results(&mut self, root: &mut RenderRoot) {
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

    pub(super) fn refresh_created_hint(&mut self, root: &mut RenderRoot) {
        let hint = self.created_hint();
        let share = self.graph.share;
        if let Some(id) = self.graph.controls.as_ref().map(|c| c.created) {
            root.edit_widget(id, |mut w| Slider::set(&mut w.downcast(), share, &hint));
        }
    }
}
