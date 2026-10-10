//! Состояние приложения поверх дерева виджетов: ядро, дерево файлов, открытая заметка.
//! Хост передаёт сюда действия виджетов и события ядра.

mod book;
mod embeds;
mod editor_requests;
mod graph_page;
mod history;
mod kanban;
mod note_page;
mod panel;
mod reader;
mod search;
mod settings;
mod tabs;
mod workspaces;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use crate::ui::editor::TextEditor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

use aquilum_core::CoreEvent;
use aquilum_core::mcp::bridge::{Disposition, Navigation};
use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, NewWidget, Widget, WidgetId};
use masonry::kurbo::{Point, Size};
use masonry::layout::Length;
use masonry::widgets::{SizedBox, TextAction};

use crate::i18n::{self, t, t_with};
use crate::ui::dialog::{ConfirmDialog, DialogAnswer, DialogIds};
use crate::ui::file_tree::{FileTree, TreeAction};
use crate::ui::menu::{Menu, MenuChoice};
use crate::ui::theme::{self, UiFont};
use crate::ui::tokens::size;
use crate::ui::settings::SettingsDialog;
use crate::ui::handle::Handle;
use crate::ui::titlebar::TitlebarAction;
use crate::ui::widgets::{Pressed, Slot};
use crate::ui::{self, Chrome, ROOT_TAG};
use crate::file_ops;
use crate::interface::{Interface, SCALE_STEP};
use crate::links::Listing;
use crate::ui::backlinks::{Body, HistoryBody, VersionView};
use crate::ui::titlebar::WindowButton;
use crate::note::Note;
use crate::session::Session;
use crate::tabs::Tabs;
use crate::workspace::Workspace;
use uuid::Uuid;

/// Всплывающий слой над окном и то, к чему он относится.
enum Popup {
    /// Меню действий над файлами дерева или над заметкой вкладки (`tab`).
    FileMenu { layer: WidgetId, targets: Vec<PathBuf>, clicked: PathBuf, tab: Option<Uuid> },
    /// Подтверждение удаления.
    Delete { dialog: DialogIds, targets: Vec<PathBuf> },
    /// Окно настроек и открытый из него список выбора.
    Settings { dialog: SettingsDialog, menu: Option<settings::OpenMenu> },
    VersionMenu { layer: WidgetId, index: usize, commands: Vec<history::VersionCommand> },
    TableMenu { layer: WidgetId, commands: Vec<crate::ui::editor::TableCommand> },
    Viewer { layer: WidgetId },
    Reader(Box<reader::ReaderState>),
    Search(search::SearchState),
    Workspaces(workspaces::WorkspacesState),
}

/// Что после действия должен сделать хост.
#[derive(Default)]
pub struct Outcome {
    /// Сменилась открытая заметка — обновить заголовок окна.
    pub title: bool,
    /// Сменился масштаб интерфейса — переложить окно.
    pub rescale: bool,
    pub window: Option<WindowCommand>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowCommand {
    Drag,
    ToggleMaximize,
    Minimize,
    Close,
}

/// Пункты меню файла — `fileActionItems.ts`.
const MENU_RENAME: usize = 0;
const MENU_DUPLICATE: usize = 1;
const MENU_DELETE: usize = 2;
/// Отступ меню от края окна при развороте, как `VIEWPORT_MARGIN_PX`.
const MENU_MARGIN: f64 = 8.0;
const HOLD: std::time::Duration = std::time::Duration::from_millis(500);

pub struct App {
    workspace: Workspace,
    tree: FileTree,
    chrome: Option<Chrome>,
    interface: Interface,
    popup: Option<Popup>,
    typing: Option<(crate::ui::settings::schema::Setter, String)>,
    graph: graph_page::GraphState,
    suggest_layer: Option<(WidgetId, WidgetId)>,
    /// Открытая заметка; `None` — пустая вкладка или текст из `--file`.
    note: Option<Note>,
    /// Текст в редакторе — чтобы пересобранный интерфейс показал его же.
    editor_text: String,
    /// Тема системы — для настройки темы `system` при пересборке.
    system_dark: bool,
    tabs: Tabs,
    renaming_tab: Option<Uuid>,
    rename_field: Option<Handle<TextEditor>>,
    session: Option<Session>,
    maximized: bool,
    listing: Option<Listing>,
    panel_key: Option<(PathBuf, crate::links::Mode, crate::links::Method)>,
    panel_generation: u64,
    panel_tx: Sender<Listing>,
    panel_rx: Receiver<Listing>,
    panel_body: Option<Body>,
    controls: Option<(WidgetId, Handle<WindowButton>)>,
    window: Size,
    history: Option<history::HistoryState>,
    history_body: Option<HistoryBody>,
    opened_versions: HashMap<Uuid, crate::history::Version>,
    version_view: Option<VersionView>,
    version_notice: Option<String>,
    search_tx: Sender<search::Found>,
    search_rx: Receiver<search::Found>,
    vault_error: Option<String>,
    note_ids: Option<crate::ui::note::NoteIds>,
    hidden: usize,
    view: Option<(i64, f64)>,
    view_due: Option<Instant>,
    restore: Option<(i64, f64)>,
    embeds: Option<embeds::Embeds>,
    navigation: crate::navigation::Navigation,
    renaming_to: Option<PathBuf>,
    hero_shown: Option<Vec<String>>,
    picked_tx: Sender<book::Picked>,
    picked_rx: Receiver<book::Picked>,
    home: Option<PathBuf>,
    home_tx: Sender<(u64, Option<PathBuf>)>,
    home_rx: Receiver<(u64, Option<PathBuf>)>,
    home_generation: u64,
    hold: Option<Instant>,
    updater: crate::updater::Updater,
    update_shown: crate::updater::Status,
    update_layer: Option<WidgetId>,
    kanban: kanban::KanbanState,
}

impl App {
    pub fn new(workspace: Workspace, initial_text: String) -> Self {
        let tree = FileTree::new(workspace.root.clone(), &workspace.data_dir);
        let interface = Interface::load(&workspace.data_dir);
        let navigation = crate::navigation::Navigation::load(&workspace.data_dir, workspace.root.as_deref());
        let (session, tabs) = match workspace.root.as_deref().and_then(|root| Session::open(&workspace.core, root)) {
            Some((session, tabs)) => (Some(session), tabs),
            None => (None, Tabs::default()),
        };
        let (panel_tx, panel_rx) = channel();
        let (search_tx, search_rx) = channel();
        let (picked_tx, picked_rx) = channel();
        let (home_tx, home_rx) = channel();
        let workspace_wake = Arc::clone(&workspace.wake);
        let auto_update = workspace.core.settings.get_config().updates.auto;
        let mut app = App {
            workspace,
            tree,
            chrome: None,
            interface,
            popup: None,
            typing: None,
            graph: graph_page::GraphState::default(),
            suggest_layer: None,
            note: None,
            editor_text: initial_text,
            system_dark: false,
            tabs,
            renaming_tab: None,
            rename_field: None,
            session,
            maximized: false,
            listing: None,
            panel_key: None,
            panel_generation: 0,
            panel_tx,
            panel_rx,
            panel_body: None,
            controls: None,
            window: Size::ZERO,
            history: None,
            history_body: None,
            opened_versions: HashMap::new(),
            version_view: None,
            version_notice: None,
            search_tx,
            search_rx,
            vault_error: None,
            note_ids: None,
            hidden: 0,
            view: None,
            view_due: None,
            restore: None,
            embeds: None,
            navigation,
            renaming_to: None,
            hero_shown: None,
            picked_tx,
            picked_rx,
            home: None,
            home_tx,
            home_rx,
            home_generation: 0,
            hold: None,
            updater: crate::updater::Updater::new(Arc::clone(&workspace_wake)),
            update_shown: crate::updater::Status::Idle,
            update_layer: None,
            kanban: kanban::KanbanState::default(),
        };
        crate::cover::set_waker(Arc::clone(&app.workspace.wake));
        app.start_index();
        app.workspace.core.apply_mcp_settings();
        if auto_update {
            app.updater.install();
        }
        let remote = app.workspace.data_dir.join("remote");
        std::thread::spawn(move || embeds::prune_remote(&remote));
        app.refresh_home();
        app
    }

    /// Дерево виджетов окна; `system_dark` — тема системы для настройки `system`.
    pub fn root_widget(&mut self, system_dark: bool) -> NewWidget<dyn Widget> {
        self.system_dark = system_dark;
        Slot::new(self.build_ui()).with_tag(ROOT_TAG).erased()
    }

    /// Пересобирает интерфейс по текущим настройкам (тема, язык, шрифт).
    fn replace_root(&mut self, root: &mut RenderRoot) {
        let widget = self.build_ui();
        root.set_default_properties(Arc::new(theme::default_properties()));
        root.edit_widget_with_tag(ROOT_TAG, |mut slot| Slot::set_child(&mut slot, widget));
        let window = self.window;
        self.attach_window_controls(root, window);
        self.render_panel(root);
    }

    fn build_ui(&mut self) -> NewWidget<dyn Widget> {
        let system_dark = self.system_dark;
        let config = self.workspace.core.settings.get_config();
        theme::apply_setting(&config.ui.theme, system_dark, &config.ui.primary_color);
        i18n::set_language(&config.ui.language);
        theme::set_ui_font(UiFont {
            family: config.ui.font.font_family.clone(),
            size: config.ui.font.font_size_base as f32,
            weight: config.ui.font.font_weight as f32,
        });
        theme::set_editor_font(theme::EditorFont {
            family: config.editor.font.font_family.clone(),
            size: config.editor.font.font_size_base as f32,
            weight: [400.0, 450.0, 500.0, 550.0, 600.0]
                .into_iter()
                .min_by(|a: &f32, b: &f32| (a - config.editor.font.font_weight as f32).abs().total_cmp(&(b - config.editor.font.font_weight as f32).abs()))
                .unwrap_or(400.0),
            line_height: config.editor.line_height,
            max_width_ch: config.editor.max_width_ch,
        });
        let workspace_name = self
            .tree
            .root()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let tabs = self.tab_widgets();
        let note = self.note_widget();
        let graph = self.graph_widget();
        let graph_active = self.tabs.active().graph && self.tree.root().is_some();
        let kanban = self.kanban_widget().unwrap_or_else(|| NewWidget::new(masonry::widgets::SizedBox::empty()).erased());
        let parts = ui::RootParts {
            workspace_name: &workspace_name,
            note,
            sidebar_open: self.interface.sidebar_open(),
            focus_mode: self.interface.focus_mode(),
            home: self.home.is_some(),
            tabs,
            empty_tab: self.tabs.active().path.is_none() && self.note.is_none() && self.tree.root().is_some(),
            vault_open: self.tree.root().is_some(),
            vault_error: self.vault_error.as_deref(),
            panel_open: self.panel_visible(),
            panel_mode: self.panel_mode(),
            has_analysis: !self.panel_methods().is_empty(),
            graph,
            graph_active,
            kanban,
            kanban_active: self.tabs.active().is_base() && self.tree.root().is_some(),
        };
        let (widget, chrome) = ui::root(self.tree.widget(), parts);
        self.chrome = Some(chrome);
        widget.erased()
    }

    pub fn title(&self) -> String {
        match &self.note {
            Some(note) => format!("{} — Aquilum", note.path.file_stem().unwrap_or_default().to_string_lossy()),
            None => self.tabs.active().title().map(|title| format!("{title} — Aquilum")).unwrap_or_else(|| "Aquilum".to_owned()),
        }
    }

    /// Действие виджета; `window` — размер окна в логических пикселях (для меню у края).
    pub fn on_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: ErasedAction, window: Size) -> Outcome {
        if self.on_suggest_pick(root, id, &action) {
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
        if matches!(self.popup, Some(Popup::Workspaces(_))) {
            return self.on_workspaces_action(root, id, &action);
        }
        if self.popup.is_some() {
            self.on_popup_action(root, id, &action);
            return Outcome::default();
        }
        if self.is_graph_widget(id) {
            let title = self.on_graph_action(root, id, &action);
            return Outcome { title, ..Outcome::default() };
        }
        if self.kanban_picker_open() {
            self.on_board_picker_action(root, id);
            return Outcome::default();
        }
        if self.is_kanban_widget(id) {
            self.on_kanban_action(root, id, &action);
            return Outcome { title: true, ..Outcome::default() };
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

    fn create_note_titled(&mut self, root: &mut RenderRoot, title: Option<&str>) -> bool {
        let Some(workspace) = self.tree.root().map(Path::to_path_buf) else { return false };
        match file_ops::create_note(&self.workspace.core, &workspace, title) {
            Ok(path) => {
                self.tree.paths_changed(root, std::slice::from_ref(&path));
                self.open(root, path)
            }
            Err(error) => {
                eprintln!("заметка не создана: {error}");
                false
            }
        }
    }

    /// Действие в основном окне; `true` — сменилась открытая заметка (и заголовок окна).
    fn on_main_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction, window: Size) -> bool {
        match self.tree.on_action(root, id, action) {
            Some(TreeAction::Open { path, new_tab }) => return self.open_in(root, path, new_tab),
            Some(TreeAction::Rename { from, to }) => {
                file_ops::rename(&self.workspace.core, from, to);
                return false;
            }
            Some(TreeAction::CreateNote) => return self.create_note_titled(root, None),
            Some(TreeAction::Move { sources, target }) => {
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
            let width = if open { size::SIDEBAR_WIDTH } else { 0.0 };
            chrome.sidebar.edit(root, |mut sidebar| SizedBox::set_width(&mut sidebar, Length::px(width)));
            chrome.sidebar_border.edit(root, |mut rule| crate::ui::widgets::Rule::set_hidden(&mut rule, !open));
        } else if (id == chrome.settings || id == chrome.rail_settings) && action.downcast_ref::<Pressed>().is_some() {
            self.open_settings(root);
        } else if Some(id) == chrome.graph_button && action.downcast_ref::<Pressed>().is_some() {
            self.open_graph(root);
            return true;
        } else if Some(id) == chrome.kanban_button && action.downcast_ref::<Pressed>().is_some() {
            self.open_boards(root);
            return false;
        } else if Some(id) == chrome.home_button
            && action.downcast_ref::<Pressed>().is_some()
            && let Some(home) = self.home.clone()
        {
            return self.open_in(root, home, true);
        }
        false
    }

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
        self.on_kanban_results(root);
        self.on_graph_results(root);
        self.on_update(root);
        let mut links_changed = false;
        let mut kanban_index_workspace = false;
        let mut history_changed = false;
        let mut changed = Vec::new();
        let mut navigations = Vec::new();
        for event in self.workspace.drain_events() {
            match event {
                CoreEvent::Navigate(navigation) => navigations.push(navigation),
                CoreEvent::WorkspaceChanged(paths) => changed.extend(paths.into_iter().map(PathBuf::from)),
                CoreEvent::LinksChanged(revision) => {
                    links_changed = true;
                    kanban_index_workspace |= self.tree.root().is_some_and(|root| root == Path::new(&revision.workspace_path));
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
        if kanban_index_workspace && let Some(workspace) = self.tree.root().map(Path::to_path_buf) {
            self.kanban_index_changed(&workspace);
        }
        if !changed.is_empty() {
            self.kanban_paths_changed(&changed);
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

    /// Когда записать отложенную правку заметки — хост ждёт до этого момента.
    pub fn deadline(&self) -> Option<Instant> {
        [self.note.as_ref().and_then(Note::deadline), self.view_due, self.hold].into_iter().flatten().min()
    }

    /// Пора: записывает отложенную правку.
    pub fn tick(&mut self, now: Instant) {
        if self.view_due.is_some_and(|t| t <= now) {
            self.flush_view();
        }
        if let Some(note) = self.note.as_mut().filter(|n| n.deadline().is_some_and(|t| t <= now)) {
            note.save(&self.workspace.core);
        }
    }

    fn open_file_menu(
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
        let labels = [t("common.rename"), t("common.duplicate"), t("common.delete")];
        let icons = [crate::ui::icons::PENCIL, crate::ui::icons::COPY, crate::ui::icons::TRASH];
        let items: Vec<crate::ui::menu::Entry<'_>> = labels.iter().zip(icons).map(|(label, icon)| crate::ui::menu::Entry { label, icon: Some(icon), disabled: false }).collect();
        let menu = Menu::entries(&label, &items);
        // Разворот у края окна, как в Menu.tsx; размер меню — оценка по пунктам.
        let estimate = Menu::estimated_size(items.len());
        let x = if at.x + estimate.width > window.width { (at.x - estimate.width).max(MENU_MARGIN) } else { at.x };
        let y = if at.y + estimate.height > window.height { (at.y - estimate.height).max(MENU_MARGIN) } else { at.y };
        let layer = menu.id();
        root.add_layer(menu, Point::new(x, y));
        // Фокус на меню, чтобы до него дошёл Escape.
        root.focus_on(Some(layer));
        self.popup = Some(Popup::FileMenu { layer, targets, clicked, tab });
    }

    fn on_popup_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) {
        match self.popup.take() {
            Some(Popup::FileMenu { layer, targets, clicked, tab }) => {
                let Some(choice) = action.downcast_ref::<MenuChoice>() else {
                    self.popup = Some(Popup::FileMenu { layer, targets, clicked, tab });
                    return;
                };
                if let MenuChoice::Item(_) = choice {
                    root.remove_layer(layer);
                }
                match choice {
                    MenuChoice::Item(MENU_DUPLICATE) => file_ops::duplicate_all(&self.workspace.core, targets),
                    MenuChoice::Item(MENU_DELETE) => self.confirm_delete(root, targets),
                    // Как `startRename(path)`: переименовывается строка, по которой щёлкнули.
                    MenuChoice::Item(MENU_RENAME) if tab.is_some() => self.start_tab_rename(root, &clicked),
                    MenuChoice::Item(MENU_RENAME) => self.tree.start_rename(root, clicked),
                    MenuChoice::Item(_) | MenuChoice::Dismissed => {}
                }
            }
            Some(Popup::Delete { dialog, targets }) => match dialog.answer(id, action) {
                Some(answer) => {
                    root.remove_layer(dialog.layer);
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
            Some(Popup::Viewer { layer }) => {
                if action.downcast_ref::<crate::ui::viewer::ViewerClosed>().is_some() {
                    root.remove_layer(layer);
                } else {
                    self.popup = Some(Popup::Viewer { layer });
                }
            }
            // Окно настроек разбирает свои действия само (`on_settings_action`).
            Some(popup @ (Popup::Settings { .. } | Popup::Search(_) | Popup::Workspaces(_) | Popup::Reader(_))) => self.popup = Some(popup),
            None => {}
        }
    }

    /// `DeleteNotesDialog`: заголовок и текст зависят от числа целей и от того, папка ли это.
    fn confirm_delete(&mut self, root: &mut RenderRoot, targets: Vec<PathBuf>) {
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

    /// Масштаб интерфейса (множитель к масштабу окна).
    pub fn scale(&self) -> f64 {
        self.interface.scale()
    }

    /// Горячие клавиши масштаба — `initScaling`: Ctrl(⌘)+= или +, Ctrl+−, Ctrl+0.
    /// Возвращает `true`, если масштаб изменился.
    pub fn zoom_key(&mut self, key: &str) -> bool {
        let scale = self.interface.scale();
        match key {
            "=" | "+" => self.interface.set_scale(scale + SCALE_STEP),
            "-" => self.interface.set_scale(scale - SCALE_STEP),
            "0" => self.interface.set_scale(1.0),
            _ => false,
        }
    }

    /// Ctrl+колесо: шаг масштаба по направлению прокрутки.
    pub fn zoom_wheel(&mut self, up: bool) -> bool {
        let step = if up { SCALE_STEP } else { -SCALE_STEP };
        self.interface.set_scale(self.interface.scale() + step)
    }

    /// Дописывает несохранённое и останавливает фоновые задачи ядра.
    pub fn shutdown(&mut self) {
        self.flush_view();
        if let Some(note) = &mut self.note {
            note.save(&self.workspace.core);
        }
        self.workspace.core.shutdown();
    }

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
                .map_err(|error| eprintln!("домашняя страница не найдена: {error:?}"))
                .ok()
                .and_then(|r| r.paths.into_iter().next().flatten())
                .map(PathBuf::from);
            if sender.send((generation, path)).is_ok() {
                wake();
            }
        });
    }

    fn on_home_resolved(&mut self, root: &mut RenderRoot) {
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

    pub fn hold_due(&mut self, now: Instant) -> bool {
        self.hold.is_some_and(|until| until <= now) && self.hold.take().is_some()
    }

    pub fn holding(&mut self) -> bool {
        let loading = self.embeds.as_ref().is_some_and(embeds::Embeds::busy);
        if !loading || self.hold.is_some_and(|until| until <= Instant::now()) {
            self.hold = None;
        }
        self.hold.is_some()
    }

    fn on_update(&mut self, root: &mut RenderRoot) {
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
                eprintln!("обновление не установлено: {error}");
            }
        }
    }

    pub fn editor_id(&self) -> Option<WidgetId> {
        self.note_ids.as_ref().map(|ids| ids.body.id())
    }

    pub fn restore(&mut self, root: &mut RenderRoot) -> bool {
        self.show_active(root);
        self.note.is_some()
    }

    pub fn forget_session(&mut self) {
        self.session = None;
        self.tabs = Tabs::default();
    }
}
