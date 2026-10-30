use std::path::{Path, PathBuf};

use aquilum_core::search::paths::{is_markdown, is_plain_text, same_path};
use masonry::app::RenderRoot;
use masonry::widgets::IndexedStack;

use super::super::{App, Outcome};
use crate::ui::editor::TextEditor;
use crate::ui::widgets::Slot;

impl App {
    pub(in crate::app) fn show_active(&mut self, root: &mut RenderRoot) {
        self.set_drawer(root, None);
        self.show_suggest(root, None);
        self.close_editor_menu(root);
        let tab = self.tabs.active();
        let stays = if tab.graph { self.graph_shown() } else { tab.path.as_deref().is_some_and(|path| self.shows(path)) };
        if !self.show_tab(root) {
            self.refresh_tabs(root);
            return;
        }
        if !stays {
            self.forget(super::super::Moment::Tab, root);
        }
        self.show_version(root);
        self.refresh_tabs(root);
        self.request_panel(root);
        self.workspace.core.active_note.set(self.note.as_ref().map(|n| n.path.clone()));
    }

    fn show_tab(&mut self, root: &mut RenderRoot) -> bool {
        let tab = self.tabs.active().clone();
        let opened = self.opened(tab.path.as_deref().filter(|path| !self.shows(path)));
        if tab.graph && self.tree.root().is_some() {
            self.leave_note();
            self.drop_note_page(root);
            self.tree.set_active(root, None);
            self.show_graph(root);
            self.show_page(root, 4);
            return true;
        }
        let Some(path) = tab.path else {
            self.leave_note();
            self.drop_note_page(root);
            self.tree.set_active(root, None);
            self.show_page(root, if self.tree.root().is_some() { 1 } else { 3 });
            return true;
        };
        if self.shows(&path) {
            self.reveal_pending(root, &path);
            self.show_page(root, 0);
            return true;
        }
        let Some(opened) = opened else { return false };
        self.leave_note();
        match opened {
            Ok((note, text)) => {
                self.open_error = None;
                self.open_error_ids = None;
                self.text = text;
                self.note = Some(note);
                let _zone = aq_trace::zone("открытие: страница заметки");
                self.render_note(root);
                self.reveal_pending(root, &path);
            }
            Err(error) => {
                let err_str = format!("{error}");
                report!("заметка {} не прочитана: {err_str}", path.display());
                self.open_error = Some((path.clone(), err_str.clone()));
                self.show_open_error(root, &path, &err_str);
            }
        }
        self.tree.set_active(root, Some(path));
        self.show_page(root, 0);
        true
    }

    pub(in crate::app) fn show_open_error(&mut self, root: &mut RenderRoot, path: &Path, error: &str) {
        self.text = crop::Rope::new();
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
            chrome.note_page.edit(root, |mut slot| Slot::clear(&mut slot));
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

    fn reveal_pending(&mut self, root: &mut RenderRoot, path: &Path) {
        let Some((_, units)) = self.pending_reveal.take().filter(|(target, _)| same_path(target, path)) else { return };
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
        let current = self.note.as_ref().map(|n| n.path.clone());
        self.pending_reveal = offset.map(|offset| (path.clone(), offset));
        if new_tab {
            self.tabs.open_new(path.clone());
        } else {
            self.tabs.open(path.clone());
        }
        self.show_active(root);
        for visited in current.iter().chain([&path]) {
            self.navigation.push(visited);
        }
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
