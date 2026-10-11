use super::*;

impl App {
    pub(super) fn refresh_home(&mut self) {
        self.home_generation += 1;
        let generation = self.home_generation;
        let Some(root) = self.workspace.root.clone() else {
            let _ = self.home_tx.send((generation, None));
            return;
        };
        let workspace = root.to_string_lossy().into_owned();
        let page = self
            .workspace
            .core
            .ui_state
            .list_workspaces()
            .ok()
            .and_then(|known| known.into_iter().find(|w| w.path == workspace))
            .map(|w| w.home_page.trim().to_owned())
            .unwrap_or_default();
        let sender = self.home_tx.clone();
        let wake = Arc::clone(&self.workspace.wake);
        if page.is_empty() {
            let _ = sender.send((generation, None));
            wake();
            return;
        }
        let core = Arc::clone(&self.workspace.core);
        std::thread::spawn(move || {
            let mut result = core.search.resolve_wiki_links(&workspace, &workspace, std::slice::from_ref(&page));
            for _ in 0..600 {
                if !matches!(result, Err(aquilum_core::search::error::SearchError::Unavailable { .. })) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
                result = core.search.resolve_wiki_links(&workspace, &workspace, std::slice::from_ref(&page));
            }
            let path = result
                .map_err(|error| report!("домашняя страница не найдена: {error:?}"))
                .ok()
                .and_then(|r| r.paths.into_iter().next().flatten())
                .map(PathBuf::from);
            if sender.send((generation, path)).is_ok() {
                wake();
            }
        });
    }

    pub(super) fn on_home_resolved(&mut self, root: &mut RenderRoot) {
        let Some((_, home)) = self.home_rx.try_iter().filter(|(g, _)| *g == self.home_generation).last() else { return };
        if home == self.home {
            return;
        }
        self.home = home;
        let (widget, id) = ui::home_widget(self.home.is_some() && !self.interface.focus_mode());
        if let Some(chrome) = &mut self.chrome {
            chrome.home_slot.edit(root, |mut slot| Slot::set_child(&mut slot, widget));
            chrome.home_button = id;
        }
    }
}
