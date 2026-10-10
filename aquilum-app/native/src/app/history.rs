use std::path::PathBuf;

use masonry::app::RenderRoot;
use crate::ui::editor::TextEditor;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;
use masonry::widgets::{TextAction};

use super::{App, Outcome, Popup};
use crate::history::{self, Version};
use crate::i18n::t;
use crate::ui::backlinks::{self, HistoryRow, VersionView};
use crate::ui::menu::{Menu, MenuChoice};
use crate::ui::widgets::{ItemAction, Pressed, Slot};

pub(super) struct HistoryState {
    path: PathBuf,
    pub(super) versions: Vec<Version>,
    total: usize,
    limit: usize,
    renaming: Option<usize>,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum VersionCommand {
    Restore,
    Revert,
    Rename,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or_default()
}

impl App {
    pub(super) fn load_history(&mut self, root: &mut RenderRoot) {
        let Some(path) = self.tabs.active().path.clone() else {
            self.history = None;
            self.render_history(root);
            return;
        };
        let limit = self.history.as_ref().filter(|h| h.path == path).map_or(history::PAGE_SIZE, |h| h.limit);
        let page = history::page(&self.workspace.core, &path, limit);
        self.history = Some(HistoryState { path, versions: page.versions, total: page.total, limit, renaming: None });
        self.render_history(root);
    }

    pub(super) fn render_history(&mut self, root: &mut RenderRoot) {
        let Some(chrome) = &self.chrome else { return };
        let Some(state) = &self.history else {
            let (empty, _) = backlinks::body(crate::links::Mode::History, &[], crate::links::Method::Bm25f, None);
            chrome.panel_parts.body.edit(root, |mut slot| Slot::set_child(&mut slot, empty));
            self.history_body = None;
            return;
        };
        let selected = self.opened_versions.get(&self.tabs.active_id()).map(|v| v.id.clone());
        let mut days = Vec::new();
        for (label, versions) in history::group_by_day(&state.versions, now_ms()) {
            let rows = versions
                .into_iter()
                .map(|version| {
                    let index = state.versions.iter().position(|v| v.id == version.id).unwrap_or_default();
                    let row = HistoryRow {
                        label: history::label(version),
                        time: crate::dates::time(version.at_ms),
                        selected: selected.as_deref() == Some(version.id.as_str()),
                        renaming: state.renaming == Some(index),
                    };
                    (index, row)
                })
                .collect();
            days.push((label, rows));
        }
        let more = state.versions.len() < state.total;
        let (body, bindings) = backlinks::history_body(selected.is_none(), days, state.total == 0, more);
        chrome.panel_parts.body.edit(root, |mut slot| Slot::set_child(&mut slot, body));
        if let Some(field) = bindings.rename {
            field.edit(root, |mut field| {
                let len = field.widget.text().to_string().len();
                TextEditor::select_byte_range(&mut field, 0, len);
            });
            root.focus_on(Some(field.id()));
        }
        self.history_body = Some(bindings);
    }

    pub(super) fn on_history_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Option<Outcome> {
        if let Some(view) = &self.version_view {
            let pressed = action.downcast_ref::<Pressed>().is_some();
            let command = if !pressed {
                None
            } else if Some(id) == view.revert {
                Some(VersionCommand::Revert)
            } else if id == view.restore {
                Some(VersionCommand::Restore)
            } else if id == view.back {
                self.close_version(root);
                return Some(Outcome::default());
            } else {
                None
            };
            if let Some(command) = command {
                let version = self.opened_versions.get(&self.tabs.active_id())?.clone();
                self.run_version_command(root, &version, command);
                return Some(Outcome::default());
            }
        }
        let body = self.history_body.as_ref()?;
        if let Some(field) = body.rename
            && field.id() == id
        {
            match action.downcast_ref::<TextAction>() {
                Some(TextAction::Entered(name)) => self.finish_version_rename(root, Some(name.clone())),
                Some(TextAction::Cancelled) => self.finish_version_rename(root, None),
                _ => {}
            }
            return Some(Outcome::default());
        }
        if action.downcast_ref::<Pressed>().is_some() && body.more == Some(id) {
            if let Some(state) = &mut self.history {
                state.limit += history::PAGE_SIZE;
            }
            self.load_history(root);
            return Some(Outcome::default());
        }
        let item = action.downcast_ref::<ItemAction>()?;
        if id == body.current {
            if matches!(item, ItemAction::Open { .. }) {
                self.close_version(root);
            }
            return Some(Outcome::default());
        }
        let index = body.rows.iter().find(|(row, _)| *row == id)?.1;
        let version = self.history.as_ref()?.versions.get(index)?.clone();
        match item {
            ItemAction::Open { .. } => self.open_version(root, version),
            ItemAction::Menu(at) => self.open_version_menu(root, index, *at),
        }
        Some(Outcome::default())
    }

    fn open_version_menu(&mut self, root: &mut RenderRoot, index: usize, at: Point) {
        let Some(state) = &self.history else { return };
        let Some(version) = state.versions.get(index) else { return };
        let oldest = state.versions.len() == state.total && index + 1 == state.versions.len();
        let mut commands = Vec::new();
        if !version.is_current {
            commands.push((VersionCommand::Restore, t("history.restore")));
        }
        if !oldest {
            commands.push((VersionCommand::Revert, t("history.revert")));
        }
        commands.push((VersionCommand::Rename, t("common.rename")));
        let labels: Vec<&str> = commands.iter().map(|(_, label)| label.as_str()).collect();
        let menu = Menu::new(&t("history.versionActions"), &labels);
        let layer = menu.id();
        root.add_layer(menu, at);
        root.focus_on(Some(layer));
        self.popup = Some(Popup::VersionMenu { layer, index, commands: commands.into_iter().map(|(c, _)| c).collect() });
    }

    pub(super) fn on_version_menu(&mut self, root: &mut RenderRoot, layer: WidgetId, index: usize, commands: Vec<VersionCommand>, action: &ErasedAction) {
        let Some(choice) = action.downcast_ref::<MenuChoice>() else {
            self.popup = Some(Popup::VersionMenu { layer, index, commands });
            return;
        };
        let MenuChoice::Item(item) = choice else { return };
        root.remove_layer(layer);
        let Some(&command) = commands.get(*item) else { return };
        if command == VersionCommand::Rename {
            if let Some(state) = &mut self.history {
                state.renaming = Some(index);
            }
            self.render_history(root);
            return;
        }
        let Some(version) = self.history.as_ref().and_then(|s| s.versions.get(index)).cloned() else { return };
        self.run_version_command(root, &version, command);
    }

    fn finish_version_rename(&mut self, root: &mut RenderRoot, name: Option<String>) {
        let Some(state) = &mut self.history else { return };
        let Some(index) = state.renaming.take() else { return };
        if let (Some(name), Some(version)) = (name, state.versions.get(index)) {
            let label = history::label(version);
            if name.trim() != label && Some(name.trim()) != version.name.as_deref() {
                history::rename(&self.workspace.core, &state.path, version, &name);
            }
        }
        self.load_history(root);
    }

    fn open_version(&mut self, root: &mut RenderRoot, version: Version) {
        self.opened_versions.insert(self.tabs.active_id(), version);
        self.version_notice = None;
        self.show_version(root);
        self.render_history(root);
    }

    pub(super) fn show_version(&mut self, root: &mut RenderRoot) -> bool {
        let Some(version) = self.opened_versions.get(&self.tabs.active_id()).cloned() else { return false };
        let Some(path) = self.tabs.active().path.clone() else { return false };
        let texts = history::texts(&self.workspace.core, &path, &version.id);
        let (lines, notice, previous) = match &texts {
            Some(texts) => (history::diff(texts.previous.as_deref(), &texts.text), self.version_notice.clone(), texts.previous.is_some()),
            None => (Vec::new(), Some(t("history.missing")), false),
        };
        let can_restore = texts.is_some() && !version.is_current;
        let (view, ids): (_, VersionView) =
            backlinks::version_view(&history::heading(&version), previous, can_restore, &lines, notice.as_deref());
        let Some(chrome) = &self.chrome else { return false };
        chrome.version_page.edit(root, |mut slot| Slot::set_child(&mut slot, view));
        self.version_view = Some(ids);
        self.show_page(root, 2);
        true
    }

    fn close_version(&mut self, root: &mut RenderRoot) {
        self.opened_versions.remove(&self.tabs.active_id());
        self.version_view = None;
        self.show_page(root, 0);
        self.render_history(root);
    }

    fn run_version_command(&mut self, root: &mut RenderRoot, version: &Version, command: VersionCommand) {
        let Some(path) = self.tabs.active().path.clone() else { return };
        if let Some(note) = &mut self.note {
            note.save(&self.workspace.core);
        }
        let result = match command {
            VersionCommand::Restore => history::restore(&self.workspace.core, &path, version),
            VersionCommand::Revert => history::revert(&self.workspace.core, &path, version),
            VersionCommand::Rename => return,
        };
        match result {
            Ok(_) => {
                if let Some(note) = &mut self.note
                    && let Some(text) = note.reload()
                {
                    self.set_editor_text(root, text);
                }
                if self.opened_versions.contains_key(&self.tabs.active_id()) {
                    self.close_version(root);
                }
                self.load_history(root);
            }
            Err(error) => {
                eprintln!("история: {error:?}");
                self.version_notice = Some(t("history.actionFailed"));
                self.show_version(root);
            }
        }
    }
}
