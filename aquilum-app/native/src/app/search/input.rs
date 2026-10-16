use std::path::PathBuf;

use masonry::app::RenderRoot;
use masonry::core::keyboard::{Key, Modifiers};
use crate::ui::keymap::{self, Command, Scope};
use masonry::core::{ErasedAction, WidgetId};

use super::{App, Popup, select_card};
use crate::app::Outcome;
use crate::ui::dialog::DialogAnswer;
use crate::ui::editor::{TextAction, TextEditor};
use crate::ui::search::CardAction;
use crate::ui::widgets::Pressed;

impl App {
    fn select_result(&mut self, root: &mut RenderRoot, index: usize) {
        let Some(Popup::Search(state)) = &mut self.popup else { return };
        select_card(root, crate::ui::search::CardKind::Result, &state.cards, state.list, &mut state.selected, index);
    }

    fn open_selected(&mut self, root: &mut RenderRoot, new_tab: bool) -> Outcome {
        let Some(Popup::Search(state)) = &self.popup else { return Outcome::default() };
        let Some(result) = state.results.get(state.selected) else { return Outcome::default() };
        let path = PathBuf::from(&result.path);
        let offset = result.match_offset;
        self.close_search(root);
        let title = self.open_in_with_offset(root, path, new_tab, Some(offset));
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
        match keymap::command(&[Scope::List], key, modifiers)? {
            command @ (Command::ListNext | Command::ListPrevious) => {
                if count > 0 {
                    self.select_result(root, keymap::cycle(selected, count, command == Command::ListNext));
                }
                Some(Outcome::default())
            }
            Command::ListCreate => Some(self.create_from_query(root)),
            command @ (Command::ListOpen | Command::ListOpenInNewTab) => Some(self.open_selected(root, command == Command::ListOpenInNewTab)),
            Command::Close => {
                self.close_search(root);
                Some(Outcome::default())
            }
            _ => None,
        }
    }

    pub(in crate::app) fn on_search_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        let Some(Popup::Search(state)) = &self.popup else { return Outcome::default() };
        if id == state.layer && matches!(action.downcast_ref::<DialogAnswer>(), Some(DialogAnswer::Cancel)) {
            self.close_search(root);
            return Outcome::default();
        }
        if id == state.field.id() {
            if let Some(TextAction::Changed(text)) = action.downcast_ref::<TextAction>() {
                self.query_changed(root, text.clone());
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
