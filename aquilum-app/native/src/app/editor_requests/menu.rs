use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;

use super::super::{App, Popup};
use crate::frontmatter;
use crate::i18n::t;
use crate::ui::editor::{TableCommand, TextEditor};
use crate::ui::icons;
use crate::ui::menu::{Entry, Menu, MenuChoice};

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
        let entries: Vec<Entry<'_>> = commands.iter().zip(&labels).map(|((c, enabled), label)| Entry { label, icon: Some(c.icon()), disabled: !enabled }).collect();
        let menu = Menu::entries(&t("editor.table.menuAria"), &entries);
        let commands: Vec<TableCommand> = commands.into_iter().map(|(c, _)| c).collect();
        let layer = menu.id();
        root.add_layer(menu, Menu::context_origin(at, entries.len(), self.window));
        root.focus_on(Some(layer));
        self.popup = Some(Popup::TableMenu { layer, commands });
    }

    pub(in crate::app) fn on_table_menu(&mut self, root: &mut RenderRoot, layer: WidgetId, commands: Vec<TableCommand>, action: &ErasedAction) {
        let Some(choice) = action.downcast_ref::<MenuChoice>() else {
            self.popup = Some(Popup::TableMenu { layer, commands });
            return;
        };
        let MenuChoice::Item(item) = choice else { return };
        root.remove_layer(layer);
        let Some(&command) = commands.get(*item) else { return };
        if let Some(ids) = &self.note_ids {
            let body = ids.body.id();
            root.focus_on(Some(body));
            ids.body.edit(root, |mut editor| TextEditor::table_command(&mut editor, command));
        }
    }

    pub(in crate::app) fn open_editor_context_menu(&mut self, root: &mut RenderRoot, at: Point, has_selection: bool) {
        let has_frontmatter = frontmatter::end(&self.editor_text).is_some();
        let commands = vec![
            EditorCommand::Copy,
            EditorCommand::Paste,
            EditorCommand::InsertBook,
            EditorCommand::InsertTable,
            EditorCommand::InsertFrontmatter,
        ];
        let entries = [
            Entry { label: &t("common.copy"), icon: Some(icons::COPY), disabled: !has_selection },
            Entry { label: &t("common.paste"), icon: Some(icons::CLIPBOARD), disabled: false },
            Entry { label: &t("editor.insertBook"), icon: Some(icons::BOOK_OPEN), disabled: false },
            Entry { label: &t("editor.insertTable"), icon: Some(icons::TABLE), disabled: false },
            Entry { label: &t("editor.insertFrontmatter"), icon: Some(icons::TAG), disabled: has_frontmatter },
        ];
        let menu = Menu::entries(&t("editor.actions"), &entries);
        let layer = menu.id();
        root.add_layer(menu, Menu::context_origin(at, entries.len(), self.window));
        root.focus_on(Some(layer));
        self.popup = Some(Popup::EditorMenu { layer, commands });
    }

    pub(in crate::app) fn on_editor_menu(&mut self, root: &mut RenderRoot, layer: WidgetId, commands: Vec<EditorCommand>, action: &ErasedAction) {
        let Some(choice) = action.downcast_ref::<MenuChoice>() else {
            self.popup = Some(Popup::EditorMenu { layer, commands });
            return;
        };
        let MenuChoice::Item(item) = choice else { return };
        root.remove_layer(layer);
        let Some(&command) = commands.get(*item) else { return };
        let Some(ids) = &self.note_ids else { return };
        let body = ids.body;
        root.focus_on(Some(body.id()));
        match command {
            EditorCommand::Copy => {
                let selected = body.get(root).and_then(|e| e.inner().selected_text());
                if let Some(text) = selected
                    && let Ok(mut cb) = arboard::Clipboard::new()
                {
                    let _ = cb.set_text(text);
                }
            }
            EditorCommand::Paste => {
                if let Ok(mut cb) = arboard::Clipboard::new()
                    && let Ok(text) = cb.get_text()
                {
                    body.edit(root, |mut e| TextEditor::insert_text(&mut e, &text));
                }
            }
            EditorCommand::InsertBook => {
                body.edit(root, |mut e| TextEditor::insert_text(&mut e, "> [!book] [[Название книги]]\n"));
            }
            EditorCommand::InsertTable => {
                body.edit(root, |mut e| TextEditor::insert_table(&mut e, 2, 2));
            }
            EditorCommand::InsertFrontmatter => {
                if frontmatter::end(&self.editor_text).is_none() {
                    let new_text = frontmatter::template(&[(frontmatter::TAGS, "[]")]) + &self.editor_text;
                    self.commit_editor_text(root, new_text);
                }
            }
        }
    }
}
