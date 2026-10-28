mod actions;
mod back;
mod caches;

pub(crate) use caches::Moment;
mod commands;
mod book;
mod confirm;
mod core_events;
mod embeds;
mod file_menu;
mod find;
mod editor_requests;
mod graph_page;
mod history;
mod home;
mod init;
mod lifecycle;
mod media;
mod note_page;
mod panel;
#[cfg(feature = "memory-probe")]
mod probe;
mod reader;
mod root_ui;
mod search;
mod settings;
mod tabs;
mod templates;
mod widths;
mod update;
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

use crate::i18n::{self, t};
use crate::ui::dialog::{DialogAnswer, DialogIds};
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

pub(in crate::app) enum Popup {
    FileMenu { layer: WidgetId, targets: Vec<PathBuf>, clicked: PathBuf, tab: Option<Uuid> },
    Settings { dialog: SettingsDialog, menu: Option<settings::OpenMenu> },
    Menu { layer: WidgetId, commands: Vec<MenuCommand> },
    SortMenu { layer: WidgetId, button: WidgetId },
    TabsMenu { layer: WidgetId, button: WidgetId },
    Viewer { layer: WidgetId },
    Reader(Box<reader::ReaderState>),
    Search(search::SearchState),
    Templates(templates::TemplatesState),
    Workspaces(workspaces::WorkspacesState),
    Confirm(DialogIds, confirm::Confirmed),
}

#[derive(Clone, Copy, PartialEq)]
pub(in crate::app) enum MenuCommand {
    Version(usize, history::VersionCommand),
    Table(crate::ui::editor::TableCommand),
    Editor(editor_requests::EditorCommand),
    Note(note_page::menu::NoteCommand),
}

#[derive(Default)]
pub struct Outcome {
    pub title: bool,
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

const MENU_RENAME: usize = 0;
const MENU_DUPLICATE: usize = 1;
const MENU_DELETE: usize = 2;
const MENU_CLOSE: usize = 3;
const MENU_COLOR: usize = 3;
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
    note: Option<Note>,
    opening: Option<tabs::reading::Opening>,
    text: crop::Rope,
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
    resizing: Option<(crate::interface::Side, f64, f64)>,
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
    view: Option<(usize, f64)>,
    view_due: Option<Instant>,
    restore: Option<(usize, f64)>,
    embeds: Option<embeds::Embeds>,
    navigation: crate::navigation::Navigation,
    renaming_to: Option<PathBuf>,
    hero_shown: Option<Vec<String>>,
    picked_tx: Sender<book::Picked>,
    failed_renames: (Sender<(PathBuf, PathBuf)>, Receiver<(PathBuf, PathBuf)>),
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
    update: update::UpdateView,
    pub open_error: Option<(PathBuf, String)>,
    pub open_error_ids: Option<crate::ui::note::OpenErrorIds>,
    pub save_error_ids: Option<crate::ui::note::SaveErrorIds>,
    pub pending_reveal: Option<(PathBuf, usize)>,
}
