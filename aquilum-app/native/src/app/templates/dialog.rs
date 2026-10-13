use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, NewWidget, PropertySet, WidgetId};
use masonry::layout::Length;
use masonry::properties::Padding;
use crate::ui::editor::TextAction;

use super::super::{App, Outcome, Popup};
use super::NoteTemplate;
use crate::i18n::{t, t_with};
use crate::ui::dialog::DialogAnswer;
use crate::ui::handle::Handle;
use crate::ui::icons;
use crate::ui::scroll::ScrollArea;
use crate::ui::search::{empty_state_with, results_list, CardAction, CardData, CardKind, Marks, Placeholder, ResultCard};
use crate::ui::tokens::size;
use crate::ui::widgets::{ButtonSize, Pressed, Slot, TextButton, Variant};

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn marks(text: &str, query: &str) -> Marks {
    let needle: Vec<char> = query.chars().map(fold).collect();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let lower: Vec<char> = chars.iter().map(|&(_, c)| fold(c)).collect();
    let mut found = Vec::new();
    let mut at = 0;
    while !needle.is_empty() && at + needle.len() <= lower.len() {
        if lower[at..at + needle.len()] == needle[..] {
            found.push((chars[at].0, chars.get(at + needle.len()).map_or(text.len(), |&(end, _)| end)));
            at += needle.len();
        } else {
            at += 1;
        }
    }
    found
}

pub(super) fn filtered(templates: &[NoteTemplate], query: &str) -> Vec<usize> {
    let query = query.trim();
    (0..templates.len()).filter(|&i| query.is_empty() || !marks(&templates[i].relative, query).is_empty()).collect()
}

impl App {
    pub(super) fn render_templates(&mut self, root: &mut RenderRoot) {
        let dir = self.templates_dir();
        let dir_exists = dir.as_ref().is_some_and(|d| d.is_dir());
        let Some(Popup::Templates(state)) = &mut self.popup else { return };

        if !dir_exists || state.templates.is_empty() {
            let label = t("templateDialog.empty");
            let folder_name = dir.and_then(|d| d.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_else(|| "Templates".to_owned());
            let hint = t_with("templateDialog.emptyHint", &[("folder", &folder_name)]);
            let create_btn = NewWidget::new(TextButton::new(t("templateDialog.createFolder"), Variant::Primary).size(ButtonSize::S));
            let btn_id = create_btn.id();
            state.create_button = Some(btn_id);
            state.cards.clear();

            let empty = empty_state_with(icons::FILE_TEXT, &label, Some(&hint), Some(create_btn.erased()));
            let empty = empty.with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_24))));
            let body = state.body;
            body.edit(root, |mut slot| Slot::set_child(&mut slot, empty.erased()));
            return;
        }

        let query = state.query.trim().to_owned();
        let cards: Vec<_> = state
            .filtered
            .iter()
            .enumerate()
            .map(|(i, &idx)| {
                let tmpl = &state.templates[idx];
                let title = tmpl.title().to_owned();
                let data = CardData {
                    title_marks: marks(&title, &query),
                    title,
                    extension: tmpl.extension().to_owned(),
                    snippet: String::new(),
                    count: String::new(),
                    extension_marks: Vec::new(),
                    snippet_marks: Vec::new(),
                };
                ResultCard::template(data, i == state.selected)
            })
            .collect();
        let (column, ids) = results_list(cards);
        state.cards = ids;
        let scroll = NewWidget::new(ScrollArea::new(column));
        state.list = Some(Handle::of(&scroll));
        let body = state.body;
        body.edit(root, |mut slot| Slot::set_child(&mut slot, scroll.erased()));
    }

    pub(super) fn select_template(&mut self, root: &mut RenderRoot, index: usize) {
        let Some(Popup::Templates(state)) = &mut self.popup else { return };
        super::super::search::select_card(root, CardKind::Template, &state.cards, state.list, &mut state.selected, index);
    }

    fn template_query_changed(&mut self, root: &mut RenderRoot, query: String) {
        let Some(Popup::Templates(state)) = &mut self.popup else { return };
        state.filtered = filtered(&state.templates, &query);
        state.selected = 0;
        let (placeholder, empty): (Handle<Placeholder>, bool) = (state.placeholder, query.is_empty());
        state.query = query;
        placeholder.edit(root, |mut p| Placeholder::set_empty(&mut p, empty));
        self.render_templates(root);
    }

    pub fn on_templates_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        let Some(Popup::Templates(state)) = &self.popup else { return Outcome::default() };
        if id == state.layer && matches!(action.downcast_ref::<DialogAnswer>(), Some(DialogAnswer::Cancel)) {
            self.close_templates(root);
            return Outcome::default();
        }
        if let Some(btn) = state.create_button
            && id == btn
            && action.downcast_ref::<Pressed>().is_some()
        {
            self.create_starter_templates(root);
            return Outcome::default();
        }
        if id == state.field.id() {
            match action.downcast_ref::<TextAction>() {
                Some(TextAction::Changed(query)) => self.template_query_changed(root, query.clone()),
                Some(TextAction::Cancelled) => self.close_templates(root),
                _ => {}
            }
            return Outcome::default();
        }
        if let Some(index) = state.cards.iter().position(|&card| card == id) {
            match action.downcast_ref::<CardAction>() {
                Some(CardAction::Hover) => self.select_template(root, index),
                Some(CardAction::Open { .. }) => return self.choose_template(root, index),
                None => {}
            }
        }
        Outcome::default()
    }
}
