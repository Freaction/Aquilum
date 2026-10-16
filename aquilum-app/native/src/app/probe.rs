use masonry::app::RenderRoot;

use super::App;

impl App {
    pub fn probe(&mut self, root: &mut RenderRoot, step: &crate::memory_probe::Step) {
        use crate::memory_probe::Step;
        match step {
            Step::OpenNote(path) => {
                self.open_in(root, path.clone(), true);
            }
            Step::OpenSettings => self.open_settings(root),
            Step::CloseSettings => {
                if let Some(super::Popup::Settings { dialog, menu }) = self.popup.take() {
                    if let Some(menu) = menu {
                        root.remove_layer(menu.layer);
                    }
                    crate::ui::dialog::close(root, dialog.layer);
                }
            }
            Step::OpenGraph => self.open_graph(root),
            Step::CloseTab => {
                self.close_active_tab(root);
            }
            Step::Same(path) => {
                self.open_in(root, path.clone(), false);
            }
            Step::Tab(index) => {
                if let Some(id) = self.tabs.tabs().get(*index).map(|t| t.id) {
                    self.tabs.select(id);
                    self.show_active(root);
                }
            }
            Step::Reader(path) => {
                let title = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                self.open_reader(root, super::reader::Opening { file: path.clone(), page: None, title, initial: None, peek: false });
            }
            Step::CloseReader => {
                if let Some(super::Popup::Reader(state)) = self.popup.take() {
                    root.remove_layer(state.layer);
                }
            }
            Step::Scroll(fraction) => {
                if let Some(ids) = &self.note_ids {
                    let scroll = ids.scroll;
                    scroll.edit(root, |mut area| crate::ui::scroll::ScrollArea::set_fraction(&mut area, *fraction));
                }
            }
            Step::Video => {
                if let Some(ids) = &self.note_ids {
                    ids.body.edit(root, |mut body| crate::ui::editor::TextEditor::probe_play(&mut body));
                }
            }
            Step::Evict(name) => {
                let embeds = name.contains("вставок");
                if let Some(ids) = &self.note_ids {
                    ids.body.edit(root, |mut body| crate::ui::editor::TextEditor::probe_evict(&mut body, embeds));
                }
            }
            Step::Wait | Step::Mark(_) | Step::Minimize(_) => {}
        }
    }

    pub fn probe_stats(&self, root: &RenderRoot) -> String {
        let editor = self.note_ids.as_ref().and_then(|ids| root.get_widget(ids.body.id())).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>().map(|e| e.inner().probe_stats()));
        format!("вкладок {} · текст заметки {:.1} МБ · {}\n", self.tabs.tabs().len(), crate::stats::mb(self.text.byte_len()),editor.unwrap_or_default())
    }
}
