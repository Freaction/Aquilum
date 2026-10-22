use super::bm25f;
use super::graph::GraphSnapshot;
use super::models::{AnalysisMethod, AnalysisResult};
use crate::search::error::SearchError;
use crate::search::models::SearchIndexState;
use crate::search::service::{CachedGraph, OpenIndex, SearchService};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;

use crate::settings::models::AppConfig;

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 100;

impl SearchService {
    pub fn analyze_document(
        &self,
        workspace: &str,
        document: &str,
        method: AnalysisMethod,
        limit: Option<usize>,
        config: &AppConfig,
    ) -> Result<Vec<AnalysisResult>, SearchError> {
        self.with_index(workspace, |open| {
            let status = open.progress.snapshot();
            if status.updating {
                return Err(SearchError::Unavailable {
                    message: "Search index is still updating".to_owned(),
                });
            }
            if status.state == SearchIndexState::Error {
                return Err(SearchError::Unavailable {
                    message: status
                        .error
                        .unwrap_or_else(|| "Search index is unavailable".to_owned()),
                });
            }
            let document = crate::search::paths::inside(&open.root, document)?;
            let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
            match method {
                AnalysisMethod::Bm25f if !config.analysis.enable_bm25f => {
                    Err(SearchError::Unavailable {
                        message: "BM25F analysis is disabled".to_owned(),
                    })
                }
                AnalysisMethod::AdamicAdar if !config.analysis.enable_adamic_adar => {
                    Err(SearchError::Unavailable {
                        message: "Adamic-Adar analysis is disabled".to_owned(),
                    })
                }
                AnalysisMethod::Bm25f => bm25f::analyze(
                    &open.index,
                    &document,
                    limit,
                    &config.analysis.bm25f_params,
                    &config.search,
                ),
                AnalysisMethod::AdamicAdar => Ok(self.link_graph(open, status.revision)?.adamic_adar(&document, limit)),
            }
        })
    }

    pub fn incoming_links(
        &self,
        workspace: &str,
        documents: &[PathBuf],
    ) -> Result<Vec<usize>, SearchError> {
        self.with_index(workspace, |open| Ok(self.link_graph(open, open.progress.snapshot().revision)?.incoming_counts(documents)))
    }

    fn link_graph(&self, open: &OpenIndex, revision: u64) -> Result<Arc<GraphSnapshot>, SearchError> {
        let load = || Ok::<_, SearchError>(Arc::new(GraphSnapshot::load(&Connection::open(&open.metadata_path)?, &open.root)?));
        if !open.is_visible() {
            return load();
        }
        let fresh = |cached: &CachedGraph| cached.root == open.root && cached.generation == open.generation && cached.revision == revision;
        if let Some(cached) = self.analysis_graph.read().map_err(SearchError::task)?.as_ref().filter(|cached| fresh(cached)) {
            return Ok(Arc::clone(&cached.graph));
        }
        let loaded = load()?;
        let mut cache = self.analysis_graph.write().map_err(SearchError::task)?;
        if let Some(cached) = cache.as_ref().filter(|cached| fresh(cached)) {
            return Ok(Arc::clone(&cached.graph));
        }
        *cache = Some(CachedGraph { root: open.root.clone(), generation: open.generation, revision, graph: Arc::clone(&loaded) });
        Ok(loaded)
    }
}
