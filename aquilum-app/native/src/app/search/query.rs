use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use aquilum_core::Core;
use masonry::app::RenderRoot;

use super::{App, Popup};

pub(in crate::app) struct Found {
    generation: u64,
    response: Result<aquilum_core::search::models::SearchResponse, String>,
}

fn spawn_search(core: Arc<Core>, workspace: PathBuf, query: String, generation: u64, results: Sender<Found>, wake: Arc<dyn Fn() + Send + Sync>) {
    std::thread::spawn(move || {
        let response = core.search.search(&workspace.to_string_lossy(), &query, None).map_err(|e| format!("{e:?}"));
        if results.send(Found { generation, response }).is_ok() {
            wake();
        }
    });
}

impl App {
    pub(in crate::app) fn on_search_results(&mut self, root: &mut RenderRoot) {
        let Some(Popup::Search(state)) = &mut self.popup else {
            let _ = self.search_rx.try_iter().count();
            return;
        };
        let Some(found) = self.search_rx.try_iter().filter(|f| f.generation == state.generation).last() else { return };
        match found.response {
            Ok(response) => {
                state.results = response.results;
                state.terms = response.query_terms;
                state.status = Some(response.status);
                state.failed = false;
            }
            Err(error) => {
                report!("поиск не выполнен: {error}");
                state.failed = true;
            }
        }
        state.selected = state.selected.min(state.results.len().saturating_sub(1));
        self.render_search(root);
    }

    pub(super) fn query_changed(&mut self, root: &mut RenderRoot, query: String) {
        let workspace = self.tree.root().map(|p| p.to_path_buf());
        let core = Arc::clone(&self.workspace.core);
        let wake = Arc::clone(&self.workspace.wake);
        let tx = self.search_tx.clone();
        let Some(Popup::Search(state)) = &mut self.popup else { return };
        state.query = query;
        state.selected = 0;
        state.generation += 1;
        let trimmed = state.query.trim().to_owned();
        if trimmed.is_empty() {
            state.results.clear();
            state.terms.clear();
        } else if let Some(workspace) = workspace {
            spawn_search(core, workspace, trimmed, state.generation, tx, wake);
        }
        self.render_search(root);
    }
}
