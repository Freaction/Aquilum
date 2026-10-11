use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct NoteTemplate {
    pub name: String,
    pub path: PathBuf,
    pub relative: String,
}

pub(super) fn scan_dir(base: &Path, current: &Path, list: &mut Vec<NoteTemplate>) {
    let Ok(entries) = std::fs::read_dir(current) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(base, &path, list);
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")) {
            let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            let relative = path.strip_prefix(base).map_or_else(|_| name.clone(), |p| p.to_string_lossy().into_owned());
            list.push(NoteTemplate { name, path, relative });
        }
    }
}
