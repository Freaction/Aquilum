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
                self.toggle_task_in_file(root, &target, line);
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

    fn toggle_task_in_file(&mut self, root: &mut RenderRoot, target: &str, line: usize) {
        let Some(vault) = self.tree.root() else { return };
        let path = vault.join(target);
        if self.change_note(root, &path, aquilum_core::history::Source::Me, |text| toggle_task_line(text, line))
            && let Some(embeds) = &self.embeds
        {
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

fn toggle_task_line(text: &crop::Rope, line: usize) -> super::note_page::text::Change {
    let index = line.checked_sub(1).filter(|index| *index < text.line_len())?;
    let start = text.byte_of_line(index);
    let current = text.line(index).to_string();
    let next = aquilum_core::markdown::list::toggle_task(&current)?;
    Some((start..start + current.len(), next))
}

#[cfg(test)]
mod tests {
    #[test]
    fn toggles_task_lines() {
        let toggled = |text: &str, line| {
            let mut rope = crop::Rope::from(text);
            let (range, insert) = super::toggle_task_line(&rope, line)?;
            rope.replace(range, &insert);
            Some(rope.to_string())
        };
        assert_eq!(toggled("a\n- [ ] дело", 2).as_deref(), Some("a\n- [x] дело"));
        assert_eq!(toggled("\t1. [x] b", 1).as_deref(), Some("\t1. [ ] b"));
        assert_eq!(toggled("просто текст", 1), None);
    }
}
