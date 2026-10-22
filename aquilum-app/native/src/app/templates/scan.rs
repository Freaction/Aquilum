use aquilum_core::search::paths::{markdown_files, relative_slash_path, title};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct NoteTemplate {
    pub name: String,
    pub path: PathBuf,
    pub relative: String,
}

impl NoteTemplate {
    pub fn title(&self) -> &str {
        self.relative.rsplit_once('.').map_or(&self.relative, |(stem, _)| stem)
    }

    pub fn extension(&self) -> &str {
        self.relative.rsplit_once('.').map_or("", |(_, extension)| extension)
    }
}

pub(super) fn scan(dir: &Path) -> Vec<NoteTemplate> {
    markdown_files(dir).map(|path| NoteTemplate { name: title(&path), relative: relative_slash_path(dir, &path), path }).collect()
}
