use masonry::app::RenderRoot;

use super::{App, Popup};

impl App {
    pub(in crate::app) fn on_search_results(&mut self, root: &mut RenderRoot) {
        let Some(Popup::Search(state)) = &mut self.popup else {
            let _ = self.search_rx.try_iter().count();
            return;
        };
        let current = state.worker.current();
        let Some(found) = self.search_rx.try_iter().filter(|f| f.generation == current).last() else { return };
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
        state.selected = 0;
        self.render_search(root);
    }

    pub(super) fn query_changed(&mut self, root: &mut RenderRoot, query: String) {
        let workspace = self.tree.root().map(|p| p.to_path_buf());
        let Some(Popup::Search(state)) = &mut self.popup else { return };
        state.query = query;
        let trimmed = state.query.trim();
        match workspace {
            Some(workspace) if !trimmed.is_empty() => {
                state.worker.ask(workspace, trimmed.to_owned());
                self.show_placeholder(root);
            }
            _ => {
                state.worker.stop();
                state.results.clear();
                state.terms.clear();
                self.render_search(root);
            }
        }
    }
}
