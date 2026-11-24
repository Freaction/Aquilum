use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalysisMethod {
    AdamicAdar,
    Bm25f,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    pub path: String,
    pub title: String,
    pub raw_score: f64,
    pub reasons: Vec<String>,
}
