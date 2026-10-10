use std::path::{Path, PathBuf};

use masonry::app::RenderRoot;
use crate::ui::editor::TextEditor;
use masonry::core::{NewWidget, Widget};
use masonry::kurbo::Size;
use masonry::widgets::{IndexedStack};

use super::{App, Outcome, WindowCommand};
use crate::i18n::t;
use crate::note::Note;
use crate::file_ops;
use crate::ui::titlebar::{TabButton, TabRename, TabStrip, TitlebarAction, WindowButton};

impl App {
    pub(super) fn tab_widgets(&mut self) -> Vec<NewWidget<dyn Widget>> {
        let active = self.tabs.active_id();
        self.rename_field = None;
        let mut widgets = Vec::new();
        for tab in self.tabs.tabs() {
            let title = if tab.graph { t("tabs.graph") } else { tab.title().unwrap_or_else(|| t("tabs.newTab")) };
            if self.renaming_tab == Some(tab.id) {
                let (widget, field) = TabRename::new(&title);
                self.rename_field = Some(field);
                widgets.push(widget.erased());
            } else {
                let icon = if tab.graph {
                    Some(crate::ui::icons::NETWORK)
                } else if tab.is_base() {
                    Some(crate::ui::icons::COLUMNS)
                } else {
                    None
                };
                widgets.push(TabButton::new(tab.id, &title, tab.id == active, tab.path.is_some(), icon).erased());
            }
        }
        widgets
    }

    pub(super) fn refresh_tabs(&mut self, root: &mut RenderRoot) {
        self.release_graph(root);
        if let Some(session) = &mut self.session {
            session.save(&self.workspace.core, &mut self.tabs);
        }
        let widgets = self.tab_widgets();
        if let Some(chrome) = &self.chrome {
            chrome.tab_strip.edit(root, |mut strip| TabStrip::set_tabs(&mut strip, widgets));
        }
        if let Some(field) = self.rename_field {
            field.edit(root, |mut field| {
                let len = field.widget.text().to_string().len();
                TextEditor::select_byte_range(&mut field, 0, len);
            });
            root.focus_on(Some(field.id()));
        }
    }

    pub(super) fn start_tab_rename(&mut self, root: &mut RenderRoot, path: &Path) {
        let Some(tab) = self.tabs.tabs().iter().find(|t| t.path.as_deref() == Some(path)) else { return };
        self.renaming_tab = Some(tab.id);
        self.refresh_tabs(root);
    }

    pub(super) fn finish_tab_rename(&mut self, root: &mut RenderRoot, input: Option<&str>) {
        let Some(id) = self.renaming_tab.take() else { return };
        let field = self.rename_field.take();
        let input = input.map(str::to_owned).or_else(|| field.and_then(|f| f.get(root)).map(|f| f.inner().text().to_string()));
        let path = self.tabs.tabs().iter().find(|t| t.id == id).and_then(|t| t.path.clone());
        if let (Some(input), Some(from)) = (input, path)
            && let Some((to, _)) = file_ops::rename_target(&from, &input)
        {
            file_ops::rename(&self.workspace.core, from, to);
        }
        self.refresh_tabs(root);
    }

    pub(super) fn cancel_tab_rename(&mut self, root: &mut RenderRoot) {
        self.rename_field = None;
        if self.renaming_tab.take().is_some() {
            self.refresh_tabs(root);
        }
    }

    pub(super) fn is_tab_rename_field(&self, id: masonry::core::WidgetId) -> bool {
        self.rename_field.is_some_and(|field| field.id() == id)
    }

    pub(super) fn show_active(&mut self, root: &mut RenderRoot) {
        if let Some((layer, _)) = self.suggest_layer.take() {
            root.remove_layer(layer);
        }
        if !self.tabs.active().is_base() {
            self.invalidate_kanban();
        }
        for _ in 0..self.tabs.tabs().len().max(1) {
            let tab = self.tabs.active().clone();
            if tab.graph && self.tree.root().is_some() {
                self.leave_note();
                self.tree.set_active(root, None);
                self.show_graph(root);
                self.show_page(root, 4);
                break;
            }
            if tab.is_base() {
                let Some(path) = tab.path.clone() else { break };
                self.leave_note();
                self.tree.set_active(root, Some(path));
                self.show_kanban(root);
                self.show_page(root, 5);
                break;
            }
            let Some(path) = tab.path else {
                self.leave_note();
                self.tree.set_active(root, None);
                self.show_page(root, if self.tree.root().is_some() { 1 } else { 3 });
                break;
            };
            if self.note.as_ref().is_some_and(|n| n.is(&path)) {
                self.show_page(root, 0);
                break;
            }
            self.leave_note();
            match Note::open(path.clone()) {
                Ok((note, text)) => {
                    self.editor_text = text;
                    self.note = Some(note);
                    self.render_note(root);
                    self.hold = Some(std::time::Instant::now() + super::HOLD);
                    self.tree.set_active(root, Some(path));
                    self.show_page(root, 0);
                    break;
                }
                Err(error) => {
                    eprintln!("заметка {} не прочитана: {error:?}", path.display());
                    self.tabs.close(tab.id);
                }
            }
        }
        self.version_view = None;
        self.show_version(root);
        self.refresh_tabs(root);
        self.request_panel(root);
        self.workspace.core.active_note.set(self.note.as_ref().map(|n| n.path.clone()));
    }

    pub(super) fn leave_note(&mut self) {
        self.flush_view();
        self.view = None;
        self.restore = None;
        if let Some(mut note) = self.note.take() {
            note.save(&self.workspace.core);
        }
    }

    pub(super) fn show_page(&self, root: &mut RenderRoot, index: usize) {
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

    pub(super) fn open_in(&mut self, root: &mut RenderRoot, path: PathBuf, new_tab: bool) -> bool {
        if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("base")) {
            return self.open_base(root, path, new_tab);
        }
        if path.extension().is_none_or(|ext| !ext.eq_ignore_ascii_case("md")) {
            return false;
        }
        if let Some(current) = self.note.as_ref().map(|n| n.path.clone()) {
            self.navigation.push(&current);
        }
        self.navigation.push(&path);
        if new_tab {
            self.tabs.open_new(path);
        } else {
            self.tabs.open(path);
        }
        self.show_active(root);
        true
    }

    pub(super) fn new_tab(&mut self, root: &mut RenderRoot) -> Outcome {
        self.tabs.new_tab();
        self.show_active(root);
        Outcome { title: true, ..Outcome::default() }
    }

    pub(super) fn close_active_tab(&mut self, root: &mut RenderRoot) -> Outcome {
        self.tabs.close(self.tabs.active_id());
        self.show_active(root);
        Outcome { title: true, ..Outcome::default() }
    }

    pub(super) fn on_titlebar(&mut self, root: &mut RenderRoot, action: &TitlebarAction, window_size: Size) -> Outcome {
        if self.renaming_tab.is_some() {
            self.finish_tab_rename(root, None);
        }
        let window = |command| Outcome { window: Some(command), ..Outcome::default() };
        match action {
            TitlebarAction::Menu(id, at) => {
                let Some(path) = self.tabs.tabs().iter().find(|t| t.id == *id).and_then(|t| t.path.clone()) else {
                    return Outcome::default();
                };
                self.open_file_menu(root, vec![path.clone()], path, false, *at, window_size, Some(*id));
                Outcome::default()
            }
            TitlebarAction::Reorder(from, to) => {
                if self.tabs.reorder(*from, *to) {
                    self.refresh_tabs(root);
                }
                Outcome::default()
            }
            TitlebarAction::Select(id) => {
                if self.tabs.select(*id) {
                    self.show_active(root);
                }
                Outcome { title: true, ..Outcome::default() }
            }
            TitlebarAction::Close(id) => {
                self.tabs.close(*id);
                self.show_active(root);
                Outcome { title: true, ..Outcome::default() }
            }
            TitlebarAction::Drag => window(WindowCommand::Drag),
            TitlebarAction::ToggleMaximize => window(WindowCommand::ToggleMaximize),
            TitlebarAction::Minimize => window(WindowCommand::Minimize),
            TitlebarAction::CloseWindow => window(WindowCommand::Close),
        }
    }

    pub(super) fn relocate_tabs(&mut self, root: &mut RenderRoot, from: &Path, to: &Path) {
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

    pub(super) fn close_missing_tabs(&mut self, root: &mut RenderRoot, removed: &[PathBuf]) {
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

    pub fn set_maximized(&mut self, root: &mut RenderRoot, maximized: bool) {
        if self.maximized == maximized {
            return;
        }
        self.maximized = maximized;
        if let Some((_, button)) = self.controls {
            button.edit(root, |mut button| WindowButton::set_maximized(&mut button, maximized));
        }
    }
}
