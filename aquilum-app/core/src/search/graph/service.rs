use super::encode::encode;
use super::snapshot::RenderSnapshot;
use crate::search::error::SearchError;
use crate::search::models::SearchIndexState;
use crate::search::paths::same_workspace;
use crate::search::service::SearchService;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

struct GraphSource {
    root: PathBuf,
    metadata_path: PathBuf,
}

impl SearchService {
    pub fn render_graph(&self, workspace: &str) -> Result<(Vec<u8>, Vec<String>), SearchError> {
        let source = self.graph_source(workspace)?;
        let _building = self.render_lock.lock().map_err(SearchError::task)?;
        let snapshot = RenderSnapshot::build(&Connection::open(&source.metadata_path)?, &source.root)?;
        let epoch = self.next_render_epoch.fetch_add(1, Ordering::Relaxed) + 1;
        Ok((encode(&snapshot, epoch), snapshot.paths))
    }

    fn graph_source(&self, workspace: &str) -> Result<GraphSource, SearchError> {
        let active = self.active.read().map_err(SearchError::task)?;
        let active = active.as_ref().ok_or_else(|| SearchError::Unavailable {
            message: "Индекс ещё не готов".to_owned(),
        })?;
        if !same_workspace(&active.root, workspace) {
            return Err(SearchError::InvalidWorkspace {
                message: "Индекс принадлежит другой базе знаний".to_owned(),
            });
        }
        let status = active.progress.snapshot();
        if status.state == SearchIndexState::Error {
            return Err(SearchError::Unavailable {
                message: status
                    .error
                    .unwrap_or_else(|| "Поисковый индекс недоступен".to_owned()),
            });
        }
        if status.updating {
            return Err(SearchError::Unavailable {
                message: "Индекс обновляется".to_owned(),
            });
        }
        Ok(GraphSource {
            root: active.root.clone(),
            metadata_path: active.metadata_path.clone(),
        })
    }
}
