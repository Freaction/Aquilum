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
        self.tree.poll(root);
        self.on_panel_results(root);
        self.on_search_results(root);
        self.on_home_resolved(root);
        self.on_link_titles(root);
        self.on_graph_results(root);
        title |= self.on_opened(root);
        self.on_update(root);
        self.on_retention_done(root);
        self.undo_failed_renames(root);
        self.poll_find(root);
        let mut links_changed = false;
        let mut history_changed = false;
        let mut changed = Vec::new();
        let mut edited: Vec<PathBuf> = Vec::new();
        let mut rescan = false;
        let mut navigations = Vec::new();
        for event in self.workspace.drain_events() {
            match event {
                CoreEvent::Navigate(navigation) => navigations.push(navigation),
                CoreEvent::SyncChanged => self.refresh_section(root, crate::ui::settings::schema::Section::Sync),
                CoreEvent::Settle(done) => {
                    if let Some(note) = &mut self.note {
                        note.flush(&self.workspace.core, &self.text);
                    }
                    let _ = done.send(());
                }
                CoreEvent::WorkspaceRescanned => rescan = true,
                CoreEvent::WorkspaceChanged(paths) => changed.extend(paths.into_iter().map(PathBuf::from)),
                CoreEvent::ContentChanged(paths) => edited.extend(paths.into_iter().map(PathBuf::from)),
                CoreEvent::LinksChanged(_) => links_changed = true,
                CoreEvent::NoteHistoryChanged(changed) => {
                    history_changed |= self.tabs.active().path.as_deref() == Some(Path::new(&changed.path));
                }
                CoreEvent::NotesRelocated(relocation) => {
                    for moved in &relocation.moves {
                        self.follow_move(root, Path::new(&moved.from), Path::new(&moved.to));
                        changed.push(PathBuf::from(&moved.from));
                        changed.push(PathBuf::from(&moved.to));
                    }
                    let removed: Vec<PathBuf> = relocation.removed.iter().map(PathBuf::from).collect();
                    title |= self.close_missing_tabs(root, &removed);
                    changed.extend(removed);
                }
            }
        }
        if rescan || !changed.is_empty() || !edited.is_empty() {
            let touched: Vec<PathBuf> = changed.iter().chain(&edited).cloned().collect();
            if let Some(embeds) = &self.embeds {
                embeds.forget_paths(&touched);
            }
            if let Some(note) = self.note.as_mut().filter(|n| rescan || touched.iter().any(|p| n.is(p)))
                && let Some(text) = note.reload()
            {
                self.set_text(root, text);
            }
        }
        if rescan {
            self.tree.reload(root);
            let vault: Vec<PathBuf> = self.tree.root().map(Path::to_path_buf).into_iter().collect();
            title |= self.close_missing_tabs(root, &vault);
        } else if !changed.is_empty() {
            self.tree.paths_changed(root, &changed);
            title |= self.close_missing_tabs(root, &changed);
        }
        for navigation in navigations {
            title |= match navigation {
                Navigation::Note { path, disposition } => self.open_in(root, PathBuf::from(path), matches!(disposition, Disposition::NewTab)),
                Navigation::Workspace { path } => self.switch_workspace(root, PathBuf::from(path)).title,
            };
        }
        if links_changed {
            if let Some(embeds) = &self.embeds {
                embeds.forget_queries();
            }
            self.refresh_home();
            self.graph_links_changed();
            self.search_index_changed();
            if self.panel_mode() != crate::links::Mode::History {
                self.request_panel(root);
            }
        }
        if history_changed {
            self.refresh_history(root);
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
            self.refresh_section(root, crate::ui::settings::schema::Section::System);
        }
        if let Some(payload) = self.updater.take_ready() {
            let _ = root.redraw();
            let mut closed = false;
            if let Err(error) = crate::updater::apply(&payload, || {
                self.shutdown();
                closed = true;
            }) {
                report!("обновление не установлено: {error}");
                if closed {
                    std::process::exit(1);
                }
                self.updater.fail(error);
            }
        }
    }
}
