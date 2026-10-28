use masonry::app::RenderRoot;
use masonry::kurbo::Point;

use super::super::{App, MenuCommand, Popup};
use crate::frontmatter;
use crate::i18n::t;
use crate::ui::editor::{TableCommand, TextEditor};
use crate::ui::icons;
use crate::ui::menu::{Entry, Menu};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditorCommand {
    Copy,
    Paste,
    InsertBook,
    InsertTable,
    InsertFrontmatter,
}

impl App {
    pub(in crate::app) fn open_table_menu(&mut self, root: &mut RenderRoot, at: Point, commands: Vec<(TableCommand, bool)>) {
        let labels: Vec<String> = commands.iter().map(|(c, _)| t(c.label_key())).collect();
        let entries: Vec<Entry<'_>> = commands.iter().zip(&labels).map(|((c, enabled), label)| Entry { label, icon: Some(c.icon()), disabled: !enabled, divided: false }).collect();
        let menu = Menu::entries(&t("editor.table.menuAria"), &entries);
        let commands = commands.into_iter().map(|(c, _)| MenuCommand::Table(c)).collect();
        self.show_layer(root, menu, Menu::context_origin(at, entries.len(), self.window), |layer| Popup::Menu { layer, commands });
    }

    pub(in crate::app) fn table_command(&mut self, root: &mut RenderRoot, command: TableCommand) {
        if let Some(ids) = &self.note_ids {
            let body = ids.body.id();
            root.focus_on(Some(body));
            ids.body.edit(root, |mut editor| TextEditor::table_command(&mut editor, command));
        }
    }

    pub(in crate::app) fn open_editor_context_menu(&mut self, root: &mut RenderRoot, at: Point, has_selection: bool) {
        let has_frontmatter = self.has_frontmatter();
        let commands = [
            EditorCommand::Copy,
            EditorCommand::Paste,
            EditorCommand::InsertBook,
            EditorCommand::InsertTable,
            EditorCommand::InsertFrontmatter,
        ]
        .map(MenuCommand::Editor)
        .to_vec();
        let entries = [
            Entry { label: &t("common.copy"), icon: Some(icons::COPY), disabled: !has_selection, divided: false },
            Entry { label: &t("common.paste"), icon: Some(icons::CLIPBOARD), disabled: false, divided: false },
            Entry { label: &t("editor.insertBook"), icon: Some(icons::BOOK_OPEN), disabled: false, divided: false },
            Entry { label: &t("editor.insertTable"), icon: Some(icons::TABLE), disabled: false, divided: false },
            Entry { label: &t("editor.insertFrontmatter"), icon: Some(icons::TAG), disabled: has_frontmatter, divided: false },
        ];
        let menu = Menu::entries(&t("editor.actions"), &entries);
        self.show_layer(root, menu, Menu::context_origin(at, entries.len(), self.window), |layer| Popup::Menu { layer, commands });
    }

    pub(in crate::app) fn editor_command(&mut self, root: &mut RenderRoot, command: EditorCommand) {
        let Some(ids) = &self.note_ids else { return };
        let body = ids.body;
        root.focus_on(Some(body.id()));
        match command {
            EditorCommand::Copy => {
                let selected = body.get(root).and_then(|e| e.inner().selected_text());
                if let Some(text) = selected {
                    crate::clipboard::copy(text);
                }
            }
            EditorCommand::Paste => {
                self.paste(root);
            }
            EditorCommand::InsertBook => {
                body.edit(root, |mut e| TextEditor::insert_text(&mut e, "> [!book] [[Название книги]]\n"));
            }
            EditorCommand::InsertTable => {
                body.edit(root, |mut e| TextEditor::insert_table(&mut e, 2, 2));
            }
            EditorCommand::InsertFrontmatter => {
                if !self.has_frontmatter() {
                    let mut text = self.text.clone();
                    text.insert(0, frontmatter::template(&[(frontmatter::TAGS, "[]")]));
                    self.commit_text(root, text);
                }
            }
        }
    }
}
