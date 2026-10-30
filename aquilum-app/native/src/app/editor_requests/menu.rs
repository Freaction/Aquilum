use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::{Point, Rect};

use super::super::{App, MenuCommand, Popup};
use crate::frontmatter;
use crate::i18n::t;
use crate::ui::editor::{ContextMenu, TableCommand, TextAction, TextEditor};
use crate::ui::icons;
use crate::ui::menu::{Entry, Menu, MenuChoice};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditorCommand {
    Cut,
    Copy,
    Paste,
    SelectAll,
    InsertBook,
    InsertTable,
    InsertFrontmatter,
}

const SELECTED: &[EditorCommand] = &[EditorCommand::Cut, EditorCommand::Copy, EditorCommand::Paste, EditorCommand::SelectAll];
const CARET: &[EditorCommand] = &[EditorCommand::Paste, EditorCommand::SelectAll, EditorCommand::InsertBook, EditorCommand::InsertTable, EditorCommand::InsertFrontmatter];
const PLAIN: &[EditorCommand] = &[EditorCommand::Paste, EditorCommand::SelectAll];

impl EditorCommand {
    fn entry(self) -> (String, icons::Icon) {
        match self {
            EditorCommand::Cut => (t("common.cut"), icons::SCISSORS),
            EditorCommand::Copy => (t("common.copy"), icons::COPY),
            EditorCommand::Paste => (t("common.paste"), icons::CLIPBOARD),
            EditorCommand::SelectAll => (t("common.selectAll"), icons::TEXT_SELECT),
            EditorCommand::InsertBook => (t("editor.insertBook"), icons::BOOK_OPEN),
            EditorCommand::InsertTable => (t("editor.insertTable"), icons::TABLE),
            EditorCommand::InsertFrontmatter => (t("editor.insertFrontmatter"), icons::TAG),
        }
    }
}

impl App {
    pub(in crate::app) fn open_table_menu(&mut self, root: &mut RenderRoot, at: Point, commands: Vec<(TableCommand, bool)>) {
        let labels: Vec<String> = commands.iter().map(|(c, _)| t(c.label_key())).collect();
        let entries: Vec<Entry<'_>> = commands.iter().zip(&labels).map(|((c, enabled), label)| Entry { label, icon: Some(c.icon()), disabled: !enabled, divided: false }).collect();
        let menu = Menu::entries(&t("editor.table.menuAria"), &entries);
        let commands = commands.into_iter().map(|(c, _)| MenuCommand::Table(c)).collect();
        self.show_menu(root, menu, Rect::from_points(at, at), |layer| Popup::Menu { layer, commands });
    }

    pub(in crate::app) fn table_command(&mut self, root: &mut RenderRoot, command: TableCommand) {
        if let Some(ids) = &self.note_ids {
            let body = ids.body.id();
            root.focus_on(Some(body));
            ids.body.edit(root, |mut editor| TextEditor::table_command(&mut editor, command));
        }
    }

    pub(in crate::app) fn close_editor_menu(&mut self, root: &mut RenderRoot) {
        if let Some(editor) = self.remove_editor_menu(root)
            && root.get_widget(editor).is_some()
        {
            root.edit_widget(editor, |mut w| TextEditor::menu_closed(&mut w.downcast()));
        }
    }

    fn remove_editor_menu(&mut self, root: &mut RenderRoot) -> Option<WidgetId> {
        let (layer, editor, _) = self.editor_menu.take()?;
        root.remove_layer(layer);
        Some(editor)
    }

    fn show_editor_menu(&mut self, root: &mut RenderRoot, editor: WidgetId, menu: Option<ContextMenu>) {
        self.remove_editor_menu(root);
        let Some(menu) = menu else { return };
        let commands = match (menu.selected, menu.inserts) {
            (true, _) => SELECTED,
            (false, true) => CARET,
            (false, false) => PLAIN,
        };
        let has_frontmatter = self.has_frontmatter();
        let labels: Vec<(String, icons::Icon)> = commands.iter().map(|command| command.entry()).collect();
        let entries: Vec<Entry<'_>> = commands.iter().zip(&labels).map(|(&command, (label, icon))| Entry { label, icon: Some(*icon), disabled: command == EditorCommand::InsertFrontmatter && has_frontmatter, divided: false }).collect();
        let widget = Menu::holding(Menu::entries(&t("editor.actions"), &entries), editor);
        let layer = widget.id();
        root.add_layer(widget, Point::ZERO);
        self.editor_menu = Some((layer, editor, commands));
        self.place_layer(root, layer, menu.around);
    }

    pub(in crate::app) fn on_editor_menu(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> bool {
        if let Some(TextAction::Menu(menu)) = action.downcast_ref::<TextAction>() {
            self.show_editor_menu(root, id, *menu);
            return true;
        }
        let Some((layer, editor, commands)) = self.editor_menu else { return false };
        if !root.get_widget(layer).is_some_and(|menu| menu.find_widget_by_id(id).is_some()) {
            return false;
        }
        if let Some(MenuChoice::Item(index)) = action.downcast_ref::<MenuChoice>() {
            self.close_editor_menu(root);
            if let Some(&command) = commands.get(*index) {
                self.editor_command(root, editor, command);
            }
        }
        true
    }

    fn editor_command(&mut self, root: &mut RenderRoot, editor: WidgetId, command: EditorCommand) {
        let edit = |root: &mut RenderRoot, change: &dyn Fn(&mut masonry::core::WidgetMut<'_, TextEditor>)| root.edit_widget(editor, |mut w| change(&mut w.downcast()));
        match command {
            EditorCommand::Cut | EditorCommand::Copy => edit(root, &|e| TextEditor::to_clipboard(e, command == EditorCommand::Cut)),
            EditorCommand::Paste => {
                self.paste(root, false);
            }
            EditorCommand::SelectAll => edit(root, &TextEditor::select_everything),
            EditorCommand::InsertBook => edit(root, &|e| TextEditor::insert_text(e, "> [!book] [[Название книги]]\n")),
            EditorCommand::InsertTable => edit(root, &|e| TextEditor::insert_table(e, 2, 2)),
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
