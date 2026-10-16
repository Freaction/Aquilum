use crate::search::analysis::window::read_window;
use crate::search::error::SearchError;
use std::path::Path;

pub fn load_source(document_path: &Path) -> Result<(String, String), SearchError> {
    let body = read_window(document_path)?;
    Ok((title(&document_path.to_string_lossy()), body))
}

pub fn title(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_owned()
}
