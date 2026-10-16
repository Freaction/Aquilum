use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchIndexState {
    Idle,
    Indexing,
    Ready,
    Error,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexStatus {
    pub state: SearchIndexState,
    pub updating: bool,
    pub generation: u64,
    pub revision: u64,
    pub indexed_documents: u64,
    pub scanned_documents: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SearchIndexStatus {
    pub fn idle() -> Self {
        Self {
            state: SearchIndexState::Idle,
            updating: false,
            generation: 0,
            revision: 0,
            indexed_documents: 0,
            scanned_documents: 0,
            error: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexRevision {
    pub workspace_path: String,
    pub generation: u64,
    pub revision: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub path: String,
    pub title: String,
    pub extension: String,
    pub snippet: String,
    pub match_count: usize,
    pub match_offset: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<String>,
    pub matched_terms: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub query_terms: Vec<String>,
    pub results: Vec<SearchResult>,
    pub has_more: bool,
    pub status: SearchIndexStatus,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backlink {
    pub path: String,
    pub title: String,
    pub offset: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingLink {
    pub target: String,
    pub title: String,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiLinkResolution {
    pub paths: Vec<Option<String>>,
    pub complete: bool,
}

impl SearchResponse {
    pub fn idle() -> Self {
        Self {
            query_terms: Vec::new(),
            results: Vec::new(),
            has_more: false,
            status: SearchIndexStatus::idle(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSuggestion {
    pub path: String,
    pub title: String,
}
