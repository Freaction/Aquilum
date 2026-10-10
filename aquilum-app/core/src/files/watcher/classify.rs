use super::batch::{WatchBatch, WatchScope};
use crate::search::paths::is_markdown;
use notify::event::{ModifyKind, RemoveKind};
use notify::{Event, EventKind};

pub fn classify(result: notify::Result<Event>) -> Option<WatchBatch> {
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
    let paths = event
        .paths
        .into_iter()
        .filter(|path| is_watched_file(path))
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return None;
    }
    Some(WatchBatch::new(paths, scope))
}

fn is_watched_file(path: &std::path::Path) -> bool {
    is_markdown(path)
        || path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("base"))
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
        assert!(classify(event(read, &["a.md"])).is_none());
    }

    #[test]
    fn ignores_files_that_are_not_notes() {
        assert!(classify(written(&["cover.png"])).is_none());
    }

    #[test]
    fn a_write_touches_content_only() {
        let batch = classify(written(&["Заметка.md"])).unwrap();
        assert_eq!(batch.scope, WatchScope::Content);
        assert_eq!(batch.paths.len(), 1);
    }

    #[test]
    fn creating_and_removing_a_note_is_structural() {
        let created = classify(event(EventKind::Create(CreateKind::File), &["Новая.md"])).unwrap();
        assert_eq!(created.scope, WatchScope::Structure);
    }

    #[test]
    fn a_removed_folder_asks_for_a_full_rescan() {
        let batch = classify(event(EventKind::Remove(RemoveKind::Folder), &["Проекты"])).unwrap();
        assert_eq!(batch.scope, WatchScope::Rescan);
    }

    #[test]
    fn a_watcher_error_asks_for_a_rescan_instead_of_being_dropped() {
        let batch = classify(Err(notify::Error::generic("очередь событий переполнена"))).unwrap();
        assert_eq!(batch.scope, WatchScope::Rescan);
    }

    #[test]
    fn an_atomic_write_rename_stays_a_note_event_and_does_not_trigger_a_rescan() {
        let batch = classify(event(
            EventKind::Modify(ModifyKind::Name(notify::event::RenameMode::Both)),
            &["Заметка.md.tmp", "Заметка.md"],
        ))
        .unwrap();
        assert_eq!(batch.scope, WatchScope::Structure);
        assert_eq!(batch.paths, vec![PathBuf::from("Заметка.md")]);
    }

    #[test]
    fn base_writes_and_case_insensitive_extensions_are_classified() {
        for path in ["board.base", "BOARD.BASE", "note.MD"] {
            let batch = classify(written(&[path])).unwrap();
            assert_eq!(batch.scope, WatchScope::Content);
            assert_eq!(batch.paths, vec![PathBuf::from(path)]);
        }
        let batch = classify(event(
            EventKind::Modify(ModifyKind::Name(notify::event::RenameMode::Both)),
            &["board.base.tmp", "board.BASE"],
        ))
        .unwrap();
        assert_eq!(batch.scope, WatchScope::Structure);
        assert_eq!(batch.paths, vec![PathBuf::from("board.BASE")]);
    }
}
