use std::path::{Path, PathBuf};

use masonry::app::RenderRoot;
use masonry::widgets::IndexedStack;

use super::super::{App, Outcome};
use crate::note::Note;
use crate::ui::editor::TextEditor;
use crate::ui::widgets::Slot;

impl App {
    pub(in crate::app) fn show_active(&mut self, root: &mut RenderRoot) {
        if let Some((layer, _)) = self.suggest_layer.take() {
            root.remove_layer(layer);
        }
        for _ in 0..self.tabs.tabs().len().max(1) {
            let tab = self.tabs.active().clone();
            if tab.graph && self.tree.root().is_some() {
                self.leave_note();
                self.drop_note_page(root);
                self.tree.set_active(root, None);
                self.show_graph(root);
                self.show_page(root, 4);
                break;
            }
            let Some(path) = tab.path else {
                self.leave_note();
                self.drop_note_page(root);
                self.tree.set_active(root, None);
                self.show_page(root, if self.tree.root().is_some() { 1 } else { 3 });
                break;
            };
            if self.note.as_ref().is_some_and(|n| n.is(&path)) {
                self.reveal_pending(root);
                self.show_page(root, 0);
                break;
            }
            self.leave_note();
            match Note::open(path.clone()) {
                Ok((note, text)) => {
                    self.open_error = None;
                    self.open_error_ids = None;
                    self.editor_text = text;
                    self.note = Some(note);
                    self.render_note(root);
                    self.reveal_pending(root);
                    self.hold = Some(std::time::Instant::now() + super::super::HOLD);
                    self.tree.set_active(root, Some(path));
                    self.show_page(root, 0);
                    break;
                }
                Err(error) => {
                    let err_str = format!("{error}");
                    report!("заметка {} не прочитана: {err_str}", path.display());
                    self.open_error = Some((path.clone(), err_str.clone()));
                    self.show_open_error(root, &path, &err_str);
                    self.hold = Some(std::time::Instant::now() + super::super::HOLD);
                    self.tree.set_active(root, Some(path));
                    self.show_page(root, 0);
                    break;
                }
            }
        }
        self.version_view = None;
        self.show_version(root);
        self.refresh_tabs(root);
        self.request_panel(root);
        self.workspace.core.active_note.set(self.note.as_ref().map(|n| n.path.clone()));
    }

    pub(in crate::app) fn show_open_error(&mut self, root: &mut RenderRoot, path: &Path, error: &str) {
        self.note = None;
        self.note_ids = None;
        let (page, ids) = crate::ui::note::open_error_page(path, error);
        self.open_error_ids = Some(ids);
        if let Some(chrome) = &self.chrome {
            chrome.note_page.edit(root, |mut slot| Slot::set_child(&mut slot, page.erased()));
        }
    }

    fn drop_note_page(&mut self, root: &mut RenderRoot) {
        if self.note_ids.take().is_none() {
            return;
        }
        self.editor_text = String::new();
        if let Some(chrome) = &self.chrome {
            chrome.note_page.edit(root, |mut slot| Slot::set_child(&mut slot, masonry::core::NewWidget::new(masonry::widgets::SizedBox::empty()).erased()));
        }
    }

    pub(in crate::app) fn leave_note(&mut self) {
        self.flush_view();
        self.view = None;
        self.restore = None;
        if let Some(mut note) = self.note.take() {
            let _ = note.save(&self.workspace.core, &self.editor_text);
        }
    }

    pub(in crate::app) fn show_page(&self, root: &mut RenderRoot, index: usize) {
        if let Some(chrome) = &self.chrome {
            chrome.pages.edit(root, |mut pages| {
                if pages.widget.active_child() != index {
                    IndexedStack::set_active_child(&mut pages, index);
                }
            });
        }
    }

    pub(crate) fn open(&mut self, root: &mut RenderRoot, path: PathBuf) -> bool {
        self.open_in(root, path, false)
    }

    fn reveal_pending(&mut self, root: &mut RenderRoot) {
        let Some(units) = self.pending_reveal.take() else { return };
        let byte = aquilum_core::documents::merge::utf16_byte(&self.editor_text, usize::try_from(units).unwrap_or(0)).saturating_sub(self.hidden);
        self.reveal(root, byte);
        if let Some(ids) = &self.note_ids {
            ids.body.edit(root, |mut b| TextEditor::select_byte_range(&mut b, byte, byte));
        }
    }

    pub(in crate::app) fn open_in(&mut self, root: &mut RenderRoot, path: PathBuf, new_tab: bool) -> bool {
        self.open_in_with_offset(root, path, new_tab, None)
    }

    pub(crate) fn open_in_with_offset(&mut self, root: &mut RenderRoot, path: PathBuf, new_tab: bool, offset: Option<usize>) -> bool {
        if path.extension().is_none_or(|ext| !ext.eq_ignore_ascii_case("md")) {
            return false;
        }
        if let Some(current) = self.note.as_ref().map(|n| n.path.clone()) {
            self.navigation.push(&current);
        }
        self.navigation.push(&path);
        self.pending_reveal = offset;
        if new_tab {
            self.tabs.open_new(path);
        } else {
            self.tabs.open(path);
        }
        self.show_active(root);
        true
    }

    pub(in crate::app) fn new_tab(&mut self, root: &mut RenderRoot) -> Outcome {
        self.tabs.new_tab();
        self.show_active(root);
        Outcome { title: true, ..Outcome::default() }
    }

    pub(in crate::app) fn close_active_tab(&mut self, root: &mut RenderRoot) -> Outcome {
        self.tabs.close(self.tabs.active_id());
        self.show_active(root);
        Outcome { title: true, ..Outcome::default() }
    }

    pub(in crate::app) fn relocate_tabs(&mut self, root: &mut RenderRoot, from: &Path, to: &Path) {
        if !self.tabs.rename(from, to) {
            return;
        }
        if let Some(session) = &self.session {
            for tab in self.tabs.tabs().iter().filter(|t| t.path.as_deref() == Some(to)) {
                session.renamed(&self.workspace.core, tab);
            }
        }
        self.refresh_tabs(root);
    }

    pub(in crate::app) fn close_missing_tabs(&mut self, root: &mut RenderRoot, removed: &[PathBuf]) {
        let gone: Vec<_> = self
            .tabs
            .tabs()
            .iter()
            .filter(|tab| tab.path.as_ref().is_some_and(|p| removed.contains(p) && !p.exists()))
            .map(|tab| tab.id)
            .collect();
        if gone.is_empty() {
            return;
        }
        for id in gone {
            self.tabs.close(id);
        }
        self.show_active(root);
    }
}
