pub mod menu;
pub mod suggest;

pub use menu::EditorCommand;

use std::sync::Arc;

use masonry::app::RenderRoot;
use masonry::kurbo::Point;

use super::{App, Popup};
use crate::ui::editor::resolve::Request;
use crate::ui::editor::TextEditor;

impl App {
    pub(in crate::app) fn on_editor_request(&mut self, root: &mut RenderRoot, request: Request) -> bool {
        match request {
            Request::TableMenu { at, commands } => {
                self.open_table_menu(root, at, commands);
                false
            }
            Request::ContextMenu { at, has_selection } => {
                self.open_editor_context_menu(root, at, has_selection);
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
            Request::View(image, path) => {
                let sharp = path.and_then(|p| std::fs::read(p).ok()).and_then(|bytes| crate::images::decode(&bytes));
                self.open_viewer(root, sharp.unwrap_or(image));
                false
            }
            Request::LinkTitle { editor, placeholder, url } => {
                let (sender, wake) = (self.titles_tx.clone(), Arc::clone(&self.workspace.wake));
                std::thread::spawn(move || {
                    if sender.send((editor, placeholder, aquilum_core::link_title::fetch::page_title(&url))).is_ok() {
                        wake();
                    }
                });
                false
            }
            Request::Open { target, wiki, new_tab } => {
                let target = if wiki { crate::links::Target::Wiki(target) } else { crate::links::Target::Url(target) };
                self.open_target(root, target, new_tab)
            }
        }
    }

    pub(in crate::app) fn on_link_titles(&mut self, root: &mut RenderRoot) {
        while let Ok((editor, placeholder, title)) = self.titles_rx.try_recv() {
            root.edit_widget(editor, |mut w| TextEditor::resolve_link_title(&mut w.downcast(), &placeholder, &title));
        }
    }

    fn toggle_task_in_file(&mut self, target: &str, line: usize) {
        let Some(root) = self.tree.root() else { return };
        let path = root.join(target);
        let Ok(snapshot) = aquilum_core::files::document::read_file_snapshot_impl(&path) else { return };
        let Some(text) = toggle_task_line(&snapshot.content, line) else { return };
        if let Err(error) = aquilum_core::files::gate::write(&self.workspace.core, &path, &text, Some(&snapshot.hash), aquilum_core::history::Source::Me, None) {
            report!("{} не сохранён: {error:?}", path.display());
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
