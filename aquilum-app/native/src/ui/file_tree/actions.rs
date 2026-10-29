use std::path::PathBuf;

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;

use super::model::Modifiers;
use super::pinned::Unpin;
use super::row::RowAction;
use super::{FileTree, Press, TreeAction};
use crate::ui::editor::TextAction;
use crate::ui::scroll::Scrolled;
use crate::ui::widgets::Pressed;

impl FileTree {
    pub fn on_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Option<TreeAction> {
        if let Some(Scrolled(offset)) = action.downcast_ref::<Scrolled>() {
            if self.scroll.is_some_and(|s| s.id() == id) {
                self.offset = *offset;
                self.show_rows_from(root, *offset);
                self.update_pins(root);
            }
            return None;
        }
        if let Some(Unpin(index)) = action.downcast_ref::<Unpin>() {
            if self.model.press(*index, Modifiers::default()).is_some() {
                self.save_expanded();
                self.show_row(root, Some(*index));
            }
            return None;
        }
        if Some(id) == self.create_button && action.downcast_ref::<Pressed>().is_some() {
            return Some(TreeAction::CreateNote);
        }
        if let Some(toolbar) = self.toolbar
            && action.downcast_ref::<Pressed>().is_some()
        {
            if id == toolbar.collapse {
                if self.model.collapse_all() {
                    self.save_expanded();
                    self.first = Self::first_near(self.offset);
                    self.refresh(root);
                }
                return None;
            }
            if id == toolbar.clear.id() {
                self.filter(root, "");
                root.focus_on(Some(toolbar.filter.id()));
                return None;
            }
            let folder = self.model.target_folder();
            if id == toolbar.new_note {
                return folder.map(|folder| TreeAction::NewNote { folder });
            }
            if id == toolbar.new_folder {
                return folder.map(|folder| TreeAction::NewFolder { folder });
            }
            if id == toolbar.sort {
                return Some(TreeAction::SortMenu { button: id });
            }
        }
        if let Some(text) = action.downcast_ref::<TextAction>() {
            if self.toolbar.is_some_and(|toolbar| toolbar.filter.id() == id) {
                return match text {
                    TextAction::Changed(query) => {
                        self.set_filter(root, query);
                        None
                    }
                    TextAction::Cancelled => {
                        self.filter(root, "");
                        None
                    }
                    TextAction::Entered(_) | TextAction::Edited(_) | TextAction::Page { .. } | TextAction::Line { .. } => None,
                };
            }
            return match text {
                TextAction::Entered(name) => self.commit_rename(root, name),
                TextAction::Cancelled => {
                    self.model.stop_rename();
                    self.refresh(root);
                    None
                }
                TextAction::Changed(_) | TextAction::Edited(_) | TextAction::Page { .. } | TextAction::Line { .. } => None,
            };
        }
        self.on_row_action(root, id, action.downcast_ref::<RowAction>()?)
    }

    fn on_row_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &RowAction) -> Option<TreeAction> {
        if self.model.renaming().is_some() {
            let name = self.rename.and_then(|field| field.get(root)).map(|f| f.inner().text());
            if let Some(name) = name {
                return self.commit_rename(root, &name);
            }
        }
        let index = self.first + self.row_ids.iter().position(|&row| row == id)?;
        let modifiers = match action {
            RowAction::Press(modifiers) => *modifiers,
            RowAction::DragStart(at) => {
                self.start_drag(root, index, *at);
                return None;
            }
            RowAction::DragMove(at) => {
                self.drag_to(root, *at);
                return None;
            }
            RowAction::Drop(at) => {
                self.drag_to(root, *at);
                return self.drop(root);
            }
            RowAction::ContextMenu(at) => return self.context_menu(root, index, *at),
        };
        match self.model.press(index, modifiers)? {
            Press::Rebuild => {
                self.save_expanded();
                self.refresh(root);
                None
            }
            Press::Open { path, new_tab } => Some(TreeAction::Open { path, new_tab }),
        }
    }

    fn context_menu(&mut self, root: &mut RenderRoot, index: usize, at: Point) -> Option<TreeAction> {
        let row = self.model.rows()[index].clone();
        let note = aquilum_core::search::paths::is_markdown(&row.path) || aquilum_core::search::paths::is_plain_text(&row.path);
        if !row.folder && !note {
            return None;
        }
        let targets = self.model.focus(index);
        self.refresh(root);
        Some(TreeAction::ContextMenu { targets, clicked: row.path, folder: row.folder, at })
    }

    pub fn start_rename(&mut self, root: &mut RenderRoot, path: PathBuf) {
        self.model.start_rename(path);
        self.rename = None;
        self.refresh(root);
        if let Some(field) = self.rename {
            field.focus_all(root);
        }
    }

    fn commit_rename(&mut self, root: &mut RenderRoot, input: &str) -> Option<TreeAction> {
        let from = self.model.renaming()?.to_path_buf();
        self.model.stop_rename();
        let Some((to, name)) = crate::file_ops::rename_target(&from, input) else {
            self.refresh(root);
            return None;
        };
        self.model.patch(&from, &to, &name);
        self.save_expanded();
        self.save_colors();
        self.refresh(root);
        Some(TreeAction::Rename { from, to })
    }
}
