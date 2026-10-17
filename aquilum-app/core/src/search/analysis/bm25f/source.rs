use crate::files::document::read_head;
use crate::search::error::SearchError;
use std::path::Path;

pub fn load_source(document_path: &Path) -> Result<(String, String), SearchError> {
    let body = read_head(document_path)?;
    Ok((crate::search::paths::title(document_path), body))
}
