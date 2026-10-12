mod actions;
mod version;

use std::path::PathBuf;

use masonry::app::RenderRoot;
use crate::ui::editor::TextEditor;

use super::App;
use crate::history::{self, Version};
use crate::ui::backlinks::{self, HistoryRow};
use crate::ui::widgets::Slot;

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
        for (label, versions) in history::group_by_day(&state.versions, crate::dates::now_ms() as u64) {
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
}
