use masonry::app::RenderRoot;
use masonry::core::ErasedAction;
use masonry::kurbo::{Size, Vec2};

use super::super::App;
use crate::ui::editor::TextEditor;
use crate::ui::menu::Menu;
use crate::ui::suggest::{SuggestList, SuggestPicked};
use crate::ui::tokens::size;

impl App {
    pub(in crate::app) fn show_suggest(&mut self, root: &mut RenderRoot, view: Option<crate::ui::editor::resolve::Suggestions>) {
        let Some(view) = view else {
            if let Some((layer, _)) = self.suggest_layer.take() {
                root.remove_layer(layer);
            }
            return;
        };
        let Some(transform) = root.get_widget(view.editor).map(|w| w.ctx().window_transform()) else { return };
        let caret = transform.transform_rect_bbox(view.caret) - Vec2::new(size::PADDING_XS + size::GAP_XS, 0.0);
        let natural = SuggestList::size_for(&view.items);
        let (at, height) = Menu::around(caret, natural, self.room());
        let fitted = Size::new(natural.width, height);
        match self.suggest_layer {
            Some((layer, _)) => {
                root.edit_widget(layer, |mut w| SuggestList::set(&mut w.downcast(), view.items, view.query, view.selected, fitted));
                root.reposition_layer(layer, at);
                self.suggest_layer = Some((layer, view.editor));
            }
            None => {
                let list = SuggestList::new(view.items, view.query, view.selected, fitted);
                let layer = list.id();
                root.add_layer(list, at);
                self.suggest_layer = Some((layer, view.editor));
            }
        }
    }

    pub(crate) fn on_suggest_pick(&mut self, root: &mut RenderRoot, action: &ErasedAction) -> bool {
        let (Some((_, editor)), Some(picked)) = (self.suggest_layer, action.downcast_ref::<SuggestPicked>()) else { return false };
        root.focus_on(Some(editor));
        root.edit_widget(editor, |mut w| TextEditor::accept_suggestion(&mut w.downcast(), picked.0));
        true
    }
}
