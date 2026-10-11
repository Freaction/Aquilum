use super::*;

impl App {
    pub fn on_core_events(&mut self, root: &mut RenderRoot) -> bool {
        let mut title = false;
        while let Ok(picked) = self.picked_rx.try_recv() {
            title |= self.on_picked(root, picked);
        }
        self.refresh_covers(root);
        let requests = self.embeds.as_ref().map(embeds::Embeds::take_requests).unwrap_or_default();
        for request in requests {
            title |= self.on_editor_request(root, request);
        }
        if self.embeds.as_ref().is_some_and(embeds::Embeds::take_changed)
            && let Some(ids) = &self.note_ids
        {
            ids.body.edit(root, |mut body| crate::ui::editor::TextEditor::refresh_embeds(&mut body));
        }
        self.on_panel_results(root);
        self.on_search_results(root);
        self.on_home_resolved(root);
        self.on_link_titles(root);
        self.on_graph_results(root);
        self.on_update(root);
        self.on_retention_done(root);
        let mut links_changed = false;
        let mut history_changed = false;
        let mut changed = Vec::new();
        let mut navigations = Vec::new();
        for event in self.workspace.drain_events() {
            match event {
                CoreEvent::Navigate(navigation) => navigations.push(navigation),
                CoreEvent::WorkspaceChanged(paths) => changed.extend(paths.into_iter().map(PathBuf::from)),
                CoreEvent::LinksChanged(_) => {
                    links_changed = true;
                    if let Some(embeds) = &self.embeds {
                        embeds.forget_queries();
                    }
                    self.refresh_home();
                    self.graph_links_changed();
                }
                CoreEvent::NoteHistoryChanged(changed) => {
                    history_changed |= self.tabs.active().path.as_deref() == Some(Path::new(&changed.path));
                }
                CoreEvent::NotesRelocated(relocation) => {
                    for moved in &relocation.moves {
                        // Открытая заметка переехала — заголовок и подсветка следуют за ней.
                        self.interface.move_metadata(Path::new(&moved.from), Path::new(&moved.to));
                        if let Some(note) = self.note.as_mut().filter(|n| n.is(Path::new(&moved.from))) {
                            note.path = PathBuf::from(&moved.to);
                            self.workspace.core.active_note.set(Some(note.path.clone()));
                            self.tree.set_active(root, Some(note.path.clone()));
                            self.refresh_note_names(root);
                        }
                        self.relocate_tabs(root, Path::new(&moved.from), Path::new(&moved.to));
                        changed.push(PathBuf::from(&moved.from));
                        changed.push(PathBuf::from(&moved.to));
                    }
                    let removed: Vec<PathBuf> = relocation.removed.iter().map(PathBuf::from).collect();
                    self.close_missing_tabs(root, &removed);
                    changed.extend(removed);
                }
                _ => {}
            }
        }
        if !changed.is_empty() {
            if let Some(embeds) = &self.embeds {
                embeds.forget_paths(&changed);
            }
            // Открытую заметку изменили снаружи (агент, другая программа) — показать новый текст.
            if let Some(note) = self.note.as_mut().filter(|n| changed.iter().any(|p| n.is(p)))
                && let Some(text) = note.reload()
            {
                self.set_editor_text(root, text);
            }
            self.tree.paths_changed(root, &changed);
        }
        for navigation in navigations {
            title |= match navigation {
                Navigation::Note { path, disposition } => self.open_in(root, PathBuf::from(path), matches!(disposition, Disposition::NewTab)),
                Navigation::Workspace { path } => self.switch_workspace(root, PathBuf::from(path)).title,
            };
        }
        if links_changed && self.panel_mode() != crate::links::Mode::History {
            self.request_panel(root);
        }
        if history_changed && self.panel_mode() == crate::links::Mode::History && self.panel_visible() {
            self.load_history(root);
        }
        title
    }

    pub(super) fn on_update(&mut self, root: &mut RenderRoot) {
        use crate::updater::Status;
        let status = self.updater.status();
        if status != self.update_shown {
            self.update_shown = status.clone();
            let splash = match &status {
                Status::Downloading { done, total, .. } => Some((false, total.filter(|t| *t > 0).map(|t| *done as f64 * 100.0 / t as f64))),
                Status::Ready(_) | Status::Installing => Some((true, None)),
                _ => None,
            };
            match (splash, self.update_layer) {
                (Some((installing, percent)), Some(layer)) => {
                    root.edit_widget(layer, |mut w| crate::ui::update::UpdateSplash::set(&mut w.downcast(), installing, percent));
                }
                (Some((installing, percent)), None) => {
                    let widget = crate::ui::update::UpdateSplash::new(installing, percent);
                    self.update_layer = Some(widget.id());
                    root.add_layer(widget, Point::ORIGIN);
                }
                (None, Some(layer)) => {
                    root.remove_layer(layer);
                    self.update_layer = None;
                }
                (None, None) => {}
            }
            if let Some(Popup::Settings { dialog, .. }) = &self.popup
                && dialog.section == crate::ui::settings::schema::Section::System
            {
                self.refresh_settings(root);
            }
        }
        if let Some(payload) = self.updater.take_ready() {
            let _ = root.redraw();
            if let Err(error) = crate::updater::apply(&payload, || self.shutdown()) {
                report!("обновление не установлено: {error}");
            }
        }
    }
}
