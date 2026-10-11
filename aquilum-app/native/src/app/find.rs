//! Поиск по заметке: Ctrl+F открывает панель, Enter / Shift+Enter — к следующему и предыдущему
//! совпадению, Escape закрывает.

use masonry::app::RenderRoot;
use masonry::core::keyboard::{Key, Modifiers, NamedKey};
use masonry::core::{ErasedAction, NewWidget, WidgetId};
use masonry::widgets::SizedBox;
use crate::ui::editor::TextAction;

use super::App;
use crate::ui::editor::TextEditor;
use crate::ui::search::Placeholder;
use crate::ui::widgets::{Pressed, Slot};

impl App {
    pub fn open_find(&mut self, root: &mut RenderRoot) {
        let Some(ids) = &self.note_ids else { return };
        if self.find.is_none() {
            let (bar, find) = crate::ui::find::find_bar();
            ids.find.edit(root, |mut slot| Slot::set_child(&mut slot, bar));
            self.find = Some(find);
        }
        let field = self.find.as_ref().map(|f| f.field.id());
        root.focus_on(field);
    }

    fn close_find(&mut self, root: &mut RenderRoot) {
        let (Some(_), Some(ids)) = (self.find.take(), &self.note_ids) else { return };
        ids.find.edit(root, |mut slot| Slot::set_child(&mut slot, NewWidget::new(SizedBox::empty()).erased()));
        ids.body.edit(root, |mut body| TextEditor::find(&mut body, ""));
        root.focus_on(Some(ids.body.id()));
    }

    fn show_status(&self, root: &mut RenderRoot, (current, total): (usize, usize)) {
        if let Some(find) = &self.find {
            find.counter.edit(root, |mut c| crate::ui::widgets::Label::set_text(&mut c, &format!("{current}/{total}")));
        }
    }

    fn find_step(&mut self, root: &mut RenderRoot, delta: isize) {
        let Some(ids) = &self.note_ids else { return };
        let Some((status, start)) = ids.body.edit(root, |mut body| TextEditor::find_step(&mut body, delta)) else { return };
        self.show_status(root, status);
        if let Some(byte) = start {
            self.reveal(root, byte);
        }
    }

    pub(super) fn on_find_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> bool {
        let Some(find) = &self.find else { return false };
        let (field, placeholder, prev, next, all, close) = (find.field.id(), find.placeholder, find.prev, find.next, find.all, find.close);
        if id == field {
            if let Some(TextAction::Changed(query)) = action.downcast_ref::<TextAction>()
                && let Some(ids) = &self.note_ids
            {
                let status = ids.body.edit(root, |mut body| TextEditor::find(&mut body, query)).unwrap_or_default();
                placeholder.edit(root, |mut p| Placeholder::set_empty(&mut p, query.is_empty()));
                self.show_status(root, status);
            }
            return true;
        }
        if action.downcast_ref::<Pressed>().is_none() || ![prev, next, all, close].contains(&id) {
            return false;
        }
        match id {
            _ if id == prev => self.find_step(root, -1),
            _ if id == next => self.find_step(root, 1),
            _ if id == all => {
                if let Some(ids) = &self.note_ids {
                    ids.body.edit(root, |mut body| TextEditor::find_show_all(&mut body));
                }
            }
            _ => self.close_find(root),
        }
        true
    }

    pub fn find_key(&mut self, root: &mut RenderRoot, key: &Key, modifiers: Modifiers) -> bool {
        let Some(find) = &self.find else { return false };
        if root.focused_widget() != Some(find.field.id()) {
            return false;
        }
        match key {
            Key::Named(NamedKey::Enter) => self.find_step(root, if modifiers.shift() { -1 } else { 1 }),
            Key::Named(NamedKey::Escape) => self.close_find(root),
            _ => return false,
        }
        true
    }
}
