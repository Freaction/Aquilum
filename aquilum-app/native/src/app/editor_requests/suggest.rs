use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use super::super::App;
use crate::ui::editor::TextEditor;

impl App {
    pub(in crate::app) fn show_suggest(&mut self, root: &mut RenderRoot, view: Option<crate::ui::editor::resolve::Suggestions>) {
        let Some(view) = view else {
            if let Some((layer, _)) = self.suggest_layer.take() {
                root.remove_layer(layer);
            }
            return;
        };
        let Some(transform) = root.get_widget(view.editor).map(|w| w.ctx().window_transform()) else { return };
        let inset = crate::ui::tokens::size::PADDING_XS + crate::ui::tokens::size::GAP_XS;
        let caret = transform.transform_rect_bbox(view.caret) - masonry::kurbo::Vec2::new(inset, 0.0);
        let at = crate::ui::menu::Menu::around(caret, crate::ui::suggest::SuggestList::size_for(&view.items), self.room());
        match self.suggest_layer {
            Some((layer, _)) => {
                root.edit_widget(layer, |mut w| crate::ui::suggest::SuggestList::set(&mut w.downcast(), view.items, view.query, view.selected));
                root.reposition_layer(layer, at);
                self.suggest_layer = Some((layer, view.editor));
            }
            None => {
                let list = crate::ui::suggest::SuggestList::new(view.items, view.query, view.selected);
                let layer = list.id();
                root.add_layer(list, at);
                self.suggest_layer = Some((layer, view.editor));
            }
        }
    }

    pub(crate) fn on_suggest_pick(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> bool {
        let Some((layer, editor)) = self.suggest_layer else { return false };
        if id != layer {
            return false;
        }
        if let Some(picked) = action.downcast_ref::<crate::ui::suggest::SuggestPicked>() {
            root.focus_on(Some(editor));
            root.edit_widget(editor, |mut w| TextEditor::accept_suggestion(&mut w.downcast(), picked.0));
        }
        true
    }
}
