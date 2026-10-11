use super::*;

impl App {
    pub(super) fn press(&mut self, dialog: &mut SettingsDialog, action: Action) -> Effect {
        let core = &self.workspace.core;
        let workspace = self.tree.root().map(Path::to_path_buf);
        match action {
            Action::OpenTrash => {
                if let Some(workspace) = workspace {
                    match ensure_trash_impl(&workspace) {
                        Ok(path) => system::open(&path),
                        Err(error) => report!("корзина не открыта: {error:?}"),
                    }
                }
                Effect::None
            }
            Action::Restore(id) => {
                let Some(workspace) = workspace else { return Effect::None };
                if let Err(error) = gate::restore_deletion(core, &workspace, &id) {
                    report!("заметка не восстановлена: {error:?}");
                }
                Effect::Content
            }
            Action::TrashPage(page) => {
                dialog.trash_page = page;
                Effect::Content
            }
            Action::CreateTemplates => {
                let Some(workspace) = workspace else { return Effect::None };
                crate::app::templates::write_starters(core, &schema::templates_path(&workspace, &core.settings.get_config().templates.folder));
                Effect::Content
            }
            Action::CheckUpdates => {
                self.updater.check();
                Effect::Content
            }
            Action::InstallUpdate => {
                self.updater.install();
                Effect::Content
            }
            Action::NewToken => self.apply(Setter::Config(|c, _| c.mcp.token = schema::new_token()), Value::Bool(true)),
            Action::Copy(index, code) => {
                match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(code)) {
                    Ok(()) => dialog.copied = Some(index),
                    Err(error) => report!("не скопировано: {error}"),
                }
                Effect::Content
            }
            Action::OpenUrl(url) => {
                system::open_url(url);
                Effect::None
            }
        }
    }

    pub(super) fn apply(&mut self, setter: Setter, value: Value) -> Effect {
        match setter {
            Setter::Scale => {
                let Value::Number(percent) = value else { return Effect::None };
                if self.interface.set_scale(percent / 100.0) { Effect::Rescale } else { Effect::None }
            }
            Setter::Autostart => {
                let Value::Bool(on) = value else { return Effect::None };
                if !crate::autostart::set(on) {
                    report!("автозапуск не изменён");
                }
                Effect::Content
            }
            Setter::HomePage => {
                let (Value::Text(page), Some(root)) = (value, self.tree.root()) else { return Effect::None };
                if let Err(error) = self.workspace.core.ui_state.set_home_page(&root.to_string_lossy(), page.trim()) {
                    report!("домашняя страница не сохранена: {error:?}");
                }
                self.refresh_home();
                Effect::None
            }
            Setter::Config(set) => {
                let core = &self.workspace.core;
                let before = core.settings.get_config();
                let mut after = before.clone();
                set(&mut after, &value);
                let language_changed = before.ui.language != after.ui.language;
                let colors_changed = before.ui.theme != after.ui.theme || before.ui.primary_color != after.ui.primary_color;
                let fonts_changed = before.ui.font.font_family != after.ui.font.font_family
                    || before.ui.font.font_weight != after.ui.font.font_weight
                    || before.ui.font.font_size_base != after.ui.font.font_size_base
                    || before.editor.font.font_family != after.editor.font.font_family
                    || before.editor.font.font_weight != after.editor.font.font_weight
                    || before.editor.font.font_size_base != after.editor.font.font_size_base
                    || before.editor.line_height != after.editor.line_height
                    || before.editor.max_width_ch != after.editor.max_width_ch;
                let mcp_changed = serde_json::to_value(&before.mcp).ok() != serde_json::to_value(&after.mcp).ok();
                let retention = before.trash.retention_days != after.trash.retention_days || before.history.retention_days != after.history.retention_days;
                let json = |c: &aquilum_core::settings::models::AppConfig| serde_json::to_value((&c.analysis, &c.search)).ok();
                let analysis_changed = json(&before) != json(&after);
                let methods_toggled = crate::links::Method::enabled(&before).is_empty() != crate::links::Method::enabled(&after).is_empty();
                let rows_changed = before.analysis.enable_bm25f != after.analysis.enable_bm25f
                    || before.updates.auto != after.updates.auto
                    || before.templates.folder != after.templates.folder
                    || mcp_changed
                    || retention;
                if let Err(error) = core.settings.update_config(after) {
                    report!("настройки не сохранены: {error}");
                    return Effect::None;
                }
                if mcp_changed {
                    core.apply_mcp_settings();
                }
                if retention {
                    self.cleanup_retention();
                }
                if language_changed || methods_toggled {
                    Effect::Everything
                } else if fonts_changed || colors_changed {
                    Effect::Restyle(fonts_changed)
                } else if rows_changed {
                    Effect::Content
                } else if analysis_changed {
                    Effect::Panel
                } else {
                    Effect::None
                }
            }
        }
    }

    pub(super) fn refresh_analysis(&mut self, root: &mut RenderRoot) {
        if self.panel_mode() == crate::links::Mode::Analysis {
            self.request_panel(root);
        }
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
