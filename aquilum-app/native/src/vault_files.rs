use std::path::Path;

use aquilum_core::Core;
use aquilum_core::files::document::ensure_directory_impl;
use aquilum_core::files::gate;
use aquilum_core::files::error::FileCommandError;

use crate::file_ops::sanitize_file_name;

pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "avif", "bmp", "svg"];

const FREE_NAME_ATTEMPTS: usize = 1000;

pub fn normalize_folder(folder: &str) -> String {
    let segments: Vec<&str> = folder
        .split(['/', '\\'])
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "." && *s != ".." && !s.contains(':'))
        .collect();
    if segments.is_empty() { "Files".to_owned() } else { segments.join("/") }
}

fn split_name(name: &str, fallback_ext: &str) -> (String, String) {
    match name.rfind('.') {
        Some(dot) if dot > 0 => {
            let ext = name[dot + 1..].to_lowercase();
            (name[..dot].to_owned(), if ext.is_empty() { fallback_ext.to_owned() } else { ext })
        }
        _ => (name.to_owned(), fallback_ext.to_owned()),
    }
}

pub enum Source<'a> {
    Path(&'a Path),
    Bytes(&'a [u8], &'a str),
}

pub fn import(core: &Core, workspace: &Path, source: Source<'_>, fallback_name: &str, fallback_ext: &str) -> Result<String, FileCommandError> {
    let folder = normalize_folder(&core.settings.get_config().files.folder);
    ensure_directory_impl(&workspace.join(&folder))?;
    let original = match &source {
        Source::Path(path) => path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        Source::Bytes(_, name) => (*name).to_owned(),
    };
    let (stem, ext) = split_name(&original, fallback_ext);
    let base = match sanitize_file_name(&stem) {
        name if name.is_empty() => fallback_name.to_owned(),
        name => name,
    };
    for attempt in 0..FREE_NAME_ATTEMPTS {
        let name = if attempt == 0 { base.clone() } else { format!("{base} {attempt}") };
        let relative = format!("{folder}/{name}.{ext}");
        let target = workspace.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        let result = match &source {
            Source::Path(path) => gate::copy(core, path, &target),
            Source::Bytes(bytes, _) => gate::create_binary(core, &target, bytes).map(|_| ()),
        };
        match result {
            Ok(()) => return Ok(relative),
            Err(FileCommandError::AlreadyExists { .. }) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(FileCommandError::AlreadyExists { path: format!("{folder}/{base}.{ext}") })
}

pub fn resolve(workspace: Option<&Path>, value: &str) -> Option<std::path::PathBuf> {
    let value = value.trim();
    if value.is_empty() || value.starts_with("http") || value.starts_with("data:") || value.starts_with("blob:") {
        return None;
    }
    let value = value.strip_prefix("file:///").or_else(|| value.strip_prefix("file://")).unwrap_or(value);
    let path = Path::new(value);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    let mut out = workspace?.to_path_buf();
    for part in value.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            part => out.push(part),
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_and_names() {
        assert_eq!(normalize_folder(" a / ../b\\c: /"), "a/b");
        assert_eq!(normalize_folder(""), "Files");
        assert_eq!(split_name("Обложка.PNG", "jpg"), ("Обложка".into(), "png".into()));
        assert_eq!(split_name(".hidden", "jpg"), (".hidden".into(), "jpg".into()));
        let root = Path::new("C:/vault");
        assert_eq!(resolve(Some(root), "Files/a b.png").unwrap(), root.join("Files").join("a b.png"));
        assert_eq!(resolve(Some(root), "pattern:x").unwrap(), root.join("pattern:x"));
        assert!(resolve(Some(root), "https://x").is_none());
    }
}
