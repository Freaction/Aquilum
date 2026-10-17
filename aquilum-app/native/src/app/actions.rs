use super::*;

impl App {
    /// Действие виджета; `window` — размер окна в логических пикселях (для меню у края).
    pub fn on_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: ErasedAction, window: Size) -> Outcome {
        if action.downcast_ref::<DialogAnswer>() == Some(&DialogAnswer::Closed) {
            root.remove_layer(id);
            return Outcome::default();
        }
        if self.on_suggest_pick(root, id, &action) || self.on_find_action(root, id, &action) || self.on_sash(root, id, &action) {
            return Outcome::default();
        }
        if matches!(self.popup, Some(Popup::Settings { .. })) {
            return self.on_settings_action(root, id, &action);
        }
        if matches!(self.popup, Some(Popup::Reader(_))) {
            return self.on_reader_action(root, id, &action);
        }
        if matches!(self.popup, Some(Popup::Search(_))) {
            return self.on_search_action(root, id, &action);
        }
        if matches!(self.popup, Some(Popup::Templates(_))) {
            return self.on_templates_action(root, id, &action);
        }
        if matches!(self.popup, Some(Popup::Workspaces(_))) {
            return self.on_workspaces_action(root, id, &action);
        }
        if self.popup.is_some() {
            return self.on_popup_action(root, id, &action);
        }
        if self.is_graph_widget(id) {
            let title = self.on_graph_action(root, id, &action);
            return Outcome { title, ..Outcome::default() };
        }
        if let Some(outcome) = self.on_panel_action(root, id, &action) {
            return outcome;
        }
        if let Some(outcome) = self.on_history_action(root, id, &action) {
            return outcome;
        }
        if let Some(outcome) = self.on_book_action(root, id, &action) {
            return outcome;
        }
        if let Some(outcome) = self.on_note_action(root, id, &action) {
            return outcome;
        }
        if let Some(titlebar) = action.downcast_ref::<TitlebarAction>() {
            return self.on_titlebar(root, titlebar, window);
        }
        if self.is_tab_rename_field(id) {
            match action.downcast_ref::<TextAction>() {
                Some(TextAction::Entered(text)) => self.finish_tab_rename(root, Some(text)),
                Some(TextAction::Cancelled) => self.cancel_tab_rename(root),
                _ => {}
            }
            return Outcome::default();
        }
        if self.renaming_tab.is_some() && action.downcast_ref::<TextAction>().is_none() {
            self.finish_tab_rename(root, None);
        }
        if let Some(chrome) = &self.chrome
            && action.downcast_ref::<Pressed>().is_some()
        {
            if id == chrome.new_tab {
                return self.new_tab(root);
            }
            if id == chrome.close_tab {
                return self.close_active_tab(root);
            }
            if id == chrome.create_note {
                let title = self.create_note_titled(root, None);
                return Outcome { title, ..Outcome::default() };
            }
            if id == chrome.open_file {
                self.open_search(root);
                return Outcome::default();
            }
            if id == chrome.workspaces {
                self.open_workspaces(root);
                return Outcome::default();
            }
            if id == chrome.open_vault {
                return self.pick_workspace(root);
            }
        }
        let title = self.on_main_action(root, id, &action, window);
        Outcome { title, ..Outcome::default() }
    }

    pub fn new_note(&mut self, root: &mut RenderRoot) -> bool {
        self.create_note_titled(root, None)
    }

    pub(super) fn create_note_titled(&mut self, root: &mut RenderRoot, title: Option<&str>) -> bool {
        let Some(workspace) = self.tree.root().map(Path::to_path_buf) else { return false };
        match file_ops::create_note(&self.workspace.core, &workspace, title) {
            Ok(path) => {
                self.tree.paths_changed(root, std::slice::from_ref(&path));
                self.open(root, path)
            }
            Err(error) => {
                report!("заметка не создана: {error}");
                false
            }
        }
    }

    pub(super) fn create_note_in(&mut self, root: &mut RenderRoot, folder: &Path) -> bool {
        match file_ops::create_note(&self.workspace.core, folder, None) {
            Ok(path) => {
                self.tree.paths_changed(root, std::slice::from_ref(&path));
                self.tree.reveal(root, &path);
                let opened = self.open(root, path.clone());
                self.tree.start_rename(root, path);
                opened
            }
            Err(error) => {
                report!("заметка не создана: {error}");
                false
            }
        }
    }

    /// Действие в основном окне; `true` — сменилась открытая заметка (и заголовок окна).
    pub(super) fn on_main_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction, window: Size) -> bool {
        match self.tree.on_action(root, id, action) {
            Some(TreeAction::Open { path, new_tab }) => return self.open_in(root, path, new_tab),
            Some(TreeAction::Rename { from, to }) => {
                self.rename_file(root, from, to);
                return false;
            }
            Some(TreeAction::CreateNote) => return self.create_note_titled(root, None),
            Some(TreeAction::NewNote { folder }) => return self.create_note_in(root, &folder),
            Some(TreeAction::NewFolder { folder }) => {
                match file_ops::create_folder(&folder) {
                    Ok(path) => {
                        self.tree.paths_changed(root, std::slice::from_ref(&path));
                        self.tree.reveal(root, &path);
                        self.tree.start_rename(root, path);
                    }
                    Err(error) => report!("папка не создана: {error}"),
                }
                return false;
            }
            Some(TreeAction::SortMenu { button }) => {
                self.open_sort_menu(root, button);
                return false;
            }
            Some(TreeAction::Move { sources, target }) => {
                self.commit_note();
                file_ops::move_all(&self.workspace.core, sources, target);
                return false;
            }
            Some(TreeAction::ContextMenu { targets, clicked, folder, at }) => {
                self.open_file_menu(root, targets, clicked, folder, at, window, None);
                return false;
            }
            None => {}
        }
        let Some(chrome) = &self.chrome else { return false };
        if id == chrome.toggle_sidebar {
            let open = !self.interface.sidebar_open();
            self.interface.set_sidebar_open(open);
            chrome.sidebar.edit(root, |mut sidebar| crate::ui::widgets::Collapse::set_open(&mut sidebar, open));
            root.edit_widget(id, |mut w| crate::ui::widgets::IconButton::set_label(&mut w.downcast(), &crate::i18n::t(crate::ui::sidebar_label(open))));
            self.shown = Some(self.visible_panels());
            self.apply_widths(root);
        } else if (id == chrome.settings || id == chrome.rail_settings) && action.downcast_ref::<Pressed>().is_some() {
            self.open_settings(root);
        } else if id == chrome.search_button && action.downcast_ref::<Pressed>().is_some() {
            self.open_search(root);
        } else if id == chrome.tab_list && action.downcast_ref::<Pressed>().is_some() {
            self.open_tabs_menu(root, id);
        } else if Some(id) == chrome.graph_button && action.downcast_ref::<Pressed>().is_some() {
            self.open_graph(root);
            return true;
        } else if Some(id) == chrome.home_button
            && action.downcast_ref::<Pressed>().is_some()
            && let Some(home) = self.home.clone()
        {
            return self.open_in(root, home, true);
        }
        false
    }
}
