use super::SNIPPET_LIMIT;
use crate::app_core::Core;
use crate::mcp::tools::workspace::{indexed_vault, requested_vault};
use crate::mcp::tools::{limit, optional_text, shorten, text};
use crate::search::query::Request;
use serde_json::{json, Value};
use std::path::Path;

pub fn search_notes(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = indexed_vault(core, arguments)?;
    let folder = optional_text(arguments, "folder").map(|_| vault.folder_argument(arguments)).transpose()?;
    let query = text(arguments, "query")?;
    let request = Request {
        folder: folder.as_deref(),
        limit: Some(limit(arguments, 10, 50)),
        ..Request::new(&query)
    };
    let response = core
        .search
        .search(&vault.root.to_string_lossy(), &request)
        .map_err(|error| error.to_string())?;

    let shown = &response.results;
    let counts = core
        .search
        .backlink_counts(
            &vault.root.to_string_lossy(),
            &shown.iter().map(|result| result.path.clone()).collect::<Vec<_>>(),
        )
        .ok();
    let results = shown
        .iter()
        .enumerate()
        .map(|(position, result)| {
            json!({
                "path": vault.relative(Path::new(&result.path)),
                "heading": result.heading,
                "snippet": shorten(&result.snippet, SNIPPET_LIMIT),
                "matches": result.match_count,
                "matchedTerms": result.matched_terms,
                "backlinks": counts.as_ref().and_then(|counts| counts.get(position)),
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "indexing": response.status.updating,
        "terms": response.query_terms,
        "results": results,
        "hasMore": response.has_more,
    }))
}

pub fn dataview_syntax(arguments: &Value) -> Result<Value, String> {
    let Some(query) = optional_text(arguments, "query") else {
        return Ok(json!({ "syntax": crate::search::dataview::syntax_help() }));
    };
    match crate::search::dataview::check_query(&query) {
        Ok(()) => Ok(json!({ "valid": true, "query": query })),
        Err(message) => Ok(json!({ "valid": false, "error": message })),
    }
}

pub fn index_status(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = requested_vault(core, arguments)?;
    let status = core
        .search
        .status(Some(&vault.root.to_string_lossy()));
    serde_json::to_value(status).map_err(|error| error.to_string())
}
