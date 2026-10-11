//! Состояние приложения поверх дерева виджетов: ядро, дерево файлов, открытая заметка.
//! Хост передаёт сюда действия виджетов и события ядра.

mod actions;
mod book;
mod core_events;
mod embeds;
mod file_menu;
mod find;
mod editor_requests;
mod graph_page;
mod history;
mod home;
mod media;
mod note_page;
mod panel;
mod reader;
mod search;
mod settings;
mod tabs;
mod templates;
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
use crate::ui::editor::TextAction;

use crate::i18n::{self, t, t_with};
use crate::ui::dialog::{ConfirmDialog, DialogAnswer, DialogIds};
use crate::ui::file_tree::{FileTree, TreeAction};
use crate::ui::menu::{Menu, MenuChoice};
use crate::ui::theme::{self, UiFont};
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
    EditorMenu { layer: WidgetId, commands: Vec<editor_requests::EditorCommand> },
    NoteActionsMenu { layer: WidgetId },
    Viewer { layer: WidgetId },
    Reader(Box<reader::ReaderState>),
    Search(search::SearchState),
    Templates(templates::TemplatesState),
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
const MENU_CLOSE: usize = 3;
/// Отступ меню от края окна при развороте, как `VIEWPORT_MARGIN_PX`.
const MENU_MARGIN: f64 = 8.0;
const HOLD: std::time::Duration = std::time::Duration::from_millis(500);

pub struct App {
    workspace: Workspace,
    tree: FileTree,
    chrome: Option<Chrome>,
    interface: Interface,
    popup: Option<Popup>,
    typing: Option<(crate::ui::settings::schema::Setter, String, usize)>,
    graph: graph_page::GraphState,
    suggest_layer: Option<(WidgetId, WidgetId)>,
    find: Option<crate::ui::find::FindBar>,
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
    retention_done: Arc<std::sync::atomic::AtomicBool>,
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
    shown: Option<(bool, bool)>,
    titles_tx: Sender<(WidgetId, String, String)>,
    titles_rx: Receiver<(WidgetId, String, String)>,
    hold: Option<Instant>,
    updater: crate::updater::Updater,
    update_shown: crate::updater::Status,
    update_layer: Option<WidgetId>,
    pub open_error: Option<(PathBuf, String)>,
    pub open_error_ids: Option<crate::ui::note::OpenErrorIds>,
    pub save_error_ids: Option<crate::ui::note::SaveErrorIds>,
    pub pending_reveal: Option<usize>,
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
        let (titles_tx, titles_rx) = channel();
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
            find: None,
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
            retention_done: Arc::default(),
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
            shown: None,
            titles_tx,
            titles_rx,
            hold: None,
            updater: crate::updater::Updater::new(Arc::clone(&workspace_wake)),
            update_shown: crate::updater::Status::Idle,
            update_layer: None,
            open_error: None,
            open_error_ids: None,
            save_error_ids: None,
            pending_reveal: None,
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
        app.cleanup_retention();
        app
    }

    /// Дерево виджетов окна; `system_dark` — тема системы для настройки `system`.
    pub fn root_widget(&mut self, system_dark: bool) -> NewWidget<dyn Widget> {
        self.system_dark = system_dark;
        ui::widgets::Appear::new(Slot::new(self.build_ui()).with_tag(ROOT_TAG), ui::widgets::NORMAL).erased()
    }

    /// Пересобирает интерфейс по текущим настройкам (тема, язык, шрифт).
    fn visible_panels(&self) -> (bool, bool) {
        (self.interface.sidebar_open() && !self.interface.focus_mode(), self.panel_visible())
    }

    fn replace_root(&mut self, root: &mut RenderRoot) {
        let widget = self.build_ui();
        root.set_default_properties(Arc::new(theme::default_properties()));
        root.edit_widget_with_tag(ROOT_TAG, |mut slot| Slot::set_child(&mut slot, widget));
        self.render_panel(root);
    }

    pub(super) fn restyle(&mut self, root: &mut RenderRoot, fonts: bool) {
        self.apply_appearance();
        root.restyle(Arc::new(theme::default_properties()), fonts);
    }

    fn apply_appearance(&self) {
        let config = self.workspace.core.settings.get_config();
        theme::apply_setting(&config.ui.theme, self.system_dark, &config.ui.primary_color);
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
    }

    fn build_ui(&mut self) -> NewWidget<dyn Widget> {
        self.apply_appearance();
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
        let target = self.visible_panels();
        let from = self.shown.replace(target).unwrap_or(target);
        let parts = ui::RootParts {
            workspace_name: &workspace_name,
            note,
            sidebar_open: target.0,
            sidebar_from: from.0,
            focus_mode: self.interface.focus_mode(),
            home: self.home.is_some(),
            tabs,
            empty_tab: self.tabs.active().path.is_none() && self.note.is_none() && self.tree.root().is_some(),
            vault_open: self.tree.root().is_some(),
            vault_error: self.vault_error.as_deref(),
            panel_open: target.1,
            panel_from: from.1,
            panel_mode: self.panel_mode(),
            has_analysis: !self.panel_methods().is_empty(),
            graph,
            graph_active,
        };
        let (widget, chrome) = ui::root(self.tree.widget(), parts);
        self.chrome = Some(chrome);
        widget.erased()
    }

    pub fn title(&self) -> String {
        match &self.note {
            Some(note) => format!("{} — Aquilum", note.path.file_stem().unwrap_or_default().to_string_lossy()),
            None => "Aquilum".to_owned(),
        }
    }

    /// Когда записать отложенную правку заметки — хост ждёт до этого момента.
    pub fn deadline(&self) -> Option<Instant> {
        [self.note.as_ref().and_then(Note::deadline), self.view_due, self.hold].into_iter().flatten().min()
    }

    /// Пора: записывает отложенную правку.
    pub fn tick(&mut self, root: &mut RenderRoot, now: Instant) {
        if self.view_due.is_some_and(|t| t <= now) {
            self.flush_view();
        }
        if let Some(note) = self.note.as_mut().filter(|n| n.deadline().is_some_and(|t| t <= now)) {
            let had = note.save_error.is_some();
            if note.save(&self.workspace.core, &self.editor_text).is_err() || had {
                self.refresh_save_error(root);
            }
        }
        if let Some(text) = self.note.as_mut().and_then(crate::note::Note::take_replaced) {
            self.set_editor_text(root, text);
            self.refresh_save_error(root);
        }
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
            let _ = note.save(&self.workspace.core, &self.editor_text);
        }
        self.workspace.core.shutdown();
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
