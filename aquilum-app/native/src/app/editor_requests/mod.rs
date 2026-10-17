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
            Request::TableMenu { at, commands } => self.open_table_menu(root, at, commands),
            Request::ContextMenu { at, has_selection } => self.open_editor_context_menu(root, at, has_selection),
            Request::Suggest(view) => self.show_suggest(root, view),
            Request::Read { file, page, title } => self.open_reader(root, super::reader::Opening { file, page, title, initial: None, peek: false }),
            Request::ToggleTask { target, line } => self.toggle_task_in_file(root, &target, line),
            Request::View(image, path) => {
                let sharp = path.and_then(|p| std::fs::read(p).ok()).and_then(|bytes| crate::images::decode(&bytes));
                self.open_viewer(root, sharp.unwrap_or(image));
            }
            Request::LinkTitle { editor, placeholder, url } => {
                let (sender, wake) = (self.titles_tx.clone(), Arc::clone(&self.workspace.wake));
                std::thread::spawn(move || {
                    if sender.send((editor, placeholder, aquilum_core::link_title::fetch::page_title(&url))).is_ok() {
                        wake();
                    }
                });
            }
            Request::Open { target, wiki, new_tab } => {
                let target = if wiki { crate::links::Target::Wiki(target) } else { crate::links::Target::Url(target) };
                return self.open_target(root, target, new_tab);
            }
        }
        false
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
        self.show_layer(root, crate::ui::viewer::Viewer::new(image), Point::ORIGIN, |layer| Popup::Viewer { layer });
    }

    pub(in crate::app) fn show_layer(&mut self, root: &mut RenderRoot, widget: masonry::core::NewWidget<impl masonry::core::Widget + ?Sized>, at: Point, popup: impl FnOnce(masonry::core::WidgetId) -> Popup) {
        let layer = widget.id();
        root.add_layer(widget, at);
        root.focus_on(Some(layer));
        self.popup = Some(popup(layer));
    }
}

fn toggle_task_line(text: &dyn aquilum_core::text::content::Content, line: usize) -> super::note_page::text::Change {
    let index = line.checked_sub(1)?;
    let mut pages = aquilum_core::search::pages::Pages::of(text);
    while let Some(page) = pages.next().ok()? {
        let mut at = page.start;
        for (number, current) in (page.start_line..).zip(page.text.split_inclusive('\n')) {
            let current = current.strip_suffix('\n').unwrap_or(current);
            if number == index {
                return Some((at..at + current.len(), aquilum_core::markdown::list::toggle_task(current)?));
            }
            at += current.len() + 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn toggles_task_lines() {
        let toggled = |text: &str, line| {
            let mut rope = crop::Rope::from(text);
            let (range, insert) = super::toggle_task_line(&crate::rope::Text(&rope), line)?;
            rope.replace(range, &insert);
            Some(rope.to_string())
        };
        assert_eq!(toggled("a\n- [ ] дело", 2).as_deref(), Some("a\n- [x] дело"));
        assert_eq!(toggled("\t1. [x] b", 1).as_deref(), Some("\t1. [ ] b"));
        assert_eq!(toggled("просто текст", 1), None);
    }
}
