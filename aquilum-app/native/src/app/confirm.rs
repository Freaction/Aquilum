use std::path::{Path, PathBuf};

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;

use super::{App, Popup};
use crate::file_ops;
use crate::i18n::{t, t_with};
use crate::ui::dialog::{ConfirmDialog, DialogAnswer, DialogIds};

pub(in crate::app) enum Confirmed {
    Delete(Vec<PathBuf>),
    ResetGraph,
    ManualUpdate(String),
}

impl App {
    pub(in crate::app) fn ask(&mut self, root: &mut RenderRoot, then: Confirmed, title: &str, description: &str, cancel: &str, confirm: &str) {
        let (layer, dialog) = ConfirmDialog::new(title, description, cancel, confirm);
        self.show_layer(root, layer, Point::ORIGIN, |_| Popup::Confirm(dialog, then));
    }

    pub(in crate::app) fn on_confirm(&mut self, root: &mut RenderRoot, dialog: DialogIds, then: Confirmed, id: WidgetId, action: &ErasedAction) {
        let Some(answer) = dialog.answer(id, action) else {
            self.popup = Some(Popup::Confirm(dialog, then));
            return;
        };
        crate::ui::dialog::close(root, dialog.layer);
        if answer != DialogAnswer::Confirm {
            return;
        }
        match then {
            Confirmed::Delete(targets) => {
                let Some(workspace) = self.tree.root().map(Path::to_path_buf) else { return };
                self.commit_note();
                file_ops::trash_all(&self.workspace.core, workspace, targets);
                self.tree.clear_selection(root);
            }
            Confirmed::ResetGraph => self.reset_graph_settings(root),
            Confirmed::ManualUpdate(url) => crate::system::open_url(&url),
        }
    }

    pub(in crate::app) fn confirm_delete(&mut self, root: &mut RenderRoot, targets: Vec<PathBuf>) {
        let many = targets.len() > 1;
        let folder = !many && targets.first().is_some_and(|p| p.is_dir());
        let name = targets.first().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let count = targets.len().to_string();
        let (title, description) = if many {
            (t("confirm.notesTitle"), t_with("confirm.manyDescription", &[("count", &count)]))
        } else if folder {
            (t("confirm.folderTitle"), t_with("confirm.folderDescription", &[("name", &name)]))
        } else {
            (t("confirm.noteTitle"), t_with("confirm.noteDescription", &[("name", &name)]))
        };
        let confirm = if many { t_with("confirm.deleteMany", &[("count", &count)]) } else { t("common.delete") };
        self.ask(root, Confirmed::Delete(targets), &title, &description, &t("common.cancel"), &confirm);
    }
}
