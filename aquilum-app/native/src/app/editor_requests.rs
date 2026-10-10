use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;

use super::{App, MENU_MARGIN, Popup};
use crate::i18n::t;
use crate::ui::editor::resolve::Request;
use crate::ui::editor::{TableCommand, TextEditor};
use crate::ui::menu::{Entry, Menu, MenuChoice};

impl App {
    pub(super) fn on_editor_request(&mut self, root: &mut RenderRoot, request: Request) -> bool {
        match request {
            Request::TableMenu { at, commands } => {
                self.open_table_menu(root, at, commands);
                false
            }
            Request::Suggest(view) => {
                self.show_suggest(root, view);
                false
            }
            Request::Read { file, page, title } => {
                self.open_reader(root, super::reader::Opening { file, page, title, initial: None, peek: false });
                false
            }
            Request::ToggleTask { target, line } => {
                self.toggle_task_in_file(&target, line);
                false
            }
            Request::View(image) => {
                self.open_viewer(root, image);
                false
            }
            Request::Open { target, wiki, new_tab } => {
                let target = if wiki { crate::links::Target::Wiki(target) } else { crate::links::Target::Url(target) };
                self.open_target(root, target, new_tab)
            }
        }
    }

    fn show_suggest(&mut self, root: &mut RenderRoot, view: Option<crate::ui::editor::resolve::Suggestions>) {
        let Some(view) = view else {
            if let Some((layer, _)) = self.suggest_layer.take() {
                root.remove_layer(layer);
            }
            return;
        };
        let Some(transform) = root.get_widget(view.editor).map(|w| w.ctx().window_transform()) else { return };
        let caret = transform.transform_rect_bbox(view.caret);
        let size = crate::ui::suggest::SuggestList::size_for(&view.items);
        let gap = crate::ui::tokens::size::GAP_XS * 2.0;
        let below = caret.y1 + gap;
        let y = if below + size.height > self.window.height && caret.y0 - gap - size.height > 0.0 { caret.y0 - gap - size.height } else { below };
        let x = (caret.x0 - crate::ui::tokens::size::PADDING_XS - crate::ui::tokens::size::GAP_XS).min(self.window.width - size.width - MENU_MARGIN).max(MENU_MARGIN);
        let at = Point::new(x.round(), y.round());
        match self.suggest_layer {
            Some((layer, _)) => {
                root.edit_widget(layer, |mut w| crate::ui::suggest::SuggestList::set(&mut w.downcast(), view.items, view.query, view.selected));
                root.reposition_layer(layer, at);
                self.suggest_layer = Some((layer, view.editor));
            }
            None => {
                let list = crate::ui::suggest::SuggestList::new(view.items, view.query, view.selected);
                let layer = list.id();
                root.add_layer(list, at);
                self.suggest_layer = Some((layer, view.editor));
            }
        }
    }

    pub(super) fn on_suggest_pick(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> bool {
        let Some((layer, editor)) = self.suggest_layer else { return false };
        if id != layer {
            return false;
        }
        if let Some(picked) = action.downcast_ref::<crate::ui::suggest::SuggestPicked>() {
            root.focus_on(Some(editor));
            root.edit_widget(editor, |mut w| TextEditor::accept_suggestion(&mut w.downcast(), picked.0));
        }
        true
    }

    fn toggle_task_in_file(&mut self, target: &str, line: usize) {
        let Some(root) = self.tree.root() else { return };
        let path = root.join(target);
        let Ok(snapshot) = aquilum_core::files::document::read_file_snapshot_impl(&path) else { return };
        let Some(text) = toggle_task_line(&snapshot.content, line) else { return };
        if let Err(error) = aquilum_core::files::gate::write(&self.workspace.core, &path, &text, Some(&snapshot.hash), aquilum_core::history::Source::Me, None) {
            eprintln!("{} не сохранён: {error:?}", path.display());
            return;
        }
        if let Some(embeds) = &self.embeds {
            embeds.forget_queries();
        }
    }

    fn open_viewer(&mut self, root: &mut RenderRoot, image: masonry::peniko::ImageBrush) {
        let viewer = crate::ui::viewer::Viewer::new(image);
        let layer = viewer.id();
        root.add_layer(viewer, Point::ORIGIN);
        root.focus_on(Some(layer));
        self.popup = Some(Popup::Viewer { layer });
    }

    fn open_table_menu(&mut self, root: &mut RenderRoot, at: Point, commands: Vec<(TableCommand, bool)>) {
        let labels: Vec<String> = commands.iter().map(|(c, _)| t(c.label_key())).collect();
        let entries: Vec<Entry<'_>> = commands.iter().zip(&labels).map(|((c, enabled), label)| Entry { label, icon: Some(c.icon()), disabled: !enabled }).collect();
        let menu = Menu::entries(&t("editor.table.menuAria"), &entries);
        let estimate = Menu::estimated_size(entries.len());
        let commands: Vec<TableCommand> = commands.into_iter().map(|(c, _)| c).collect();
        let (w, h) = (self.window.width, self.window.height);
        let x = if at.x + estimate.width > w { (at.x - estimate.width).max(MENU_MARGIN) } else { at.x };
        let y = if at.y + estimate.height > h { (at.y - estimate.height).max(MENU_MARGIN) } else { at.y };
        let layer = menu.id();
        root.add_layer(menu, Point::new(x, y));
        root.focus_on(Some(layer));
        self.popup = Some(Popup::TableMenu { layer, commands });
    }

    pub(super) fn on_table_menu(&mut self, root: &mut RenderRoot, layer: WidgetId, commands: Vec<TableCommand>, action: &ErasedAction) {
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
}

fn toggle_task_line(text: &str, line: usize) -> Option<String> {
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let current = lines.get_mut(line.checked_sub(1)?)?;
    let indent = current.len() - current.trim_start_matches([' ', '\t']).len();
    let rest = &current[indent..];
    let marker = if rest.starts_with("- ") || rest.starts_with("* ") || rest.starts_with("+ ") {
        2
    } else {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 || !matches!(rest.as_bytes().get(digits..digits + 2), Some(b". " | b") ")) {
            return None;
        }
        digits + 2
    };
    let after = &rest[marker..];
    let spaces = after.len() - after.trim_start().len();
    let at = indent + marker + spaces;
    let bytes = current.as_bytes();
    if bytes.get(at) != Some(&b'[') || bytes.get(at + 2) != Some(&b']') || !current.is_char_boundary(at + 2) {
        return None;
    }
    let done = !bytes[at + 1].is_ascii_whitespace();
    current.replace_range(at + 1..at + 2, if done { " " } else { "x" });
    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn toggles_task_lines() {
        assert_eq!(super::toggle_task_line("a\n- [ ] дело", 2).as_deref(), Some("a\n- [x] дело"));
        assert_eq!(super::toggle_task_line("\t1. [x] b", 1).as_deref(), Some("\t1. [ ] b"));
        assert_eq!(super::toggle_task_line("просто текст", 1), None);
    }
}
