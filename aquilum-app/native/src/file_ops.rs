//! Действия с файлами из дерева — через `aquilum_core::files::gate`, как команды Tauri.
//! Выполняются в отдельном потоке (переименование переписывает ссылки по всей базе); дерево
//! обновляется событиями ядра, а не результатом вызова.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use aquilum_core::Core;
use aquilum_core::files::error::FileCommandError;
use aquilum_core::files::gate;
use aquilum_core::history::Source;

/// Сколько номеров перебирать в поисках свободного имени, как `NUMBERED_PATH_ATTEMPTS`.
const NUMBERED_PATH_ATTEMPTS: usize = 1000;

/// `name (index).ext` — `numberedPath` из `src/modules/paths.ts`.
fn numbered(path: &Path, index: usize) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    // Расширение — от последней точки, как `/\.[^.]+$/` во фронтенде.
    let (stem, extension) = match name.rfind('.') {
        Some(dot) => name.split_at(dot),
        None => (name.as_str(), ""),
    };
    path.with_file_name(format!("{stem} ({index}){extension}"))
}

/// «Дублировать»: копия рядом под первым свободным именем `name (1).md`, `name (2).md`…
fn duplicate(core: &Core, path: &Path) -> Result<PathBuf, FileCommandError> {
    for index in 1..=NUMBERED_PATH_ATTEMPTS {
        let candidate = numbered(path, index);
        match gate::copy(core, path, &candidate) {
            Ok(()) => return Ok(candidate),
            Err(FileCommandError::AlreadyExists { .. }) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(FileCommandError::AlreadyExists { path: path.to_string_lossy().into_owned() })
}

pub fn duplicate_all(core: &Arc<Core>, targets: Vec<PathBuf>) {
    let core = Arc::clone(core);
    std::thread::spawn(move || {
        for target in targets {
            if let Err(error) = duplicate(&core, &target) {
                eprintln!("{} не продублирован: {error}", target.display());
            }
        }
    });
}

/// Имя файла заметки — `sanitizeFileName` из `documentFactory.ts`: запрещённые символы и
/// управляющие — в пробелы, пробелы схлопываются, точки и пробелы в конце убираются, не длиннее
/// 120 знаков, к зарезервированным именам Windows добавляется « note».
pub fn sanitize_file_name(value: &str) -> String {
    let replaced: String = value
        .chars()
        .map(|c| if r#"<>:"/\|?*"#.contains(c) || u32::from(c) < 0x20 { ' ' } else { c })
        .collect();
    let collapsed = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim_end_matches(['.', ' ']);
    let limited: String = trimmed.chars().take(120).collect::<String>().trim().to_owned();
    // `/^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i`.
    let lower = limited.to_lowercase();
    let reserved = ["con", "prn", "aux", "nul"].contains(&lower.as_str())
        || (lower.chars().count() == 4
            && (lower.starts_with("com") || lower.starts_with("lpt"))
            && lower.chars().last().is_some_and(|c| ('1'..='9').contains(&c)));
    if reserved { format!("{limited} note") } else { limited }
}

pub fn rename_target(from: &Path, input: &str) -> Option<(PathBuf, String)> {
    let note = from.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"));
    let current = if note { from.file_stem() } else { from.file_name() }.map(|n| n.to_string_lossy().into_owned());
    let name = if note { sanitize_file_name(input.trim()) } else { input.trim().to_owned() };
    if name.is_empty() || Some(&name) == current.as_ref() {
        return None;
    }
    let to = from.with_file_name(if note { format!("{name}.md") } else { name.clone() });
    Some((to, name))
}

/// «Переименовать»: `gate::rename` переписывает ссылки на заметку по всей базе.
pub fn rename(core: &Arc<Core>, from: PathBuf, to: PathBuf) {
    let core = Arc::clone(core);
    std::thread::spawn(move || {
        if let Err(error) = gate::rename(&core, &from, &to) {
            eprintln!("{} не переименован: {error}", from.display());
        }
    });
}

/// Перенос перетаскиванием — `moveIntoFolder`: `target/имя`, при конфликте `имя (1)`, `имя (2)`…
pub fn move_all(core: &Arc<Core>, sources: Vec<PathBuf>, target: PathBuf) {
    let core = Arc::clone(core);
    std::thread::spawn(move || {
        for source in sources {
            let Some(name) = source.file_name() else { continue };
            let destination = target.join(name);
            let moved = (0..NUMBERED_PATH_ATTEMPTS).find_map(|index| {
                let candidate = if index == 0 { destination.clone() } else { numbered(&destination, index) };
                match gate::rename(&core, &source, &candidate) {
                    Ok(_) => Some(Ok(())),
                    Err(FileCommandError::AlreadyExists { .. }) => None,
                    Err(error) => Some(Err(error)),
                }
            });
            if let Some(Err(error)) = moved {
                eprintln!("{} не перенесён: {error}", source.display());
            }
        }
    });
}

/// Имя новой заметки, как `UNTITLED_NOTE` во фронтенде (не переводится).
const UNTITLED_NOTE: &str = "Без названия";

/// «Создать заметку» — `createUniqueFile`: `Без названия.md`, `Без названия 1.md`… в корне базы.
/// Синхронно: заметку сразу открывают, а пустой файл создаётся быстро.
pub fn create_note(core: &Core, workspace: &Path, title: Option<&str>) -> Result<PathBuf, FileCommandError> {
    let sanitized = title.map(sanitize_file_name).filter(|t| !t.is_empty());
    let base = sanitized.as_deref().unwrap_or(UNTITLED_NOTE);
    for attempt in 0..NUMBERED_PATH_ATTEMPTS {
        let name = if attempt == 0 { base.to_owned() } else { format!("{base} {attempt}") };
        let path = workspace.join(format!("{name}.md"));
        match gate::create(core, &path, "", Source::Me, None) {
            Ok(_) => return Ok(path),
            Err(FileCommandError::AlreadyExists { .. }) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(FileCommandError::AlreadyExists { path: workspace.join(UNTITLED_NOTE).to_string_lossy().into_owned() })
}

/// «Удалить»: по очереди в корзину базы знаний.
pub fn trash_all(core: &Arc<Core>, workspace: PathBuf, targets: Vec<PathBuf>) {
    let core = Arc::clone(core);
    std::thread::spawn(move || {
        for target in targets {
            if let Err(error) = gate::trash(&core, &workspace, &target) {
                eprintln!("{} не удалён: {error}", target.display());
            }
        }
    });
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

    #[test]
    fn rename_target_keeps_card_in_its_folder_and_preserves_markdown_extension() {
        assert_eq!(
            rename_target(Path::new("/база/папка/Старая.md"), "Новая: карточка?"),
            Some((
                PathBuf::from("/база/папка/Новая карточка.md"),
                "Новая карточка".to_owned()
            ))
        );
    }
}
