use std::path::{Path, PathBuf};

use aquilum_core::Core;
use aquilum_core::ui_state::models::{OpenSessionInput, SaveStateBatchInput, SessionStateInput, TabKind, TabState, ViewStateInput};
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
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or_default()
}

pub fn relative(root: &Path, path: &Path) -> Option<String> {
    let rest = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rest.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

impl Session {
    pub fn open(core: &Core, root: &Path) -> Option<(Session, Tabs)> {
        let workspace_id = core.ui_state.resolve_workspace(&root.to_string_lossy(), now_ms()).ok()?;
        let epoch = Uuid::new_v4();
        let input = OpenSessionInput { workspace_id, window_id: WINDOW_ID.into(), epoch, now_ms: now_ms() };
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
                TabKind::Empty => Some(Tab { id: tab.tab_id, path: None, document_id: None, graph: false }),
                TabKind::Document => {
                    let path = root.join(tab.relative_path?.replace('/', std::path::MAIN_SEPARATOR_STR));
                    path.is_file().then_some(Tab { id: tab.tab_id, path: Some(path), document_id: tab.document_id, graph: false })
                }
                TabKind::Graph => Some(Tab { id: tab.tab_id, path: None, document_id: None, graph: true }),
            })
            .collect();
        let camera = loaded.graph_camera.map(|c| (c.center_x, c.center_y, c.scale));
        let session = Session { root: root.to_path_buf(), workspace_id, epoch, sequence: 0, camera };
        Some((session, Tabs::restore(tabs, loaded.active_tab_id)))
    }

    pub fn save(&mut self, core: &Core, tabs: &mut Tabs) {
        let ids: Vec<Uuid> = tabs.tabs().iter().map(|t| t.id).collect();
        for id in ids {
            let Some(tab) = tabs.tab_mut(id) else { continue };
            if tab.document_id.is_some() {
                continue;
            }
            let Some(relative) = tab.path.as_deref().and_then(|p| relative(&self.root, p)) else { continue };
            match core.ui_state.resolve_document(self.workspace_id, &relative) {
                Ok(document) => tab.document_id = Some(document),
                Err(error) => eprintln!("заметка {relative} не записана в сессию: {error:?}"),
            }
        }
        let stored = tabs
            .tabs()
            .iter()
            .enumerate()
            .filter_map(|(position, tab)| {
                let kind = if tab.graph {
                    TabKind::Graph
                } else if tab.path.is_some() {
                    TabKind::Document
                } else {
                    TabKind::Empty
                };
                if kind == TabKind::Document && tab.document_id.is_none() {
                    return None;
                }
                Some(TabState { tab_id: tab.id, document_id: tab.document_id, kind, position: position as i64 })
            })
            .collect();
        self.sequence += 1;
        let batch = SaveStateBatchInput {
            workspace_id: self.workspace_id,
            window_id: WINDOW_ID.into(),
            epoch: self.epoch,
            sequence: self.sequence,
            now_ms: now_ms(),
            session: Some(SessionStateInput { active_tab_id: Some(tabs.active_id()), tabs: Some(stored) }),
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
            graph_camera: Some(aquilum_core::ui_state::models::GraphCameraState { center_x: camera.0, center_y: camera.1, scale: camera.2 }),
        };
        if let Err(error) = core.ui_state.save_batch(&batch) {
            eprintln!("камера графа не сохранена: {error:?}");
        }
    }

    pub fn document(&self, core: &Core, path: &Path) -> Option<Uuid> {
        let relative = relative(&self.root, path)?;
        core.ui_state.resolve_document(self.workspace_id, &relative).ok()
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
        let view = core.ui_state.load_view(self.workspace_id, WINDOW_ID, document, PANE_ID).ok()??;
        Some((view.fallback_scroll_anchor, view.scroll_offset_px))
    }

    pub fn renamed(&self, core: &Core, tab: &Tab) {
        let (Some(document), Some(path)) = (tab.document_id, tab.path.as_deref()) else { return };
        let Some(relative) = relative(&self.root, path) else { return };
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
        assert_eq!(paths, [Some(vault.path().join("Папка").join("a.md")), Some(vault.path().join("b.md")), None, None]);
        assert!(restored.tabs()[3].graph);
        assert_eq!(reopened.camera, Some((1.5, -2.0, 30.0)));
        assert_eq!(restored.active_id(), active);
        core.shutdown();
    }
}
