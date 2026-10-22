use std::path::{Path, PathBuf};
use std::sync::Arc;

use aquilum_core::Core;
pub use aquilum_core::files::entries::first_free;
use aquilum_core::files::error::FileCommandError;
use aquilum_core::files::gate;
use aquilum_core::history::Source;
use aquilum_core::search::paths::is_markdown;

const UNTITLED_NOTE: &str = "Без названия";

fn numbered(path: &Path, index: usize) -> PathBuf {
    if index == 0 {
        return path.to_path_buf();
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let (stem, extension) = match name.rfind('.') {
        Some(dot) => name.split_at(dot),
        None => (name.as_str(), ""),
    };
    path.with_file_name(format!("{stem} ({index}){extension}"))
}

fn background(core: &Arc<Core>, targets: Vec<PathBuf>, failure: &'static str, op: impl Fn(&Core, &Path) -> Result<(), FileCommandError> + Send + 'static) {
    let core = Arc::clone(core);
    crate::disk::run(move || {
        for target in targets {
            if let Err(error) = op(&core, &target) {
                report!("{} {failure}: {error}", target.display());
            }
        }
    });
}

pub fn duplicate_all(core: &Arc<Core>, targets: Vec<PathBuf>) {
    background(core, targets, "не продублирован", |core, path| first_free(|i| numbered(path, i + 1), |copy| gate::copy(core, path, copy)).map(drop));
}

pub fn sanitize_file_name(value: &str) -> String {
    let replaced: String = value
        .chars()
        .map(|c| if r#"<>:"/\|?*"#.contains(c) || u32::from(c) < 0x20 { ' ' } else { c })
        .collect();
    let collapsed = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim_end_matches(['.', ' ']);
    let limited: String = trimmed.chars().take(120).collect::<String>().trim().to_owned();
    let lower = limited.to_lowercase();
    let reserved = ["con", "prn", "aux", "nul"].contains(&lower.as_str())
        || (lower.chars().count() == 4
            && (lower.starts_with("com") || lower.starts_with("lpt"))
            && lower.chars().last().is_some_and(|c| ('1'..='9').contains(&c)));
    if reserved { format!("{limited} note") } else { limited }
}

pub fn title_target(from: &Path, input: &str) -> Option<(PathBuf, String)> {
    if is_markdown(from) {
        return rename_target(from, input);
    }
    let extension = from.extension()?.to_string_lossy().into_owned();
    rename_target(from, &format!("{}.{extension}", sanitize_file_name(input.trim())))
}

pub fn rename_target(from: &Path, input: &str) -> Option<(PathBuf, String)> {
    let note = is_markdown(from);
    let current = if note { from.file_stem() } else { from.file_name() }.map(|n| n.to_string_lossy().into_owned());
    let name = if note { sanitize_file_name(input.trim()) } else { input.trim().to_owned() };
    if name.is_empty() || Some(&name) == current.as_ref() {
        return None;
    }
    let to = from.with_file_name(if note { format!("{name}.md") } else { name.clone() });
    Some((to, name))
}

pub fn rename(core: &Arc<Core>, from: PathBuf, to: PathBuf, failed: impl FnOnce() + Send + 'static) {
    let core = Arc::clone(core);
    crate::disk::run(move || {
        if let Err(error) = gate::rename(&core, &from, &to) {
            report!("{} не переименован: {error}", from.display());
            failed();
        }
    });
}

pub fn move_all(core: &Arc<Core>, sources: Vec<PathBuf>, target: PathBuf) {
    background(core, sources, "не перенесён", move |core, source| {
        let Some(name) = source.file_name() else { return Ok(()) };
        let destination = target.join(name);
        first_free(|i| numbered(&destination, i), |candidate| gate::rename(core, source, candidate)).map(drop)
    });
}

pub fn create_note(core: &Core, folder: &Path, title: Option<&str>) -> Result<PathBuf, FileCommandError> {
    std::fs::create_dir_all(folder)?;
    let sanitized = title.map(sanitize_file_name).filter(|t| !t.is_empty());
    let base = sanitized.as_deref().unwrap_or(UNTITLED_NOTE);
    first_free(|i| folder.join(if i == 0 { format!("{base}.md") } else { format!("{base} {i}.md") }), |path| gate::create(core, path, &"", Source::Me, None))
}

pub fn create_folder(parent: &Path) -> Result<PathBuf, FileCommandError> {
    first_free(
        |i| parent.join(if i == 0 { UNTITLED_NOTE.to_owned() } else { format!("{UNTITLED_NOTE} {i}") }),
        |path| match std::fs::create_dir(path) {
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Err(FileCommandError::AlreadyExists { path: path.to_string_lossy().into_owned() }),
            result => result.map_err(FileCommandError::from),
        },
    )
}

pub fn trash_all(core: &Arc<Core>, workspace: PathBuf, targets: Vec<PathBuf>) {
    background(core, targets, "не удалён", move |core, target| gate::trash(core, &workspace, target).map(drop));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitized_names() {
        assert_eq!(sanitize_file_name("  Новая:  заметка?. "), "Новая заметка");
        assert_eq!(sanitize_file_name("con"), "con note");
        assert_eq!(sanitize_file_name("COM3"), "COM3 note");
        assert_eq!(sanitize_file_name("com0"), "com0");
        assert_eq!(sanitize_file_name("ёё"), "ёё");
        assert_eq!(sanitize_file_name("a\tb"), "a b");
    }

    #[test]
    fn numbered_names() {
        assert_eq!(numbered(Path::new("/база/Заметка.md"), 1), Path::new("/база/Заметка (1).md"));
        assert_eq!(numbered(Path::new("/база/папка"), 2), Path::new("/база/папка (2)"));
        assert_eq!(numbered(Path::new("/база/.скрытый"), 1), Path::new("/база/ (1).скрытый"));
    }
}
