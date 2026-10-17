use std::path::{Path, PathBuf};

use aquilum_core::search::paths::{is_markdown, is_plain_text};
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
        self.show_tab(root);
        self.version_view = None;
        self.show_version(root);
        self.refresh_tabs(root);
        self.request_panel(root);
        self.workspace.core.active_note.set(self.note.as_ref().map(|n| n.path.clone()));
    }

    fn show_tab(&mut self, root: &mut RenderRoot) {
        let tab = self.tabs.active().clone();
        if tab.graph && self.tree.root().is_some() {
            self.leave_note();
            self.drop_note_page(root);
            self.tree.set_active(root, None);
            self.show_graph(root);
            self.show_page(root, 4);
            return;
        }
        let Some(path) = tab.path else {
            self.leave_note();
            self.drop_note_page(root);
            self.tree.set_active(root, None);
            self.show_page(root, if self.tree.root().is_some() { 1 } else { 3 });
            return;
        };
        if self.shows(&path) {
            self.reveal_pending(root);
            self.show_page(root, 0);
            return;
        }
        self.leave_note();
        let read = aq_trace::zone("открытие: чтение файла");
        let opened = Note::open(path.clone());
        drop(read);
        match opened {
            Ok((note, text)) => {
                self.open_error = None;
                self.open_error_ids = None;
                self.text = text;
                self.note = Some(note);
                let _zone = aq_trace::zone("открытие: страница заметки");
                self.render_note(root);
                self.reveal_pending(root);
            }
            Err(error) => {
                let err_str = format!("{error}");
                report!("заметка {} не прочитана: {err_str}", path.display());
                self.open_error = Some((path.clone(), err_str.clone()));
                self.show_open_error(root, &path, &err_str);
            }
        }
        self.hold = Some(std::time::Instant::now() + super::super::HOLD);
        self.tree.set_active(root, Some(path));
        self.show_page(root, 0);
    }

    pub(in crate::app) fn show_open_error(&mut self, root: &mut RenderRoot, path: &Path, error: &str) {
        self.note = None;
        self.note_ids = None;
        self.embeds = None;
        let (page, ids) = crate::ui::note::open_error_page(path, error);
        self.open_error_ids = Some(ids);
        if let Some(chrome) = &self.chrome {
            chrome.note_page.edit(root, |mut slot| Slot::set_child(&mut slot, page.erased()));
        }
    }

    fn drop_note_page(&mut self, root: &mut RenderRoot) {
        self.embeds = None;
        if self.note_ids.take().is_none() {
            return;
        }
        self.text = crop::Rope::new();
        if let Some(chrome) = &self.chrome {
            chrome.note_page.edit(root, |mut slot| Slot::set_child(&mut slot, masonry::core::NewWidget::new(masonry::widgets::SizedBox::empty()).erased()));
        }
    }

    pub(in crate::app) fn leave_note(&mut self) {
        self.flush_view();
        self.view = None;
        self.restore = None;
        self.commit_note();
        self.note = None;
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
        let byte = self.byte_of(usize::try_from(units).unwrap_or(0)).saturating_sub(self.hidden);
        self.reveal(root, byte);
        if let Some(ids) = &self.note_ids {
            ids.body.edit(root, |mut b| TextEditor::select_byte_range(&mut b, byte, byte));
        }
    }

    pub(in crate::app) fn open_in(&mut self, root: &mut RenderRoot, path: PathBuf, new_tab: bool) -> bool {
        self.open_in_with_offset(root, path, new_tab, None)
    }

    pub(crate) fn open_in_with_offset(&mut self, root: &mut RenderRoot, path: PathBuf, new_tab: bool, offset: Option<usize>) -> bool {
        if !is_markdown(&path) && !is_plain_text(&path) {
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
}
