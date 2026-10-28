use masonry::app::RenderRoot;
use masonry::kurbo::Point;

use super::super::{App, MENU_MARGIN, MenuCommand, Outcome, Popup};
use crate::i18n::t;
use crate::ui::editor::TextEditor;
use crate::ui::icons;
use crate::ui::menu::{Entry, Menu};

const ACTIONS_MENU_WIDTH: f64 = 220.0;

#[derive(Clone, Copy, PartialEq)]
pub(in crate::app) enum NoteCommand {
    Undo,
    Redo,
    Wide,
    ExportPdf,
    Reveal,
    CopyPath,
    Duplicate,
    Delete,
    Close,
}

const NOTE_COMMANDS: [NoteCommand; 9] = [
    NoteCommand::Undo,
    NoteCommand::Redo,
    NoteCommand::Wide,
    NoteCommand::ExportPdf,
    NoteCommand::Reveal,
    NoteCommand::CopyPath,
    NoteCommand::Duplicate,
    NoteCommand::Delete,
    NoteCommand::Close,
];

impl NoteCommand {
    fn entry(self, wide: bool) -> (String, icons::Icon) {
        match self {
            NoteCommand::Undo => (t("settings.shortcuts.undo"), icons::UNDO),
            NoteCommand::Redo => (t("settings.shortcuts.redo"), icons::REDO),
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
        let compact = self.compact();
        let commands: Vec<NoteCommand> = NOTE_COMMANDS.into_iter().filter(|&command| !(compact && command == NoteCommand::Wide)).collect();
        let body = self.note_ids.as_ref().and_then(|ids| ids.body.get(root));
        let can = |redo: bool| body.as_ref().is_some_and(|editor| editor.inner().can_undo(redo));
        let disabled: Vec<bool> = commands.iter().map(|command| match command {
            NoteCommand::Undo => !can(false),
            NoteCommand::Redo => !can(true),
            _ => false,
        }).collect();
        let labels: Vec<(String, icons::Icon)> = commands.iter().map(|command| command.entry(self.interface.wide())).collect();
        let divided = |index: usize| index > 0 && commands[index - 1] == NoteCommand::Redo;
        let entries: Vec<Entry<'_>> = labels.iter().zip(disabled).enumerate().map(|(index, ((label, icon), disabled))| Entry { label, icon: Some(*icon), disabled, divided: divided(index) }).collect();
        let menu = Menu::entries(&t("editor.noteActions"), &entries);
        let estimate = Menu::estimated_size(entries.len());
        let (w, h) = (self.window.width, self.window.height);
        let x = if at.x + ACTIONS_MENU_WIDTH > w { (w - ACTIONS_MENU_WIDTH - MENU_MARGIN).max(MENU_MARGIN) } else { at.x };
        let y = if at.y + estimate.height > h { (at.y - estimate.height).max(MENU_MARGIN) } else { at.y };
        let commands = commands.into_iter().map(MenuCommand::Note).collect();
        self.show_layer(root, menu, Point::new(x, y), |layer| Popup::Menu { layer, commands });
    }

    pub(in crate::app) fn note_command(&mut self, root: &mut RenderRoot, command: NoteCommand) -> Outcome {
        let path = self.note.as_ref().map(|note| note.path.clone());
        match (command, path) {
            (NoteCommand::Undo | NoteCommand::Redo, _) => {
                if let Some(ids) = &self.note_ids {
                    ids.body.edit(root, |mut body| TextEditor::undo(&mut body, command == NoteCommand::Redo));
                }
            }
            (NoteCommand::Wide, _) => self.toggle_wide_mode(root),
            (NoteCommand::ExportPdf, _) => self.export_pdf(root),
            (NoteCommand::Reveal, Some(path)) => crate::system::reveal(&path),
            (NoteCommand::CopyPath, Some(path)) => {
                crate::clipboard::copy(path.to_string_lossy());
            }
            (NoteCommand::Duplicate, Some(path)) => crate::file_ops::duplicate_all(&self.workspace.core, vec![path]),
            (NoteCommand::Delete, Some(path)) => self.confirm_delete(root, vec![path]),
            (NoteCommand::Close, _) => return self.close_active_tab(root),
            _ => {}
        }
        Outcome::default()
    }
}
