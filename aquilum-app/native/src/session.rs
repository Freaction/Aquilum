use std::path::{Path, PathBuf};

use aquilum_core::Core;
use aquilum_core::search::paths::{is_markdown, strip_root};
use aquilum_core::ui_state::models::{
    OpenSessionInput, SaveStateBatchInput, SessionStateInput, TabKind, TabState, ViewStateInput,
};
use aquilum_core::ui_state::paths::{normalize_base_relative, normalize_relative};
use uuid::Uuid;

use crate::tabs::{Tab, Tabs};

const WINDOW_ID: &str = "main";
const PANE_ID: &str = "main";

pub struct Session {
    root: PathBuf,
    workspace_id: Uuid,
    epoch: Uuid,
    sequence: i64,
    pub camera: Option<(f64, f64, f64)>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

pub fn relative(root: &Path, path: &Path) -> Option<String> {
    let rest = strip_root(root, path)?;
    let parts: Vec<String> = rest
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    normalize_relative(&parts.join("/")).ok()
}

fn base_path(root: &Path, path: &Path) -> Option<String> {
    let relative = normalize_base_relative(&relative(root, path)?).ok()?;
    if !path.is_file()
        || !path
            .canonicalize()
            .ok()?
            .starts_with(root.canonicalize().ok()?)
    {
        return None;
    }
    Some(relative)
}

impl Session {
    pub fn open(core: &Core, root: &Path) -> Option<(Session, Tabs)> {
        let workspace_id = core
            .ui_state
            .resolve_workspace(&root.to_string_lossy(), now_ms())
            .ok()?;
        let epoch = Uuid::new_v4();
        let input = OpenSessionInput {
            workspace_id,
            window_id: WINDOW_ID.into(),
            epoch,
            now_ms: now_ms(),
        };
        let loaded = match core.ui_state.open_session(&input) {
            Ok(loaded) => loaded,
            Err(error) => {
                eprintln!("сессия вкладок не открыта: {error:?}");
                return None;
            }
        };
        let mut stored = loaded.tabs;
        stored.sort_by_key(|tab| tab.position);
        let tabs = stored
            .into_iter()
            .filter_map(|tab| match tab.kind {
                TabKind::Empty => Some(Tab {
                    id: tab.tab_id,
                    path: None,
                    document_id: None,
                    graph: false,
                }),
                TabKind::Document => {
                    let path = root.join(
                        tab.relative_path?
                            .replace('/', std::path::MAIN_SEPARATOR_STR),
                    );
                    path.is_file().then_some(Tab {
                        id: tab.tab_id,
                        path: Some(path),
                        document_id: tab.document_id,
                        graph: false,
                    })
                }
                TabKind::Base => {
                    let relative = normalize_base_relative(&tab.relative_path?).ok()?;
                    let path = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
                    base_path(root, &path)?;
                    Some(Tab {
                        id: tab.tab_id,
                        ..Tab::base(path)
                    })
                }
                TabKind::Graph => Some(Tab {
                    id: tab.tab_id,
                    path: None,
                    document_id: None,
                    graph: true,
                }),
            })
            .collect();
        let camera = loaded
            .graph_camera
            .map(|c| (c.center_x, c.center_y, c.scale));
        let session = Session {
            root: root.to_path_buf(),
            workspace_id,
            epoch,
            sequence: 0,
            camera,
        };
        Some((session, Tabs::restore(tabs, loaded.active_tab_id)))
    }

    pub fn save(&mut self, core: &Core, tabs: &mut Tabs) {
        let ids: Vec<Uuid> = tabs.tabs().iter().map(|t| t.id).collect();
        for id in ids {
            let Some(tab) = tabs.tab_mut(id) else {
                continue;
            };
            if tab.document_id.is_some() || !tab.path.as_deref().is_some_and(is_markdown) {
                continue;
            }
            let Some(relative) = tab.path.as_deref().and_then(|p| relative(&self.root, p)) else {
                continue;
            };
            match core.ui_state.resolve_document(self.workspace_id, &relative) {
                Ok(document) => tab.document_id = Some(document),
                Err(error) => eprintln!("заметка {relative} не записана в сессию: {error:?}"),
            }
        }
        let stored: Vec<TabState> = tabs
            .tabs()
            .iter()
            .enumerate()
            .filter_map(|(position, tab)| {
                let (kind, document_id, relative_path) = if tab.graph {
                    (TabKind::Graph, None, None)
                } else if tab.is_base() {
                    let relative = base_path(&self.root, tab.path.as_deref()?)?;
                    (TabKind::Base, None, Some(relative))
                } else if let Some(path) = &tab.path {
                    if !is_markdown(path) {
                        return None;
                    }
                    (TabKind::Document, Some(tab.document_id?), None)
                } else {
                    (TabKind::Empty, None, None)
                };
                Some(TabState {
                    tab_id: tab.id,
                    document_id,
                    relative_path,
                    kind,
                    position: position as i64,
                })
            })
            .collect();
        let active_tab_id = stored
            .iter()
            .find(|tab| tab.tab_id == tabs.active_id())
            .or(stored.first())
            .map(|tab| tab.tab_id);
        self.sequence += 1;
        let batch = SaveStateBatchInput {
            workspace_id: self.workspace_id,
            window_id: WINDOW_ID.into(),
            epoch: self.epoch,
            sequence: self.sequence,
            now_ms: now_ms(),
            session: Some(SessionStateInput {
                active_tab_id,
                tabs: Some(stored),
            }),
            views: Vec::new(),
            graph_camera: None,
        };
        if let Err(error) = core.ui_state.save_batch(&batch) {
            eprintln!("сессия вкладок не сохранена: {error:?}");
        }
    }

    pub fn save_camera(&mut self, core: &Core, camera: (f64, f64, f64)) {
        self.camera = Some(camera);
        self.sequence += 1;
        let batch = SaveStateBatchInput {
            workspace_id: self.workspace_id,
            window_id: WINDOW_ID.into(),
            epoch: self.epoch,
            sequence: self.sequence,
            now_ms: now_ms(),
            session: None,
            views: Vec::new(),
            graph_camera: Some(aquilum_core::ui_state::models::GraphCameraState {
                center_x: camera.0,
                center_y: camera.1,
                scale: camera.2,
            }),
        };
        if let Err(error) = core.ui_state.save_batch(&batch) {
            eprintln!("камера графа не сохранена: {error:?}");
        }
    }

    pub fn document(&self, core: &Core, path: &Path) -> Option<Uuid> {
        if !is_markdown(path) {
            return None;
        }
        let relative = relative(&self.root, path)?;
        core.ui_state
            .resolve_document(self.workspace_id, &relative)
            .ok()
    }

    pub fn save_view(&mut self, core: &Core, document: Uuid, anchor: i64, offset: f64) {
        self.sequence += 1;
        let view = ViewStateInput {
            path: None,
            document_id: document,
            pane_id: PANE_ID.into(),
            cursor_anchor: Vec::new(),
            cursor_head: Vec::new(),
            fallback_anchor: 0,
            fallback_head: 0,
            scroll_anchor: Vec::new(),
            fallback_scroll_anchor: anchor.max(0),
            scroll_offset_px: offset,
            focused_surface: "editor".into(),
        };
        let batch = SaveStateBatchInput {
            workspace_id: self.workspace_id,
            window_id: WINDOW_ID.into(),
            epoch: self.epoch,
            sequence: self.sequence,
            now_ms: now_ms(),
            session: None,
            views: vec![view],
            graph_camera: None,
        };
        if let Err(error) = core.ui_state.save_batch(&batch) {
            eprintln!("позиция заметки не сохранена: {error:?}");
        }
    }

    pub fn load_view(&self, core: &Core, document: Uuid) -> Option<(i64, f64)> {
        let view = core
            .ui_state
            .load_view(self.workspace_id, WINDOW_ID, document, PANE_ID)
            .ok()??;
        Some((view.fallback_scroll_anchor, view.scroll_offset_px))
    }

    pub fn load_base_view(&self, core: &Core, path: &Path, view_count: usize) -> usize {
        let Some(relative) = relative(&self.root, path) else {
            return 0;
        };
        match core
            .ui_state
            .load_base_view(self.workspace_id, &relative, view_count)
        {
            Ok(selected) => selected,
            Err(error) => {
                eprintln!("вид базы {relative} не восстановлен: {error:?}");
                0
            }
        }
    }

    pub fn save_base_view(&self, core: &Core, path: &Path, selected: usize) {
        let Some(relative) = relative(&self.root, path) else {
            return;
        };
        if let Err(error) = core
            .ui_state
            .save_base_view(self.workspace_id, &relative, selected)
        {
            eprintln!("вид базы {relative} не сохранён: {error:?}");
        }
    }

    pub fn renamed(&self, core: &Core, tab: &Tab) {
        let (Some(document), Some(path)) = (tab.document_id, tab.path.as_deref()) else {
            return;
        };
        if tab.graph || !is_markdown(path) {
            return;
        }
        let Some(relative) = relative(&self.root, path) else {
            return;
        };
        if let Err(error) = core.ui_state.rename_document(document, &relative) {
            eprintln!("переименование {relative} не записано в сессию: {error:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_survive_reopening() {
        let vault = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        std::fs::create_dir(vault.path().join("Папка")).unwrap();
        std::fs::write(vault.path().join("Папка").join("a.md"), "").unwrap();
        std::fs::write(vault.path().join("b.md"), "").unwrap();
        let core = Core::open(data.path(), std::sync::Arc::new(|_| {}));
        let (mut session, mut tabs) = Session::open(&core, vault.path()).unwrap();
        tabs.open(vault.path().join("Папка").join("a.md"));
        tabs.new_tab();
        tabs.open(vault.path().join("b.md"));
        tabs.new_tab();
        let active = tabs.active_id();
        tabs.open_graph();
        tabs.select(active);
        session.save(&core, &mut tabs);
        session.save_camera(&core, (1.5, -2.0, 30.0));
        drop(session);

        let (reopened, restored) = Session::open(&core, vault.path()).unwrap();
        let paths: Vec<_> = restored.tabs().iter().map(|t| t.path.clone()).collect();
        assert_eq!(
            paths,
            [
                Some(vault.path().join("Папка").join("a.md")),
                Some(vault.path().join("b.md")),
                None,
                None
            ]
        );
        assert!(restored.tabs()[3].graph);
        assert_eq!(reopened.camera, Some((1.5, -2.0, 30.0)));
        assert_eq!(restored.active_id(), active);
        core.shutdown();
    }

    #[test]
    fn base_tabs_restore_without_document_identity_and_drop_invalid_paths() {
        let vault = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let board = vault.path().join("board.BASE");
        let missing = vault.path().join("missing.base");
        let outside = data.path().join("outside.base");
        std::fs::write(&board, "views: []").unwrap();
        std::fs::write(&outside, "views: []").unwrap();
        let core = Core::open(data.path(), std::sync::Arc::new(|_| {}));
        let (mut session, mut tabs) = Session::open(&core, vault.path()).unwrap();
        tabs.open(board.clone());
        let id = tabs.active_id();
        tabs.open_new(missing);
        tabs.open_new(outside);
        tabs.open_new(vault.path().join("other.txt"));
        session.save(&core, &mut tabs);
        assert!(tabs.tabs().iter().all(|tab| tab.document_id.is_none()));
        let (_, restored) = Session::open(&core, vault.path()).unwrap();
        assert_eq!(restored.tabs().len(), 1);
        assert_eq!(restored.active_id(), id);
        assert_eq!(restored.active().path.as_deref(), Some(board.as_path()));
        assert!(restored.active().is_base());
        assert!(restored.active().document_id.is_none());
        std::fs::remove_file(&board).unwrap();
        let (_, restored) = Session::open(&core, vault.path()).unwrap();
        assert!(restored.active().path.is_none());
        core.shutdown();
    }

    #[test]
    fn restored_base_records_ignore_missing_wrong_extension_and_escaping_paths() {
        let vault = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        std::fs::write(vault.path().join("note.md"), "").unwrap();
        let core = Core::open(data.path(), std::sync::Arc::new(|_| {}));
        let (session, _) = Session::open(&core, vault.path()).unwrap();
        let mut input = SaveStateBatchInput {
            workspace_id: session.workspace_id,
            window_id: WINDOW_ID.into(),
            epoch: session.epoch,
            sequence: 1,
            now_ms: 1,
            session: Some(SessionStateInput {
                active_tab_id: None,
                tabs: Some(vec![TabState {
                    tab_id: Uuid::new_v4(),
                    document_id: None,
                    kind: TabKind::Base,
                    relative_path: Some("missing.base".into()),
                    position: 0,
                }]),
            }),
            views: Vec::new(),
            graph_camera: None,
        };
        assert!(core.ui_state.save_batch(&input).unwrap());
        let (_, restored) = Session::open(&core, vault.path()).unwrap();
        assert!(restored.active().path.is_none());
        let database = aquilum_core::ui_state::database::UiStateDatabase::open(
            &data.path().join("ui-state.sqlite3"),
        )
        .unwrap();
        for path in ["note.md", "../outside.base", "missing.base"] {
            database
                .connection
                .execute(
                    "UPDATE tabs SET relative_path = ?1 WHERE kind = 'base'",
                    [path],
                )
                .unwrap();
            let (_, restored) = Session::open(&core, vault.path()).unwrap();
            assert!(restored.active().path.is_none());
        }
        for path in ["note.md", "../outside.base"] {
            input.session.as_mut().unwrap().tabs.as_mut().unwrap()[0].relative_path =
                Some(path.into());
            assert!(core.ui_state.save_batch(&input).is_err());
        }
        core.shutdown();
    }

    #[test]
    fn base_selected_view_is_scoped_persisted_and_clamped() {
        let vault = tempfile::tempdir().unwrap();
        let other_vault = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let board = vault.path().join("board.base");
        let core = Core::open(data.path(), std::sync::Arc::new(|_| {}));
        let (session, _) = Session::open(&core, vault.path()).unwrap();
        session.save_base_view(&core, &board, 4);
        assert_eq!(session.load_base_view(&core, &board, 7), 4);
        assert_eq!(session.load_base_view(&core, &board, 2), 1);
        assert_eq!(session.load_base_view(&core, &board, 0), 0);
        assert_eq!(
            session.load_base_view(&core, &vault.path().join("other.base"), 7),
            0
        );
        let (other, _) = Session::open(&core, other_vault.path()).unwrap();
        assert_eq!(
            other.load_base_view(&core, &other_vault.path().join("board.base"), 7),
            0
        );
        let (reopened, _) = Session::open(&core, vault.path()).unwrap();
        assert_eq!(reopened.load_base_view(&core, &board, 7), 4);
        core.shutdown();
    }

    #[test]
    fn note_to_base_transition_never_renames_document_identity_to_base_path() {
        let vault = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let note = vault.path().join("note.md");
        let base = vault.path().join("board.BASE");
        let restored_note = vault.path().join("restored.md");
        std::fs::write(&note, "").unwrap();
        let core = Core::open(data.path(), std::sync::Arc::new(|_| {}));
        let (mut session, mut tabs) = Session::open(&core, vault.path()).unwrap();
        tabs.open(note.clone());
        session.save(&core, &mut tabs);
        let document = tabs.active().document_id.unwrap();
        std::fs::rename(&note, &base).unwrap();
        let stale = Tab {
            document_id: Some(document),
            ..Tab::base(base.clone())
        };
        session.renamed(&core, &stale);
        assert_eq!(
            core.ui_state
                .resolve_document(session.workspace_id, "note.md")
                .unwrap(),
            document
        );
        assert!(tabs.rename(&note, &base));
        assert_eq!(tabs.active().document_id, None);
        session.renamed(&core, tabs.active());
        session.save(&core, &mut tabs);
        let (_, restored) = Session::open(&core, vault.path()).unwrap();
        assert!(restored.active().is_base());
        assert_eq!(restored.active().document_id, None);
        let (mut session, _) = Session::open(&core, vault.path()).unwrap();
        std::fs::rename(&base, &restored_note).unwrap();
        assert!(tabs.rename(&base, &restored_note));
        assert_eq!(tabs.active().document_id, None);
        session.renamed(&core, tabs.active());
        session.save(&core, &mut tabs);
        assert!(tabs.active().document_id.is_some());
        assert_ne!(tabs.active().document_id, Some(document));
        core.shutdown();
    }
}
