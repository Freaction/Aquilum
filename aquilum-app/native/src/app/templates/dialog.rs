use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, NewWidget, PropertySet, WidgetId};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::Padding;
use masonry::widgets::Flex;
use crate::ui::editor::TextAction;

use super::super::{App, Outcome, Popup};
use crate::i18n::{t, t_with};
use crate::ui::handle::Handle;
use crate::ui::icons;
use crate::ui::scroll::ScrollArea;
use crate::ui::search::{empty_state_with, CardAction, CardData, ResultCard};
use crate::ui::tokens::size;
use crate::ui::widgets::{ButtonSize, Pressed, Slot, TextButton, Variant};

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

            let empty = empty_state_with(icons::FILE_TEXT, &label, Some(&hint), Some(create_btn.erased()));
            let empty = empty.with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_24)))).with_class(crate::ui::style::CANVAS);
            let body = state.body;
            body.edit(root, |mut slot| Slot::set_child(&mut slot, empty.erased()));
            return;
        }

        let mut cards = Vec::new();
        let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (i, &idx) in state.filtered.iter().enumerate() {
            let tmpl = &state.templates[idx];
            let data = CardData {
                title: tmpl.name.clone(),
                extension: "md".to_owned(),
                snippet: tmpl.relative.clone(),
                count: String::new(),
                title_marks: Vec::new(),
                extension_marks: Vec::new(),
                snippet_marks: Vec::new(),
            };
            let card = ResultCard::new(data, i == state.selected);
            cards.push(card.id());
            column = column.with_fixed(card);
        }
        state.cards = cards;

        let scroll = NewWidget::new(ScrollArea::new(NewWidget::new(column)));
        state.list = Some(Handle::of(&scroll));
        let body = state.body;
        body.edit(root, |mut slot| Slot::set_child(&mut slot, scroll.erased()));
    }

    pub(super) fn select_template(&mut self, root: &mut RenderRoot, index: usize) {
        let Some(Popup::Templates(state)) = &mut self.popup else { return };
        super::super::search::select_card(root, &state.cards, state.list, &mut state.selected, index);
    }

    pub fn on_templates_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        let Some(Popup::Templates(state)) = &self.popup else { return Outcome::default() };
        if let Some(btn) = state.create_button
            && id == btn
            && action.downcast_ref::<Pressed>().is_some()
        {
            self.create_starter_templates(root);
            return Outcome::default();
        }
        if id == state.field.id() {
            if let Some(TextAction::Changed(query)) = action.downcast_ref::<TextAction>() {
                let query = query.trim().to_lowercase();
                if let Some(Popup::Templates(state)) = &mut self.popup {
                    state.filtered = state.templates.iter().enumerate()
                        .filter(|(_, t)| query.is_empty() || t.relative.to_lowercase().contains(&query))
                        .map(|(i, _)| i)
                        .collect();
                    state.selected = 0;
                }
                self.render_templates(root);
            }
            return Outcome::default();
        }
        if let Some(index) = state.cards.iter().position(|&card| card == id) {
            match action.downcast_ref::<CardAction>() {
                Some(CardAction::Hover) => self.select_template(root, index),
                Some(CardAction::Open { .. }) => {
                    return self.choose_template(root, index);
                }
                None => {}
            }
        }
        Outcome::default()
    }
}
