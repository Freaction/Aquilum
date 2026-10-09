use super::attachments::forget_attachments_if_touched;
use super::deletions::deletion_files;
use super::document::{
    copy_file_impl, create_binary_file_impl, create_file_impl, rename_file_impl,
    write_file_atomic_impl,
};
use super::error::FileCommandError;
use super::models::{FileRenameResult, FileWriteResult};
use super::rename::rename_with_links;
use super::trash::{move_to_trash_impl, restore_from_trash_impl};
use crate::history::{NoteWrite, Source};
use crate::search::paths::is_markdown;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use crate::app_core::{Core, CoreEvent};
use walkdir::WalkDir;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Relocation {
    pub moves: Vec<NoteMove>,
    pub removed: Vec<String>,
}

#[derive(Clone, Serialize)]
pub struct NoteHistoryChanged {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NoteMove {
    pub from: String,
    pub to: String,
}

pub fn write(
    core: &Core,
    path: &Path,
    content: &str,
    expected_hash: Option<&str>,
    source: Source,
    version_name: Option<&str>,
) -> Result<FileWriteResult, FileCommandError> {
    let history = &core.history;
    let before = history.disk_before(path, expected_hash);
    let result = write_file_atomic_impl(path, content, expected_hash)?;
    let write = NoteWrite { disk_before: before, text: content, hash: &result.hash, source, name: version_name };
    if history.saved(&|| core.known_roots(), path, write) {
        announce_history(core, path);
    }
    core.search.notify_content_path(path.to_path_buf());
    Ok(result)
}

pub fn create(
    core: &Core,
    path: &Path,
    content: &str,
    source: Source,
    version_name: Option<&str>,
) -> Result<FileWriteResult, FileCommandError> {
    let result = create_file_impl(path, content)?;
    if is_markdown(path) {
        let write = NoteWrite {
            disk_before: Some((String::new(), SystemTime::now())),
            text: content,
            hash: &result.hash,
            source,
            name: version_name,
        };
        if core.history.saved(&|| core.known_roots(), path, write) {
            announce_history(core, path);
        }
    }
    paths_changed(core, vec![path.to_path_buf()]);
    Ok(result)
}

pub fn create_binary(
    core: &Core,
    path: &Path,
    bytes: &[u8],
) -> Result<FileWriteResult, FileCommandError> {
    let result = create_binary_file_impl(path, bytes)?;
    paths_changed(core, vec![path.to_path_buf()]);
    Ok(result)
}

pub fn copy(core: &Core, from: &Path, to: &Path) -> Result<(), FileCommandError> {
    copy_file_impl(from, to)?;
    paths_changed(core, vec![to.to_path_buf()]);
    Ok(())
}

pub fn rename(
    core: &Core,
    from: &Path,
    to: &Path,
) -> Result<FileRenameResult, FileCommandError> {
    core.documents.moving(core, from, || rename_now(core, from, to))
}

fn rename_now(core: &Core, from: &Path, to: &Path) -> Result<FileRenameResult, FileCommandError> {
    let notes = notes_under(from);
    let renamed = rename_with_links(&core.search, from, to)?;
    relocate_plugin_data(core, from, to);
    let relocation = relocated(&notes, from, to);
    follow_moves(core, &relocation);
    let history = &core.history;
    for rewrite in &renamed.rewritten {
        let write = NoteWrite {
            disk_before: Some((rewrite.before.clone(), SystemTime::now())),
            text: &rewrite.after,
            hash: &rewrite.hash,
            source: Source::Links,
            name: None,
        };
        if history.saved(&|| core.known_roots(), &rewrite.path, write) {
            announce_history(core, &rewrite.path);
        }
    }
    let mut changed = vec![from.to_path_buf(), to.to_path_buf()];
    changed.extend(renamed.result.updated_paths.iter().map(PathBuf::from));
    let rewritten: Vec<PathBuf> = renamed.result.updated_paths.iter().map(PathBuf::from).collect();
    paths_changed(core, changed);
    announce(core, relocation);
    core.documents.reconcile_paths(core, &rewritten);
    Ok(renamed.result)
}

pub fn move_across(core: &Core, from: &Path, to: &Path) -> Result<(), FileCommandError> {
    core.documents.moving(core, from, || move_across_now(core, from, to))
}

fn move_across_now(core: &Core, from: &Path, to: &Path) -> Result<(), FileCommandError> {
    let notes = notes_under(from);
    rename_file_impl(from, to)?;
    relocate_plugin_data(core, from, to);
    let relocation = relocated(&notes, from, to);
    follow_moves(core, &relocation);
    paths_changed(core, vec![from.to_path_buf(), to.to_path_buf()]);
    announce(core, relocation);
    Ok(())
}

pub fn trash(
    core: &Core,
    workspace: &Path,
    path: &Path,
) -> Result<PathBuf, FileCommandError> {
    core.documents.moving(core, path, || trash_now(core, workspace, path))
}

fn trash_now(core: &Core, workspace: &Path, path: &Path) -> Result<PathBuf, FileCommandError> {
    let moved = move_to_trash_impl(workspace, path)?;
    remove_plugin_data(workspace, path);
    let notes = markdown_moves(&moved.files);
    let history = &core.history;
    for (note, trashed_at) in &notes {
        history.trashed(&|| core.known_roots(), note, trashed_at);
    }
    paths_changed(core, vec![path.to_path_buf(), moved.root.clone()]);
    announce(core, removed(notes.iter().map(|(note, _)| note)));
    Ok(moved.root)
}

pub fn restore(
    core: &Core,
    workspace: &Path,
    trashed: &Path,
) -> Result<PathBuf, FileCommandError> {
    let mut roots = restore_all(core, workspace, &[trashed.to_path_buf()])?;
    Ok(roots.remove(0))
}

pub fn restore_deletion(
    core: &Core,
    workspace: &Path,
    id: &str,
) -> Result<(), FileCommandError> {
    restore_all(core, workspace, &deletion_files(workspace, id)).map(|_| ())
}

fn restore_all(
    core: &Core,
    workspace: &Path,
    trashed: &[PathBuf],
) -> Result<Vec<PathBuf>, FileCommandError> {
    let history = &core.history;
    let mut roots = Vec::new();
    let mut failure = None;
    for path in trashed {
        match restore_from_trash_impl(workspace, path) {
            Ok(restored) => {
                for (trashed_note, back) in markdown_moves(&restored.files) {
                    history.restored(&|| core.known_roots(), &trashed_note, &back);
                }
                roots.push(restored.root);
            }
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    paths_changed(core, trashed.iter().chain(&roots).cloned().collect());
    failure.map_or(Ok(roots), Err)
}

pub fn name_version(
    core: &Core,
    path: &Path,
    version: Option<&str>,
    name: &str,
) -> Option<String> {
    let named = core.history.name(&|| core.known_roots(), path, version, name);
    if named.is_some() {
        announce_history(core, path);
    }
    named
}

fn plugin_path(path: &Path) -> PathBuf {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => crate::search::paths::canonical_path(parent).join(name),
        _ => crate::search::paths::canonical_path(path),
    }
}

fn relocate_plugin_data(core: &Core, from: &Path, to: &Path) {
    use crate::search::paths::{slash_path, strip_root};
    let from = plugin_path(from);
    let to = plugin_path(to);
    // Как история: первый известный Core корень, содержащий исходный путь.
    let Some((root, from_rel)) = core.known_roots().into_iter().find_map(|root| {
        let relative = strip_root(&root, &from)?;
        Some((root.clone(), slash_path(relative)))
    }) else { return; };
    let result = match strip_root(&root, &to) {
        Some(relative) => crate::plugins::vault_data::relocate(&root, &from_rel, &slash_path(relative)),
        None => Err(crate::plugins::error::PluginError::Invalid {
            message: "перенос данных между хранилищами не определён контрактом relocate".to_owned(),
        }),
    };
    if let Err(error) = result {
        eprintln!("[aquilum:plugins] не удалось перенести данные {}: {error}", from.display());
    }
}

fn remove_plugin_data(workspace: &Path, path: &Path) {
    use crate::search::paths::{canonical_path, slash_path, strip_root};
    let root = canonical_path(workspace);
    let path = plugin_path(path);
    if let Some(relative) = strip_root(&root, &path) {
        if let Err(error) = crate::plugins::vault_data::remove(&root, &slash_path(relative)) {
            eprintln!("[aquilum:plugins] не удалось удалить данные {}: {error}", path.display());
        }
    }
}

fn paths_changed(core: &Core, paths: Vec<PathBuf>) {
    forget_attachments_if_touched(&paths);
    core.search.notify_paths(paths);
}


fn follow_moves(core: &Core, relocation: &Relocation) {
    let history = &core.history;
    for step in &relocation.moves {
        history.moved(&|| core.known_roots(), Path::new(&step.from), Path::new(&step.to));
    }
}

fn markdown_moves(files: &[(PathBuf, PathBuf)]) -> Vec<(PathBuf, PathBuf)> {
    files
        .iter()
        .filter(|(from, _)| is_markdown(from))
        .cloned()
        .collect()
}

fn announce(core: &Core, relocation: Relocation) {
    if relocation.moves.is_empty() && relocation.removed.is_empty() {
        return;
    }
    let moves: Vec<(String, String)> =
        relocation.moves.iter().map(|step| (step.from.clone(), step.to.clone())).collect();
    core.documents.relocated(&moves, &relocation.removed);
    core.emit(CoreEvent::NotesRelocated(relocation));
}

fn announce_history(core: &Core, note: &Path) {
    let changed = NoteHistoryChanged {
        path: note.to_string_lossy().into_owned(),
    };
    core.emit(CoreEvent::NoteHistoryChanged(changed));
}

fn notes_under(path: &Path) -> Vec<PathBuf> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && is_markdown(entry.path()))
        .map(|entry| entry.into_path())
        .collect()
}

fn relocated(notes: &[PathBuf], from: &Path, to: &Path) -> Relocation {
    let moves = notes
        .iter()
        .filter_map(|note| {
            let inside = note.strip_prefix(from).ok()?;
            let target = if inside.as_os_str().is_empty() {
                to.to_path_buf()
            } else {
                to.join(inside)
            };
            Some(NoteMove {
                from: note.to_string_lossy().into_owned(),
                to: target.to_string_lossy().into_owned(),
            })
        })
        .collect();
    Relocation {
        moves,
        removed: Vec::new(),
    }
}

fn removed<'a>(notes: impl Iterator<Item = &'a PathBuf>) -> Relocation {
    Relocation {
        moves: Vec::new(),
        removed: notes.map(|note| note.to_string_lossy().into_owned()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{notes_under, relocated, removed, NoteMove};
    use std::fs;

    #[test]
    fn renaming_a_folder_moves_its_plugin_color() {
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        fs::create_dir(root.join("app-data")).unwrap();
        let core = crate::Core::open(&root.join("app-data"), std::sync::Arc::new(|_: crate::CoreEvent| {}));
        core.ui_state.resolve_workspace(root.to_str().unwrap(), 0).unwrap();
        fs::create_dir(root.join("Проекты")).unwrap();
        fs::create_dir(root.join(".aquilum")).unwrap();
        let data_path = root.join(".aquilum/plugins.json");
        fs::write(&data_path, r#"{"version":1,"colors":{"Проекты":"blue","Проекты/А.md":"red"}}"#).unwrap();
        super::rename(&core, &root.join("Проекты"), &root.join("Архив")).unwrap();
        let data: serde_json::Value = serde_json::from_str(&fs::read_to_string(data_path).unwrap()).unwrap();
        assert_eq!(data["colors"], serde_json::json!({"Архив":"blue", "Архив/А.md":"red"}));
        core.shutdown();
    }

    #[test]
    fn moving_and_trashing_update_plugin_data() {
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        fs::create_dir(root.join("app-data")).unwrap();
        let core = crate::Core::open(&root.join("app-data"), std::sync::Arc::new(|_: crate::CoreEvent| {}));
        core.ui_state.resolve_workspace(root.to_str().unwrap(), 0).unwrap();
        fs::create_dir(root.join("A")).unwrap();
        let mut data = crate::plugins::vault_data::VaultPluginData::default();
        data.colors.insert("A".to_owned(), "blue".to_owned());
        crate::plugins::vault_data::save(&root, &data).unwrap();
        super::move_across(&core, &root.join("A"), &root.join("B")).unwrap();
        assert_eq!(crate::plugins::vault_data::load(&root).unwrap().colors.get("B").unwrap(), "blue");
        super::trash(&core, &root, &root.join("B")).unwrap();
        assert!(crate::plugins::vault_data::load(&root).unwrap().colors.is_empty());
        core.shutdown();
    }

    #[test]
    fn plugin_io_failures_do_not_break_rename_move_or_trash() {
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        fs::create_dir(root.join("app-data")).unwrap();
        let core = crate::Core::open(&root.join("app-data"), std::sync::Arc::new(|_: crate::CoreEvent| {}));
        core.ui_state.resolve_workspace(root.to_str().unwrap(), 0).unwrap();
        fs::create_dir(root.join("A")).unwrap();
        fs::write(root.join(".aquilum"), "blocked").unwrap();
        super::rename(&core, &root.join("A"), &root.join("B")).unwrap();
        assert!(root.join("B").is_dir());
        super::move_across(&core, &root.join("B"), &root.join("C")).unwrap();
        assert!(root.join("C").is_dir());
        super::trash(&core, &root, &root.join("C")).unwrap();
        assert!(!root.join("C").exists());
        assert_eq!(fs::read_to_string(root.join(".aquilum")).unwrap(), "blocked");
        core.shutdown();
    }

    #[test]
    fn a_note_moves_as_itself() {
        let directory = tempfile::tempdir().unwrap();
        let note = directory.path().join("Идея.md");
        fs::write(&note, "текст").unwrap();
        let target = directory.path().join("Проекты").join("Идея.md");

        let relocation = relocated(&notes_under(&note), &note, &target);

        assert_eq!(
            relocation.moves,
            vec![NoteMove {
                from: note.to_string_lossy().into_owned(),
                to: target.to_string_lossy().into_owned(),
            }]
        );
    }

    #[test]
    fn a_folder_moves_as_every_note_inside_it() {
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join("Проекты");
        fs::create_dir_all(folder.join("Глубже")).unwrap();
        fs::write(folder.join("А.md"), "").unwrap();
        fs::write(folder.join("Глубже").join("Б.md"), "").unwrap();
        fs::write(folder.join("картинка.png"), "").unwrap();
        let target = directory.path().join("Архив");

        let mut relocation = relocated(&notes_under(&folder), &folder, &target);
        relocation.moves.sort_by(|left, right| left.from.cmp(&right.from));

        let targets = relocation
            .moves
            .iter()
            .map(|entry| entry.to.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            targets,
            vec![
                target.join("А.md").to_string_lossy().into_owned(),
                target.join("Глубже").join("Б.md").to_string_lossy().into_owned(),
            ],
            "только заметки, вложенность сохраняется"
        );
    }

    #[test]
    fn a_deleted_folder_reports_the_notes_it_held() {
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join("Старое");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("А.md"), "").unwrap();

        let relocation = removed(notes_under(&folder).iter());

        assert_eq!(relocation.removed, vec![folder.join("А.md").to_string_lossy().into_owned()]);
        assert!(relocation.moves.is_empty());
    }
}
