use super::*;

impl App {
    pub(super) fn open_file_menu(
        &mut self,
        root: &mut RenderRoot,
        targets: Vec<PathBuf>,
        clicked: PathBuf,
        folder: bool,
        at: Point,
        window: Size,
        tab: Option<Uuid>,
    ) {
        let label = if tab.is_some() {
            t("tabs.actions")
        } else if folder {
            t("fileTree.folderActions")
        } else {
            t("fileTree.fileActions")
        };
        let mut labels = vec![(t("common.rename"), crate::ui::icons::PENCIL), (t("common.duplicate"), crate::ui::icons::COPY), (t("common.delete"), crate::ui::icons::TRASH)];
        if tab.is_some() {
            labels.push((t("tabs.close"), crate::ui::icons::X));
        }
        let items: Vec<crate::ui::menu::Entry<'_>> = labels.iter().map(|(label, icon)| crate::ui::menu::Entry { label, icon: Some(*icon), disabled: false }).collect();
        let menu = Menu::entries(&label, &items);
        let layer = menu.id();
        root.add_layer(menu, Menu::context_origin(at, items.len(), window));
        root.focus_on(Some(layer));
        self.popup = Some(Popup::FileMenu { layer, targets, clicked, tab });
    }

    pub(super) fn open_sort_menu(&mut self, root: &mut RenderRoot, button: WidgetId) {
        let Some(trigger) = root.get_widget(button).map(|w| w.ctx().bounding_box()) else { return };
        let labels: Vec<String> = crate::ui::file_tree::Sort::ALL.iter().map(|s| t(s.label())).collect();
        let items: Vec<&str> = labels.iter().map(String::as_str).collect();
        let checked = crate::ui::file_tree::Sort::ALL.iter().position(|s| *s == self.tree.sort()).unwrap_or(0);
        let origin = Menu::list_origin(trigger, items.len(), self.window);
        let menu = Menu::list(&t("fileTree.sort"), &items, checked, trigger);
        let layer = menu.id();
        root.add_layer(menu, origin);
        root.edit_widget(button, |mut w| crate::ui::widgets::IconButton::set_pressed(&mut w.downcast(), true));
        root.focus_on(Some(layer));
        self.popup = Some(Popup::SortMenu { layer, button });
    }

    pub(super) fn on_popup_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        match self.popup.take() {
            Some(Popup::FileMenu { layer, targets, clicked, tab }) => {
                let Some(choice) = action.downcast_ref::<MenuChoice>() else {
                    self.popup = Some(Popup::FileMenu { layer, targets, clicked, tab });
                    return Outcome::default();
                };
                if let MenuChoice::Item(_) = choice {
                    root.remove_layer(layer);
                }
                match choice {
                    MenuChoice::Item(MENU_DUPLICATE) => file_ops::duplicate_all(&self.workspace.core, targets),
                    MenuChoice::Item(MENU_DELETE) => self.confirm_delete(root, targets),
                    MenuChoice::Item(MENU_CLOSE) if let Some(tab) = tab => {
                        self.tabs.close(tab);
                        self.show_active(root);
                        return Outcome { title: true, ..Outcome::default() };
                    }
                    MenuChoice::Item(MENU_RENAME) if tab.is_some() => self.start_tab_rename(root, &clicked),
                    MenuChoice::Item(MENU_RENAME) => self.tree.start_rename(root, clicked),
                    MenuChoice::Item(_) | MenuChoice::Dismissed => {}
                }
            }
            Some(Popup::Delete { dialog, targets }) => match dialog.answer(id, action) {
                Some(answer) => {
                    crate::ui::dialog::close(root, dialog.layer);
                    if answer == DialogAnswer::Confirm
                        && let Some(workspace) = self.tree.root().map(Path::to_path_buf)
                    {
                        file_ops::trash_all(&self.workspace.core, workspace, targets);
                        self.tree.clear_selection(root);
                    }
                }
                None => self.popup = Some(Popup::Delete { dialog, targets }),
            },
            Some(Popup::VersionMenu { layer, index, commands }) => self.on_version_menu(root, layer, index, commands, action),
            Some(Popup::TableMenu { layer, commands }) => self.on_table_menu(root, layer, commands, action),
            Some(Popup::EditorMenu { layer, commands }) => self.on_editor_menu(root, layer, commands, action),
            Some(Popup::NoteActionsMenu { layer }) => return self.on_note_actions_menu(root, layer, action),
            Some(Popup::SortMenu { layer, button }) => {
                if id == button && action.downcast_ref::<crate::ui::widgets::Pressed>().is_some() {
                    root.remove_layer(layer);
                    root.edit_widget(button, |mut w| crate::ui::widgets::IconButton::set_pressed(&mut w.downcast(), false));
                    return Outcome::default();
                }
                let Some(choice) = action.downcast_ref::<MenuChoice>() else {
                    self.popup = Some(Popup::SortMenu { layer, button });
                    return Outcome::default();
                };
                if let MenuChoice::Item(_) = choice {
                    root.remove_layer(layer);
                }
                root.edit_widget(button, |mut w| crate::ui::widgets::IconButton::set_pressed(&mut w.downcast(), false));
                if let MenuChoice::Item(index) = choice
                    && let Some(sort) = crate::ui::file_tree::Sort::ALL.get(*index)
                {
                    self.tree.set_sort(root, *sort);
                }
            }
            Some(Popup::Viewer { layer }) => {
                if action.downcast_ref::<crate::ui::viewer::ViewerClosed>().is_some() {
                    root.remove_layer(layer);
                } else {
                    self.popup = Some(Popup::Viewer { layer });
                }
            }
            Some(popup @ (Popup::Settings { .. } | Popup::Search(_) | Popup::Templates(_) | Popup::Workspaces(_) | Popup::Reader(_))) => self.popup = Some(popup),
            None => {}
        }
        Outcome::default()
    }

    /// `DeleteNotesDialog`: заголовок и текст зависят от числа целей и от того, папка ли это.
    pub(super) fn confirm_delete(&mut self, root: &mut RenderRoot, targets: Vec<PathBuf>) {
        let many = targets.len() > 1;
        let folder = !many && targets.first().is_some_and(|p| p.is_dir());
        let name = targets.first().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let count = targets.len().to_string();
        let (title, description) = if many {
            (t("confirm.notesTitle"), t_with("confirm.manyDescription", &[("count", &count)]))
        } else if folder {
            (t("confirm.folderTitle"), t_with("confirm.folderDescription", &[("name", &name)]))
        } else {
            (t("confirm.noteTitle"), t_with("confirm.noteDescription", &[("name", &name)]))
        };
        let confirm = if many { t_with("confirm.deleteMany", &[("count", &count)]) } else { t("common.delete") };
        let (layer, dialog) = ConfirmDialog::new(&title, &description, &t("common.cancel"), &confirm);
        root.add_layer(layer, Point::ORIGIN);
        root.focus_on(Some(dialog.layer));
        self.popup = Some(Popup::Delete { dialog, targets });
    }
}
