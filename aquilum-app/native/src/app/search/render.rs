use aquilum_core::search::matching::find_prefix_matches;
use aquilum_core::search::models::SearchIndexState;
use masonry::app::RenderRoot;
use masonry::core::{NewWidget, Widget};

use super::{App, Popup};
use crate::i18n::{plural, t, t_with};
use crate::ui::handle::Handle;
use crate::ui::icons;
use crate::ui::scroll::ScrollArea;
use crate::ui::search::{self, CardData, Placeholder, ResultCard};
use crate::ui::widgets::Slot;

fn marks(text: &str, terms: &[String]) -> Vec<(usize, usize)> {
    let mut ranges: Vec<(usize, usize)> = find_prefix_matches(text, terms).into_iter().map(|(s, e, _)| (s, e)).collect();
    ranges.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    let mut kept: Vec<(usize, usize)> = Vec::new();
    for range in ranges {
        if kept.last().is_none_or(|last| range.0 >= last.1) {
            kept.push(range);
        }
    }
    kept
}

impl App {
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
            let widgets: Vec<_> = state
                .results
                .iter()
                .enumerate()
                .map(|(index, result)| {
                    let data = CardData {
                        title_marks: marks(&result.title, &state.terms),
                        extension_marks: marks(&result.extension, &state.terms),
                        snippet_marks: marks(&result.snippet, &state.terms),
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
        let empty_query = state.query.is_empty();
        state.body.edit(root, |mut slot| Slot::set_child(&mut slot, widget));
        state.placeholder.edit(root, |mut p| Placeholder::set_empty(&mut p, empty_query));
    }
}
