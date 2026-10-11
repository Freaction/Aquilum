use std::time::Duration;

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;
use crate::ui::editor::TextAction;

use super::super::{App, Outcome};
use crate::file_ops;
use crate::frontmatter;
use crate::ui::editor::TextEditor;
use crate::ui::note::{MetadataToggled, TitleAction, TitleField};
use crate::ui::widgets::{Pressed, Slot};

impl App {
    pub(in crate::app) fn on_note_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Option<Outcome> {
        // Проверяем кнопки на экране ошибки открытия
        if let Some(err_ids) = &self.open_error_ids {
            if action.downcast_ref::<Pressed>().is_some() {
                if id == err_ids.retry {
                    self.show_active(root);
                    return Some(Outcome::default());
                }
                if id == err_ids.reveal {
                    if let Some((path, _)) = &self.open_error {
                        crate::system::reveal(path);
                    }
                    return Some(Outcome::default());
                }
                if id == err_ids.close {
                    let active = self.tabs.active_id();
                    self.tabs.close(active);
                    self.show_active(root);
                    return Some(Outcome { title: true, ..Outcome::default() });
                }
            }
        }

        // Проверяем кнопку «Повторить» в баннере ошибки сохранения
        if let Some(save_ids) = &self.save_error_ids {
            if action.downcast_ref::<Pressed>().is_some() && id == save_ids.retry {
                self.retry_save(root);
                return Some(Outcome::default());
            }
        }

        let ids = self.note_ids.as_ref()?;
        if id == ids.scroll.id() {
            if action.downcast_ref::<crate::ui::scroll::Scrolled>().is_some() {
                self.capture_view(root);
            }
            return Some(Outcome::default());
        }
        if id == ids.body.id() {
            match action.downcast_ref::<TextAction>() {
                Some(TextAction::Edited(edits)) => self.body_edited(root, edits),
                Some(TextAction::Changed(visible)) => {
                    let range = 0..self.editor_text.len() - self.hidden;
                    self.body_edited(root, &[crate::ui::editor::Edit { range, insert: visible.clone() }]);
                }
                _ => {}
            }
            return Some(Outcome::default());
        }
        if id == ids.title.id() {
            match action.downcast_ref::<TextAction>() {
                Some(TextAction::Changed(text)) => {
                    let empty = text.is_empty();
                    ids.title_box.edit(root, |mut field| TitleField::set_empty(&mut field, empty));
                }
                Some(TextAction::Entered(text)) => {
                    let text = text.clone();
                    self.commit_title(root, &text);
                    if let Some(ids) = &self.note_ids {
                        let body = ids.body;
                        root.focus_on(Some(body.id()));
                        body.edit(root, |mut body| TextEditor::select_byte_range(&mut body, 0, 0));
                    }
                }
                _ => {}
            }
            return Some(Outcome::default());
        }
        if id == ids.title_box.id() && action.downcast_ref::<TitleAction>().is_some() {
            let title = ids.title;
            let text = title.get(root).map(|t| t.inner().text().to_string()).unwrap_or_default();
            self.commit_title(root, &text);
            return Some(Outcome::default());
        }
        if action.downcast_ref::<Pressed>().is_some() {
            if id == ids.back {
                return Some(self.navigate(root, -1));
            }
            if id == ids.forward {
                return Some(self.navigate(root, 1));
            }
            if id == ids.focus {
                self.toggle_focus_mode(root);
                return Some(Outcome::default());
            }
            if id == ids.more {
                let transform = root.get_widget(id).map_or(Point::ORIGIN, |w| w.ctx().window_transform() * Point::ZERO);
                self.open_note_actions_menu(root, Point::new(transform.x, transform.y + crate::ui::tokens::size::SIZE_32));
                return Some(Outcome::default());
            }
        }
        if action.downcast_ref::<MetadataToggled>().is_some() {
            self.toggle_metadata(root);
            return Some(Outcome::default());
        }
        None
    }

    pub fn navigate(&mut self, root: &mut RenderRoot, delta: isize) -> Outcome {
        let Some(path) = self.navigation.step(delta) else { return Outcome::default() };
        if !path.is_file() {
            return Outcome::default();
        }
        self.tabs.open(path);
        self.show_active(root);
        Outcome { title: true, ..Outcome::default() }
    }

    pub fn toggle_focus_mode(&mut self, root: &mut RenderRoot) {
        use crate::ui::widgets::{Collapse, TextButton, Visible};
        let focus = !self.interface.focus_mode();
        self.interface.set_focus_mode(focus);
        let (sidebar, panel) = self.visible_panels();
        self.shown = Some((sidebar, panel));
        let (home, home_button) = crate::ui::home_widget(self.home.is_some() && !focus);
        if let Some(chrome) = &mut self.chrome {
            chrome.sidebar.edit(root, |mut c| Collapse::set_open(&mut c, sidebar));
            chrome.panel.edit(root, |mut c| Collapse::set_open(&mut c, panel));
            chrome.controls_inset.edit(root, |mut c| Collapse::set_open(&mut c, !panel));
            for shown in &chrome.focus_hidden {
                shown.edit(root, |mut v| Visible::set_shown(&mut v, !focus));
            }
            chrome.focus_shown.edit(root, |mut v| Visible::set_shown(&mut v, focus));
            chrome.home_slot.edit(root, |mut slot| Slot::set_child(&mut slot, home));
            chrome.home_button = home_button;
        }
        if let Some(ids) = &self.note_ids {
            for shown in &ids.focus_hidden {
                shown.edit(root, |mut v| Visible::set_shown(&mut v, !focus));
            }
            ids.focus_button.edit(root, |mut b| TextButton::set_pressed(&mut b, focus));
        }
        if panel {
            self.request_panel(root);
        }
    }

    fn body_edited(&mut self, root: &mut RenderRoot, edits: &[crate::ui::editor::Edit]) {
        let had = frontmatter::end(&self.editor_text).is_some();
        for edit in edits {
            self.editor_text.replace_range(self.hidden + edit.range.start..self.hidden + edit.range.end, &edit.insert);
        }
        self.queue_save();
        self.refresh_hero(root);
        self.metadata_changed(root, had);
    }

    pub(in crate::app) fn commit_editor_text(&mut self, root: &mut RenderRoot, text: String) {
        let had = frontmatter::end(&self.editor_text).is_some();
        self.editor_text = text;
        self.queue_save();
        self.metadata_changed(root, had);
        self.sync_body(root);
        self.refresh_hero(root);
    }

    fn queue_save(&mut self) {
        if let Some(note) = &mut self.note {
            let debounce = self.workspace.core.settings.get_config().editor.save_debounce_ms;
            note.edit(Duration::from_millis(u64::from(debounce)));
        }
    }

    fn metadata_changed(&mut self, root: &mut RenderRoot, had: bool) {
        let has = frontmatter::end(&self.editor_text).is_some();
        if has == had {
            return;
        }
        if has && let Some(note) = &self.note {
            self.interface.set_metadata_expanded(&note.path, true);
        }
        let metadata = crate::ui::note::metadata_widget(self.metadata_state());
        if let Some(ids) = &self.note_ids {
            ids.metadata.edit(root, |mut slot| Slot::set_child(&mut slot, metadata));
        }
    }

    fn toggle_metadata(&mut self, root: &mut RenderRoot) {
        let Some(expanded) = self.metadata_state() else { return };
        let Some(path) = self.note.as_ref().map(|n| n.path.clone()) else { return };
        self.interface.set_metadata_expanded(&path, !expanded);
        self.sync_body(root);
    }

    fn commit_title(&mut self, root: &mut RenderRoot, input: &str) {
        let Some(from) = self.note.as_ref().map(|n| n.path.clone()) else { return };
        match file_ops::rename_target(&from, input) {
            Some((to, _)) if self.renaming_to.as_ref() == Some(&to) => {}
            Some((to, _)) => {
                self.renaming_to = Some(to.clone());
                file_ops::rename(&self.workspace.core, from, to);
            }
            None => self.refresh_note_names(root),
        }
    }
}
