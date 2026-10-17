use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;

use super::super::{App, MENU_MARGIN, Outcome, Popup};
use crate::i18n::t;
use crate::ui::icons;
use crate::ui::menu::{Entry, Menu, MenuChoice};

const ACTIONS_MENU_WIDTH: f64 = 220.0;

#[derive(Clone, Copy)]
enum NoteCommand {
    Wide,
    ExportPdf,
    Reveal,
    CopyPath,
    Duplicate,
    Delete,
    Close,
}

const NOTE_COMMANDS: [NoteCommand; 7] = [NoteCommand::Wide, NoteCommand::ExportPdf, NoteCommand::Reveal, NoteCommand::CopyPath, NoteCommand::Duplicate, NoteCommand::Delete, NoteCommand::Close];

impl NoteCommand {
    fn entry(self, wide: bool) -> (String, icons::Icon) {
        match self {
            NoteCommand::Wide => (t(if wide { "editor.normalWidth" } else { "editor.wideMode" }), icons::MOVE_HORIZONTAL),
            NoteCommand::ExportPdf => (t("editor.exportPdf"), icons::FILE_TEXT),
            NoteCommand::Reveal => (t("editor.revealFile"), icons::FOLDER),
            NoteCommand::CopyPath => (t("editor.copyPath"), icons::CLIPBOARD),
            NoteCommand::Duplicate => (t("common.duplicate"), icons::COPY),
            NoteCommand::Delete => (t("common.delete"), icons::TRASH),
            NoteCommand::Close => (t("editor.closeNote"), icons::X),
        }
    }
}

impl App {
    pub(in crate::app) fn open_note_actions_menu(&mut self, root: &mut RenderRoot, at: Point) {
        let labels: Vec<(String, icons::Icon)> = NOTE_COMMANDS.iter().map(|command| command.entry(self.interface.wide())).collect();
        let entries: Vec<Entry<'_>> = labels.iter().map(|(label, icon)| Entry { label, icon: Some(*icon), disabled: false }).collect();
        let menu = Menu::entries(&t("editor.noteActions"), &entries);
        let estimate = Menu::estimated_size(entries.len());
        let (w, h) = (self.window.width, self.window.height);
        let x = if at.x + ACTIONS_MENU_WIDTH > w { (w - ACTIONS_MENU_WIDTH - MENU_MARGIN).max(MENU_MARGIN) } else { at.x };
        let y = if at.y + estimate.height > h { (at.y - estimate.height).max(MENU_MARGIN) } else { at.y };
        self.show_layer(root, menu, Point::new(x, y), |layer| Popup::NoteActionsMenu { layer });
    }

    pub(in crate::app) fn on_note_actions_menu(&mut self, root: &mut RenderRoot, layer: WidgetId, action: &ErasedAction) -> Outcome {
        let Some(choice) = action.downcast_ref::<MenuChoice>() else {
            self.popup = Some(Popup::NoteActionsMenu { layer });
            return Outcome::default();
        };
        let MenuChoice::Item(item) = choice else { return Outcome::default() };
        root.remove_layer(layer);
        let path = self.note.as_ref().map(|note| note.path.clone());
        match (NOTE_COMMANDS.get(*item), path) {
            (Some(NoteCommand::Wide), _) => self.toggle_wide_mode(root),
            (Some(NoteCommand::ExportPdf), _) => self.export_pdf(root),
            (Some(NoteCommand::Reveal), Some(path)) => crate::system::reveal(&path),
            (Some(NoteCommand::CopyPath), Some(path)) => {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(path.to_string_lossy());
                }
            }
            (Some(NoteCommand::Duplicate), Some(path)) => crate::file_ops::duplicate_all(&self.workspace.core, vec![path]),
            (Some(NoteCommand::Delete), Some(path)) => self.confirm_delete(root, vec![path]),
            (Some(NoteCommand::Close), _) => return self.close_active_tab(root),
            _ => {}
        }
        Outcome::default()
    }
}
