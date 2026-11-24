use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikixivHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
    #[serde(default)]
    pub from_filename: bool,
    #[serde(default)]
    pub matched_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikixivSearchResult {
    pub hits: Vec<WikixivHit>,
    pub generation: u64,
    pub offline: bool,
    pub insufficient_text: bool,
}

#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub text: String,
    pub document_path: Option<String>,
    pub generation: u64,
}
