use super::*;

impl App {
    pub(super) fn wipe(&mut self, root: &mut RenderRoot, wipe: Wipe) {
        let Some(workspace) = self.tree.root() else { return };
        let wiped = match wipe {
            Wipe::History => aquilum_core::history::store::clear(workspace).map_err(|error| error.to_string()),
            Wipe::Trash => empty_trash_impl(workspace).map(drop).map_err(|error| format!("{error:?}")),
        };
        if let Err(error) = wiped {
            report!("не очищено: {error}");
        }
        self.refresh_history(root);
    }

    pub(in crate::app) fn cleanup_retention(&self) {
        let Some(workspace) = self.tree.root().map(Path::to_path_buf) else { return };
        let core = std::sync::Arc::clone(&self.workspace.core);
        let (done, wake) = (std::sync::Arc::clone(&self.retention_done), std::sync::Arc::clone(&self.workspace.wake));
        std::thread::spawn(move || {
            let config = core.settings.get_config();
            if let Err(error) = cleanup_trash_impl(&workspace, config.trash.retention_days) {
                report!("корзина не очищена: {error:?}");
            }
            aquilum_core::history::store::adopt_quantum_history(&workspace);
            aquilum_core::history::store::adopt_quantum_history(&aquilum_core::files::trash::trash_path(&workspace));
            aquilum_core::history::store::remove_expired(&workspace, config.history.retention_days);
            if let Err(error) = core.ui_state.cleanup(config.trash.retention_days, crate::dates::now_ms()) {
                report!("состояние документов не очищено: {error:?}");
            }
            core.prune_caches();
            done.store(true, std::sync::atomic::Ordering::Release);
            wake();
        });
    }

    pub(in crate::app) fn on_retention_done(&mut self, root: &mut RenderRoot) {
        if self.retention_done.swap(false, std::sync::atomic::Ordering::Acquire)
            && matches!(&self.popup, Some(Popup::Settings { dialog, .. }) if matches!(dialog.section, Section::Trash | Section::History))
        {
            self.refresh_settings(root);
        }
    }
}
