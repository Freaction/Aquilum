use aquilum_core::search::hits::query_matcher;
use aquilum_core::search::models::SearchIndexState;
use aquilum_core::text::matcher::Matcher;
use masonry::app::RenderRoot;
use masonry::core::{NewWidget, Widget};

use super::{App, Popup};
use crate::i18n::{plural, t, t_with};
use crate::ui::handle::Handle;
use crate::ui::icons;
use crate::ui::scroll::ScrollArea;
use crate::ui::search::{self, CardData, Placeholder, ResultCard};
use crate::ui::widgets::Slot;

fn marks(matcher: &Matcher, text: &str) -> Vec<(usize, usize)> {
    matcher.find_all(text).into_iter().map(|found| (found.start, found.end)).collect()
}

impl App {
    pub(super) fn show_placeholder(&mut self, root: &mut RenderRoot) {
        let Some(Popup::Search(state)) = &self.popup else { return };
        let empty_query = state.query.is_empty();
        state.placeholder.edit(root, |mut p| Placeholder::set_empty(&mut p, empty_query));
    }

    pub(super) fn render_search(&mut self, root: &mut RenderRoot) {
        let has_workspace = self.tree.root().is_some();
        let Some(Popup::Search(state)) = &mut self.popup else { return };
        let indexing = state.status.as_ref().is_some_and(|s| s.state == SearchIndexState::Indexing);
        let error = state.failed || state.status.as_ref().is_some_and(|s| s.state == SearchIndexState::Error);
        let empty = |icon, title: &str, description: String| search::empty_state(icon, &t(title), &description).erased();
        let mut cards = Vec::new();
        let widget: NewWidget<dyn Widget> = if !has_workspace {
            empty(icons::FOLDER_SEARCH, "search.noFolder", t("search.noFolderHint"))
        } else if error {
            empty(icons::DATABASE_ZAP, "search.unavailable", t("search.unavailableHint"))
        } else if state.query.trim().is_empty() {
            empty(icons::SEARCH, "search.title", t("search.titleHint"))
        } else if state.results.is_empty() && indexing {
            let scanned = state.status.as_ref().map_or(0, |s| s.scanned_documents).to_string();
            empty(icons::SEARCH, "search.indexing", t_with("search.indexingHint", &[("count", &scanned)]))
        } else if state.results.is_empty() {
            empty(icons::SEARCH_X, "search.noResults", t_with("search.createHint", &[("shortcut", "Shift + ↵")]))
        } else {
            let matcher = query_matcher(&state.terms);
            let widgets: Vec<_> = state
                .results
                .iter()
                .enumerate()
                .map(|(index, result)| {
                    let data = CardData {
                        title_marks: marks(&matcher, &result.title),
                        extension_marks: marks(&matcher, &result.extension),
                        snippet_marks: marks(&matcher, &result.snippet),
                        title: result.title.clone(),
                        extension: result.extension.clone(),
                        snippet: result.snippet.clone(),
                        count: plural("search.matches", result.match_count as u64),
                    };
                    ResultCard::new(data, index == state.selected)
                })
                .collect();
            let (list, ids) = search::results_list(widgets);
            cards = ids;
            let area = NewWidget::new(ScrollArea::new(list));
            state.list = Some(Handle::of(&area));
            area.erased()
        };
        if cards.is_empty() {
            state.list = None;
        }
        state.cards = cards;
        state.body.edit(root, |mut slot| Slot::set_child(&mut slot, widget));
        self.show_placeholder(root);
    }
}
