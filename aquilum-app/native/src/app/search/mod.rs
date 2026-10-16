mod input;
mod query;
mod render;
mod worker;

use std::sync::Arc;

use crate::ui::editor::TextEditor;

use aquilum_core::search::models::{SearchIndexStatus, SearchResult};
use masonry::accesskit::Role;
use masonry::app::RenderRoot;
use masonry::core::{NewWidget, WidgetId};
use masonry::kurbo::Point;
use masonry::widgets::Flex;

use super::{App, Popup};
use crate::i18n::t;
use crate::ui::dialog::{CardSize, Modal};
use crate::ui::handle::Handle;
use crate::ui::scroll::ScrollArea;
use crate::ui::search::{self, CardKind, Placeholder, ResultCard};
use crate::ui::widgets::Slot;

pub(super) use worker::Found;

pub(super) struct SearchState {
    layer: WidgetId,
    field: Handle<TextEditor>,
    placeholder: Handle<Placeholder>,
    clear: Handle<crate::ui::widgets::IconButton>,
    body: Handle<Slot>,
    list: Option<Handle<ScrollArea<Flex>>>,
    query: String,
    worker: worker::Worker,
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

    pub fn selected_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.results[self.selected].path)
    }

    #[cfg(feature = "memory-probe")]
    pub fn generation(&self) -> u64 {
        self.worker.current()
    }
}

#[cfg(all(test, feature = "memory-probe"))]
impl Found {
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

pub(super) fn select_card(root: &mut RenderRoot, kind: CardKind, cards: &[WidgetId], list: Option<Handle<ScrollArea<Flex>>>, selected: &mut usize, index: usize) {
    if index >= cards.len() {
        return;
    }
    let previous = std::mem::replace(selected, index);
    for (i, on) in [(previous, false), (index, true)] {
        if let Some(&card) = cards.get(i) {
            root.edit_widget(card, |mut w| ResultCard::set_selected(&mut w.downcast(), on));
        }
    }
    if let Some(list) = list {
        let (top, bottom) = search::offset_of(kind, index);
        list.edit(root, |mut area| ScrollArea::reveal(&mut area, top, bottom));
    }
}

impl App {
    pub fn open_search(&mut self, root: &mut RenderRoot) {
        if matches!(self.popup, Some(Popup::Search(_))) {
            return;
        }
        let search_box = search::search_box("");
        let body = Slot::new(NewWidget::new(Flex::column()).erased());
        let body_handle = Handle::of(&body);
        let card = search::dialog_card(search_box.widget, body, search::footer());
        let layer = Modal::new(card, &t("search.title"), CardSize::Window(search::card_size), Role::Dialog, false);
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
            worker: worker::Worker::spawn(Arc::clone(&self.workspace.core), self.search_tx.clone(), Arc::clone(&self.workspace.wake)),
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
            crate::ui::dialog::close(root, state.layer);
            if let Some(ids) = &self.note_ids {
                root.focus_on(Some(ids.body.id()));
            }
        }
    }
}
