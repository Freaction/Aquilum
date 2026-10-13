use super::batch::{WatchBatch, WatchScope};
use crate::search::paths::{is_hidden, is_markdown, is_plain_text};
use notify::event::{ModifyKind, RemoveKind};
use notify::{Event, EventKind};
use std::path::Path;

pub fn classify(result: notify::Result<Event>, root: &Path) -> Option<WatchBatch> {
    let Ok(event) = result else {
        return Some(WatchBatch::rescan());
    };
    if matches!(event.kind, EventKind::Access(_)) {
        return None;
    }
    if needs_directory_rescan(&event) {
        return Some(WatchBatch::new(event.paths, WatchScope::Rescan));
    }

    let scope = if is_structural(&event.kind) {
        WatchScope::Structure
    } else {
        WatchScope::Content
    };
    let notes = event
        .paths
        .iter()
        .filter(|path| is_markdown(path) || is_plain_text(path))
        .cloned()
        .collect::<Vec<_>>();
    if !notes.is_empty() {
        return Some(WatchBatch::new(notes, scope));
    }
    if scope != WatchScope::Structure {
        return None;
    }
    let listed = event
        .paths
        .into_iter()
        .filter(|path| !is_hidden(root, path))
        .collect::<Vec<_>>();
    (!listed.is_empty()).then(|| WatchBatch::new(listed, WatchScope::Listing))
}

fn is_structural(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_))
    )
}

fn needs_directory_rescan(event: &Event) -> bool {
    match event.kind {
        EventKind::Remove(RemoveKind::Folder) => true,
        EventKind::Remove(_) => event
            .paths
            .iter()
            .any(|path| !is_markdown(path) && path.extension().is_none()),
        EventKind::Modify(ModifyKind::Name(_)) => event.paths.iter().any(|path| path.is_dir()),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, DataChange};
    use std::path::PathBuf;

    fn event(kind: EventKind, paths: &[&str]) -> notify::Result<Event> {
        Ok(Event {
            kind,
            paths: paths.iter().map(PathBuf::from).collect(),
            attrs: Default::default(),
        })
    }

    fn written(paths: &[&str]) -> notify::Result<Event> {
        event(
            EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths,
        )
    }

    #[test]
    fn ignores_reads() {
        let read = EventKind::Access(notify::event::AccessKind::Read);
        assert!(classify(event(read, &["a.md"]), Path::new("")).is_none());
    }

    #[test]
    fn ignores_files_that_are_not_notes() {
        assert!(classify(written(&["cover.png"]), Path::new("")).is_none());
    }

    #[test]
    fn a_write_touches_content_only() {
        let batch = classify(written(&["Заметка.md"]), Path::new("")).unwrap();
        assert_eq!(batch.scope, WatchScope::Content);
        assert_eq!(batch.paths.len(), 1);
    }

    #[test]
    fn creating_and_removing_a_note_is_structural() {
        let created = classify(event(EventKind::Create(CreateKind::File), &["Новая.md"]), Path::new("")).unwrap();
        assert_eq!(created.scope, WatchScope::Structure);
    }

    #[test]
    fn a_new_folder_or_attachment_only_changes_the_listing() {
        let root = Path::new("/база");
        let folder = classify(event(EventKind::Create(CreateKind::Folder), &["/база/Проекты"]), root).unwrap();
        assert_eq!(folder.scope, WatchScope::Listing);
        let image = classify(event(EventKind::Create(CreateKind::File), &["/база/картинка.png"]), root).unwrap();
        assert_eq!(image.scope, WatchScope::Listing);
        assert!(classify(event(EventKind::Create(CreateKind::File), &["/база/.aquilum/история"]), root).is_none());
        assert!(classify(written(&["/база/картинка.png"]), root).is_none());
    }

    #[test]
    fn a_text_file_is_a_note_for_the_watcher() {
        let batch = classify(written(&["/база/заметка.txt"]), Path::new("/база")).unwrap();
        assert_eq!(batch.scope, WatchScope::Content);
        let created = classify(event(EventKind::Create(CreateKind::File), &["/база/заметка.txt"]), Path::new("/база")).unwrap();
        assert_eq!(created.scope, WatchScope::Structure);
    }

    #[test]
    fn a_removed_folder_asks_for_a_full_rescan() {
        let batch = classify(event(EventKind::Remove(RemoveKind::Folder), &["Проекты"]), Path::new("")).unwrap();
        assert_eq!(batch.scope, WatchScope::Rescan);
    }

    #[test]
    fn a_watcher_error_asks_for_a_rescan_instead_of_being_dropped() {
        let batch = classify(Err(notify::Error::generic("очередь событий переполнена")), Path::new("")).unwrap();
        assert_eq!(batch.scope, WatchScope::Rescan);
    }

    #[test]
    fn an_atomic_write_rename_stays_a_note_event_and_does_not_trigger_a_rescan() {
        let batch = classify(
            event(EventKind::Modify(ModifyKind::Name(notify::event::RenameMode::Both)), &["Заметка.md.tmp", "Заметка.md"]),
            Path::new(""),
        )
        .unwrap();
        assert_eq!(batch.scope, WatchScope::Structure);
        assert_eq!(batch.paths, vec![PathBuf::from("Заметка.md")]);
    }
}
