use super::*;

impl App {
    pub(super) fn press(&mut self, root: &mut RenderRoot, dialog: &mut SettingsDialog, action: Action) -> Effect {
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
            Action::Wipe(wipe) => {
                let (layer, confirm) = ConfirmDialog::new(&t(&wipe.key("clearTitle")), &t(&wipe.key("clearDescription")), &t("common.cancel"), &t(&wipe.key("clearConfirm")));
                root.add_layer(layer, Point::ORIGIN);
                root.focus_on(Some(confirm.layer));
                dialog.confirm = Some((confirm, wipe));
                Effect::None
            }
            Action::SyncConnect => {
                match core.sync.connect(core, system::login_links()) {
                    Ok(url) => system::open_url(&url),
                    Err(error) => report!("вход в Dropbox не начат: {error}"),
                }
                Effect::Content
            }
            Action::SyncDisconnect => {
                if let Err(error) = core.sync.disconnect() {
                    report!("Dropbox не отключён до конца: {error}");
                }
                Effect::Content
            }
            Action::SyncAllowDeletions => {
                core.sync.allow_deletions();
                Effect::Content
            }
            Action::Trash(path) => {
                let Some(workspace) = workspace else { return Effect::None };
                if let Err(error) = gate::trash(core, &workspace, &workspace.join(path)) {
                    report!("файл не убран в корзину: {error:?}");
                }
                Effect::Content
            }
            Action::SyncSend(paths) => {
                core.sync.allow(paths);
                Effect::Content
            }
            Action::Reveal(path) => {
                if let Some(workspace) = workspace {
                    system::reveal(&workspace.join(path));
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
                if crate::clipboard::copy(code) {
                    dialog.copied = Some(index);
                }
                Effect::Content
            }
            Action::OpenUrl(url) => {
                system::open_url(&url);
                Effect::None
            }
        }
    }

    pub(in crate::app) fn apply(&mut self, setter: Setter, value: Value) -> Effect {
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
            Setter::VaultSync(set) => {
                let Some(root) = self.tree.root().map(Path::to_path_buf) else { return Effect::None };
                self.update_sync(&root, |sync| set(sync, &value));
                Effect::Content
            }
            Setter::Config(set) => {
                let core = &self.workspace.core;
                let before = core.settings.get_config();
                let mut after = before.clone();
                set(&mut after, &value);
                let language_changed = before.ui.language != after.ui.language;
                let colors_changed = before.ui.theme != after.ui.theme || before.ui.primary_color != after.ui.primary_color;
                let fonts_changed = before.ui.font != after.ui.font
                    || before.editor.font != after.editor.font
                    || before.editor.line_height != after.editor.line_height
                    || before.editor.max_width_ch != after.editor.max_width_ch;
                let mcp_changed = before.mcp != after.mcp;
                let retention = before.trash.retention_days != after.trash.retention_days || before.history.retention_days != after.history.retention_days;
                let analysis_changed = before.analysis != after.analysis || before.search != after.search;
                let methods_toggled = crate::links::Method::enabled(&before).is_empty() != crate::links::Method::enabled(&after).is_empty();
                let smart = crate::ui::editor::Smart::from_config(&after);
                let smart_changed = smart != crate::ui::editor::Smart::from_config(&before);
                crate::ui::motion::set(after.ui.animations);
                let rows_changed = before.ui.animations != after.ui.animations
                    || before.analysis.enable_bm25f != after.analysis.enable_bm25f
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
                } else if smart_changed {
                    Effect::Smart(smart)
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
}
