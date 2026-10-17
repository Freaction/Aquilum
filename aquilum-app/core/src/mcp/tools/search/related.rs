use super::SNIPPET_LIMIT;
use crate::app_core::Core;
use crate::mcp::tools::workspace::{indexed_vault, requested_vault};
use crate::mcp::tools::{limit, shorten};
use crate::search::analysis::models::AnalysisMethod;
use crate::files::document::read_head;
use crate::wikixiv::models::SearchRequest;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static WIKI_GENERATION: AtomicU64 = AtomicU64::new(1);

pub fn analyze(core: &Core, arguments: &Value, method: AnalysisMethod) -> Result<Value, String> {
    let vault = indexed_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let config = core.settings.get_config();
    let results = core
        .search
        .analyze_document(
            &vault.root.to_string_lossy(),
            &note.to_string_lossy(),
            method,
            Some(limit(arguments, 10, 50)),
            &config,
        )
        .map_err(|error| error.to_string())?;

    let items = results
        .iter()
        .map(|result| {
            json!({
                "path": vault.relative(Path::new(&result.path)),
                "score": (result.raw_score * 100.0).round() / 100.0,
                "reasons": result.reasons,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({ "note": vault.relative(&note), "results": items }))
}

pub fn wiki_sources(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = requested_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let content = read_head(&note).map_err(|error| error.to_string())?;
    let enabled = core
        .settings
        .get_config()
        .analysis
        .enable_wikixiv;
    let result = core
        .wikixiv
        .search(
            SearchRequest {
                text: content,
                document_path: Some(note.to_string_lossy().into_owned()),
                generation: WIKI_GENERATION.fetch_add(1, Ordering::Relaxed),
            },
            enabled,
        )?;

    let hits = result
        .hits
        .iter()
        .map(|hit| {
            json!({
                "title": hit.title,
                "url": hit.url,
                "snippet": shorten(&hit.snippet, SNIPPET_LIMIT),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "note": vault.relative(&note),
        "offline": result.offline,
        "insufficientText": result.insufficient_text,
        "sources": hits,
    }))
}
