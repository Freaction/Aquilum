use std::path::PathBuf;
use std::sync::Arc;
use crate::ui::editor::TextEditor;
use std::sync::mpsc::Sender;

use aquilum_core::Core;
use aquilum_core::search::matching::find_prefix_matches;
use aquilum_core::search::models::{SearchIndexState, SearchIndexStatus, SearchResult};
use masonry::accesskit::Role;
use masonry::app::RenderRoot;
use masonry::core::keyboard::{Key, Modifiers, NamedKey};
use masonry::core::{ErasedAction, NewWidget, Widget, WidgetId};
use masonry::kurbo::{Axis, Point};
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Flex, TextAction};

use super::{App, Outcome, Popup};
use crate::i18n::{plural, t, t_with};
use crate::ui::dialog::{CardSize, DialogAnswer, Modal};
use crate::ui::handle::Handle;
use crate::ui::icons;
use crate::ui::scroll::ScrollArea;
use crate::ui::search::{self, CardAction, CardData, Placeholder, ResultCard};
use crate::ui::widgets::{Pressed, Rule, Slot};
use crate::ui::theme;

pub(super) struct SearchState {
    layer: WidgetId,
    field: Handle<TextEditor>,
    placeholder: Handle<Placeholder>,
    clear: Handle<crate::ui::widgets::IconButton>,
    body: Handle<Slot>,
    list: Option<Handle<ScrollArea<Flex>>>,
    query: String,
    generation: u64,
    results: Vec<SearchResult>,
    terms: Vec<String>,
    status: Option<SearchIndexStatus>,
    failed: bool,
    selected: usize,
    cards: Vec<WidgetId>,
}

#[cfg(test)]
impl SearchState {
    pub fn field_id(&self) -> WidgetId {
        self.field.id()
    }

    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    pub fn selected_path(&self) -> PathBuf {
        PathBuf::from(&self.results[self.selected].path)
    }
}

pub(super) struct Found {
    generation: u64,
    response: Result<aquilum_core::search::models::SearchResponse, String>,
}

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

fn spawn_search(core: Arc<Core>, workspace: PathBuf, query: String, generation: u64, results: Sender<Found>, wake: Arc<dyn Fn() + Send + Sync>) {
    std::thread::spawn(move || {
        let response = core.search.search(&workspace.to_string_lossy(), &query, None).map_err(|e| format!("{e:?}"));
        if results.send(Found { generation, response }).is_ok() {
            wake();
        }
    });
}

impl App {
    pub fn open_search(&mut self, root: &mut RenderRoot) {
        if matches!(self.popup, Some(Popup::Search(_))) {
            return;
        }
        let search_box = search::search_box("");
        let body = Slot::new(NewWidget::new(Flex::column()).erased());
        let body_handle = Handle::of(&body);
        let card = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(search_box.widget)
            .with_fixed(Rule::new(Axis::Horizontal, || theme::current().border_default))
            .with(body, 1.0)
            .with_fixed(Rule::new(Axis::Horizontal, || theme::current().border_default))
            .with_fixed(search::footer());
        let layer = Modal::new(NewWidget::new(card), &t("search.title"), CardSize::Window(search::card_size), Role::Dialog, false);
        let layer_id = layer.id();
        root.add_layer(layer, Point::ORIGIN);
        root.focus_on(Some(search_box.field.id()));
        let status = self.tree.root().map(|w| self.workspace.core.search.status(Some(&w.to_string_lossy())));
        self.popup = Some(Popup::Search(SearchState {
            layer: layer_id,
            field: search_box.field,
            placeholder: search_box.placeholder,
            clear: search_box.clear,
            body: body_handle,
            list: None,
            query: String::new(),
            generation: 0,
            results: Vec::new(),
            terms: Vec::new(),
            status,
            failed: false,
            selected: 0,
            cards: Vec::new(),
        }));
        self.render_search(root);
    }

    fn close_search(&mut self, root: &mut RenderRoot) {
        if let Some(Popup::Search(state)) = self.popup.take() {
            root.remove_layer(state.layer);
            if let Some(ids) = &self.note_ids {
                root.focus_on(Some(ids.body.id()));
            }
        }
    }

    fn render_search(&mut self, root: &mut RenderRoot) {
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

    pub(super) fn on_search_results(&mut self, root: &mut RenderRoot) {
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
                eprintln!("поиск не выполнен: {error}");
                state.failed = true;
            }
        }
        state.selected = state.selected.min(state.results.len().saturating_sub(1));
        self.render_search(root);
    }

    fn query_changed(&mut self, root: &mut RenderRoot, query: String) {
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

    fn select_result(&mut self, root: &mut RenderRoot, index: usize) {
        let Some(Popup::Search(state)) = &mut self.popup else { return };
        if index >= state.cards.len() {
            return;
        }
        let previous = std::mem::replace(&mut state.selected, index);
        for (i, card) in state.cards.iter().enumerate() {
            if i == previous || i == index {
                root.edit_widget(*card, |mut w| ResultCard::set_selected(&mut w.downcast(), i == index));
            }
        }
        if let Some(list) = state.list {
            let (top, bottom) = search::offset_of(index);
            list.edit(root, |mut area| ScrollArea::reveal(&mut area, top, bottom));
        }
    }

    fn open_selected(&mut self, root: &mut RenderRoot, new_tab: bool) -> Outcome {
        let Some(Popup::Search(state)) = &self.popup else { return Outcome::default() };
        let Some(result) = state.results.get(state.selected) else { return Outcome::default() };
        let path = PathBuf::from(&result.path);
        self.close_search(root);
        let title = self.open_in(root, path, new_tab);
        Outcome { title, ..Outcome::default() }
    }

    fn create_from_query(&mut self, root: &mut RenderRoot) -> Outcome {
        let Some(Popup::Search(state)) = &self.popup else { return Outcome::default() };
        let title = state.query.trim().to_owned();
        if title.is_empty() {
            return Outcome::default();
        }
        self.close_search(root);
        let opened = self.create_note_titled(root, Some(&title));
        Outcome { title: opened, ..Outcome::default() }
    }

    pub fn search_key(&mut self, root: &mut RenderRoot, key: &Key, modifiers: Modifiers) -> Option<Outcome> {
        let Some(Popup::Search(state)) = &self.popup else { return None };
        let count = state.cards.len();
        let selected = state.selected;
        match key {
            Key::Named(NamedKey::ArrowDown | NamedKey::ArrowUp) => {
                if count > 0 {
                    let next = if *key == Key::Named(NamedKey::ArrowDown) { (selected + 1) % count } else { (selected + count - 1) % count };
                    self.select_result(root, next);
                }
                Some(Outcome::default())
            }
            Key::Named(NamedKey::Enter) if modifiers.shift() => Some(self.create_from_query(root)),
            Key::Named(NamedKey::Enter) => Some(self.open_selected(root, modifiers.ctrl() || modifiers.meta())),
            Key::Named(NamedKey::Escape) => {
                self.close_search(root);
                Some(Outcome::default())
            }
            _ => None,
        }
    }

    pub(super) fn on_search_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        let Some(Popup::Search(state)) = &self.popup else { return Outcome::default() };
        if id == state.layer && matches!(action.downcast_ref::<DialogAnswer>(), Some(DialogAnswer::Cancel)) {
            self.close_search(root);
            return Outcome::default();
        }
        if id == state.field.id() {
            match action.downcast_ref::<TextAction>() {
                Some(TextAction::Changed(text)) => self.query_changed(root, text.clone()),
                Some(TextAction::Cancelled) => self.close_search(root),
                _ => {}
            }
            return Outcome::default();
        }
        if id == state.clear.id() && action.downcast_ref::<Pressed>().is_some() {
            let field = state.field;
            field.edit(root, |mut f| TextEditor::reset_text(&mut f, ""));
            root.focus_on(Some(field.id()));
            self.query_changed(root, String::new());
            return Outcome::default();
        }
        let Some(index) = state.cards.iter().position(|card| *card == id) else { return Outcome::default() };
        match action.downcast_ref::<CardAction>() {
            Some(CardAction::Hover) => {
                self.select_result(root, index);
                Outcome::default()
            }
            Some(CardAction::Open { new_tab }) => {
                if let Some(Popup::Search(state)) = &mut self.popup {
                    state.selected = index;
                }
                self.open_selected(root, *new_tab)
            }
            None => Outcome::default(),
        }
    }
}
