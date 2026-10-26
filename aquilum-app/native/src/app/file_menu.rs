use super::*;
use crate::ui::menu::{Submenu, Swatch};
use crate::ui::palette;

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
        let colored = folder && tab.is_none();
        if colored {
            labels.push((t("palette.color"), crate::ui::icons::PALETTE));
        }
        let items: Vec<crate::ui::menu::Entry<'_>> = labels.iter().map(|(label, icon)| crate::ui::menu::Entry { label, icon: Some(*icon), disabled: false }).collect();
        let names: Vec<String> = std::iter::once(t("palette.none")).chain(palette::NAMES.iter().map(|name| palette::label(name))).collect();
        let theme = crate::ui::theme::current();
        let swatches: Vec<Swatch<'_>> = names.iter().zip(std::iter::once(None).chain(palette::NAMES.iter().map(|name| palette::color(name, &theme)))).map(|(label, color)| Swatch { label, color }).collect();
        let current = self.tree.color(&clicked).and_then(|name| palette::NAMES.iter().position(|known| *known == name)).map_or(0, |at| at + 1);
        let menu = if colored { Menu::with_submenu(&label, &items, &Submenu { at: MENU_COLOR, items: &swatches, checked: current }) } else { Menu::entries(&label, &items) };
        self.show_layer(root, menu, Menu::context_origin(at, items.len(), window), |layer| Popup::FileMenu { layer, targets, clicked, tab });
    }

    pub(super) fn open_sort_menu(&mut self, root: &mut RenderRoot, button: WidgetId) {
        let Some(trigger) = root.get_widget(button).map(|w| w.ctx().bounding_box()) else { return };
        let labels: Vec<String> = crate::ui::file_tree::Sort::ALL.iter().map(|s| t(s.label())).collect();
        let items: Vec<&str> = labels.iter().map(String::as_str).collect();
        let checked = crate::ui::file_tree::Sort::ALL.iter().position(|s| *s == self.tree.sort()).unwrap_or(0);
        let origin = Menu::list_origin(trigger, items.len(), self.window);
        let menu = Menu::list(&t("fileTree.sort"), &items, checked, trigger);
        root.edit_widget(button, |mut w| crate::ui::widgets::IconButton::set_pressed(&mut w.downcast(), true));
        self.show_layer(root, menu, origin, |layer| Popup::SortMenu { layer, button });
    }

    pub(super) fn open_tabs_menu(&mut self, root: &mut RenderRoot, button: WidgetId) {
        let Some(trigger) = root.get_widget(button).map(|w| w.ctx().bounding_box()) else { return };
        let titles: Vec<String> = self.tabs.tabs().iter().map(|tab| if tab.graph { t("tabs.graph") } else { tab.title().unwrap_or_else(|| t("tabs.newTab")) }).collect();
        let items: Vec<&str> = titles.iter().map(String::as_str).collect();
        let checked = self.tabs.tabs().iter().position(|tab| tab.id == self.tabs.active_id()).unwrap_or(0);
        let width = crate::ui::tokens::size::TAB_LIST_WIDTH;
        let origin = Menu::dropdown_origin(trigger, self.window, width);
        let menu = Menu::dropdown(&t("tabs.list"), &items, checked, trigger, self.window, width);
        root.edit_widget(button, |mut w| crate::ui::widgets::IconButton::set_pressed(&mut w.downcast(), true));
        self.show_layer(root, menu, origin, |layer| Popup::TabsMenu { layer, button });
    }

    fn dropdown_answer(&mut self, root: &mut RenderRoot, layer: WidgetId, button: WidgetId, id: WidgetId, action: &ErasedAction) -> Option<Option<usize>> {
        let release = |root: &mut RenderRoot| root.edit_widget(button, |mut w| crate::ui::widgets::IconButton::set_pressed(&mut w.downcast(), false));
        if id == button && action.downcast_ref::<crate::ui::widgets::Pressed>().is_some() {
            root.remove_layer(layer);
            release(root);
            return Some(None);
        }
        let choice = action.downcast_ref::<MenuChoice>()?;
        if let MenuChoice::Item(_) = choice {
            root.remove_layer(layer);
        }
        release(root);
        Some(match choice {
            MenuChoice::Item(index) => Some(*index),
            MenuChoice::Dismissed | MenuChoice::Hover(_) | MenuChoice::Sub(_) => None,
        })
    }

    fn pick_tab(&mut self, root: &mut RenderRoot, index: usize) -> Outcome {
        let Some(id) = self.tabs.tabs().get(index).map(|tab| tab.id) else { return Outcome::default() };
        self.on_titlebar(root, &crate::ui::titlebar::TitlebarAction::Select(id), self.window)
    }

    pub(super) fn on_popup_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        match self.popup.take() {
            Some(Popup::FileMenu { layer, targets, clicked, tab }) => {
                let Some(choice) = action.downcast_ref::<MenuChoice>() else {
                    self.popup = Some(Popup::FileMenu { layer, targets, clicked, tab });
                    return Outcome::default();
                };
                if let MenuChoice::Hover(index) = choice {
                    root.edit_widget(layer, |mut menu| Menu::show_sub(&mut menu.downcast(), *index == MENU_COLOR));
                    self.popup = Some(Popup::FileMenu { layer, targets, clicked, tab });
                    return Outcome::default();
                }
                if let MenuChoice::Item(_) | MenuChoice::Sub(_) = choice {
                    root.remove_layer(layer);
                }
                match choice {
                    MenuChoice::Sub(index) => {
                        let folders: Vec<PathBuf> = targets.into_iter().filter(|path| path.is_dir()).collect();
                        self.tree.set_color(root, &folders, index.checked_sub(1).and_then(|at| palette::NAMES.get(at).copied()));
                    }
                    MenuChoice::Item(MENU_DUPLICATE) => file_ops::duplicate_all(&self.workspace.core, targets),
                    MenuChoice::Item(MENU_DELETE) => self.confirm_delete(root, targets),
                    MenuChoice::Item(MENU_CLOSE) if let Some(tab) = tab => {
                        self.tabs.close(tab);
                        self.show_active(root);
                        return Outcome { title: true, ..Outcome::default() };
                    }
                    MenuChoice::Item(MENU_RENAME) if tab.is_some() => self.start_tab_rename(root, &clicked),
                    MenuChoice::Item(MENU_RENAME) => self.tree.start_rename(root, clicked),
                    MenuChoice::Item(_) | MenuChoice::Dismissed | MenuChoice::Hover(_) => {}
                }
            }
            Some(Popup::Delete { dialog, targets }) => match dialog.answer(id, action) {
                Some(answer) => {
                    crate::ui::dialog::close(root, dialog.layer);
                    if answer == DialogAnswer::Confirm
                        && let Some(workspace) = self.tree.root().map(Path::to_path_buf)
                    {
                        self.commit_note();
                        file_ops::trash_all(&self.workspace.core, workspace, targets);
                        self.tree.clear_selection(root);
                    }
                }
                None => self.popup = Some(Popup::Delete { dialog, targets }),
            },
            Some(Popup::ResetGraph(dialog)) => match dialog.answer(id, action) {
                Some(DialogAnswer::Confirm) => { crate::ui::dialog::close(root, dialog.layer); self.reset_graph_settings(root); }
                Some(_) => crate::ui::dialog::close(root, dialog.layer),
                None => self.popup = Some(Popup::ResetGraph(dialog)),
            },
            Some(Popup::ManualUpdate(dialog, url)) => match dialog.answer(id, action) {
                Some(DialogAnswer::Confirm) => { crate::ui::dialog::close(root, dialog.layer); crate::system::open_url(&url); }
                Some(_) => crate::ui::dialog::close(root, dialog.layer),
                None => self.popup = Some(Popup::ManualUpdate(dialog, url)),
            },
            Some(Popup::VersionMenu { layer, index, commands }) => self.on_version_menu(root, layer, index, commands, action),
            Some(Popup::TableMenu { layer, commands }) => self.on_table_menu(root, layer, commands, action),
            Some(Popup::EditorMenu { layer, commands }) => self.on_editor_menu(root, layer, commands, action),
            Some(Popup::NoteActionsMenu { layer }) => return self.on_note_actions_menu(root, layer, action),
            Some(Popup::SortMenu { layer, button }) => match self.dropdown_answer(root, layer, button, id, action) {
                None => self.popup = Some(Popup::SortMenu { layer, button }),
                Some(chosen) => {
                    if let Some(sort) = chosen.and_then(|index| crate::ui::file_tree::Sort::ALL.get(index)) {
                        self.tree.set_sort(root, *sort);
                    }
                }
            },
            Some(Popup::TabsMenu { layer, button }) => match self.dropdown_answer(root, layer, button, id, action) {
                None => self.popup = Some(Popup::TabsMenu { layer, button }),
                Some(chosen) => return chosen.map_or_else(Outcome::default, |index| self.pick_tab(root, index)),
            },
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
