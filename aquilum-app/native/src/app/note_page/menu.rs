use masonry::app::RenderRoot;
use masonry::kurbo::Rect;

use super::super::{App, MenuCommand, Outcome, Popup};
use crate::i18n::t;
use crate::ui::book::CoverAction;
use crate::ui::editor::TextEditor;
use crate::ui::icons;
use crate::ui::menu::{Entry, Menu};

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
    pub(in crate::app) fn open_note_actions_menu(&mut self, root: &mut RenderRoot, trigger: Rect) {
        let compact = self.compact();
        let fields = self.fields().unwrap_or_default();
        let covers = if fields.has_page_cover() { Vec::new() } else { CoverAction::list(None, compact && fields.is_book()) };
        let shown = |command: NoteCommand| !((compact && command == NoteCommand::Wide) || (!crate::system::DESKTOP && command == NoteCommand::Reveal));
        let body = self.note_ids.as_ref().and_then(|ids| ids.body.get(root));
        let can = |redo: bool| body.as_ref().is_some_and(|editor| editor.inner().can_undo(redo));
        let wide = self.interface.wide();
        let notes = NOTE_COMMANDS.into_iter().filter(|&command| shown(command)).map(|command| {
            let disabled = match command {
                NoteCommand::Undo => !can(false),
                NoteCommand::Redo => !can(true),
                _ => false,
            };
            (MenuCommand::Note(command), command.entry(wide), disabled)
        });
        let items: Vec<(MenuCommand, (String, icons::Icon), bool)> = covers.into_iter().map(|action| (MenuCommand::Cover(action), action.entry(), false)).chain(notes).collect();
        let divided = |index: usize| index > 0 && matches!((items[index - 1].0, items[index].0), (MenuCommand::Note(NoteCommand::Redo), _) | (MenuCommand::Cover(_), MenuCommand::Note(_)));
        let entries: Vec<Entry<'_>> = items.iter().enumerate().map(|(index, (_, (label, icon), disabled))| Entry { label, icon: Some(*icon), disabled: *disabled, divided: divided(index) }).collect();
        let menu = Menu::entries(&t("editor.noteActions"), &entries);
        let commands = items.iter().map(|&(command, ..)| command).collect();
        self.show_menu(root, menu, trigger, |layer| Popup::Menu { layer, commands });
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
