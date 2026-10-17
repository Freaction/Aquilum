use super::{texts, Fixture};
use crate::files::trash::{move_to_trash_impl, restore_from_trash_impl, trash_path};
use crate::history::milestone::Source;
use crate::history::service::HistoryService;
use crate::history::store::history_dir;
use std::fs;
use std::path::Path;

fn rename(history: &HistoryService, fixture: &Fixture, from: &Path, to: &Path) {
    fs::create_dir_all(to.parent().unwrap()).unwrap();
    fs::rename(from, to).unwrap();
    history.moved(&|| fixture.roots(), from, to);
}

#[test]
fn the_history_follows_a_renamed_note() {
    let fixture = Fixture::new();
    let history = fixture.service();
    let old = fixture.vault.join("Старое.md");
    let new = fixture.vault.join("Проекты").join("Новое.md");
    fixture.save(&history, &old, "текст", Source::Agent);

    rename(&history, &fixture, &old, &new);
    fixture.save(&history, &new, "текст дальше", Source::Agent);

    assert_eq!(
        texts(&fixture.history_of("Проекты/Новое.md")),
        [("текст".to_owned(), Source::Agent), ("текст дальше".to_owned(), Source::Agent)]
    );
    assert!(!fixture.history_of("Старое.md").exists());
}

#[test]
fn a_rename_that_changes_only_the_case_keeps_the_history() {
    let fixture = Fixture::new();
    let history = fixture.service();
    let old = fixture.vault.join("идея.md");
    let new = fixture.vault.join("Идея.md");
    fixture.save(&history, &old, "текст", Source::Agent);
    fixture.save(&history, &old, "текст!", Source::Agent);

    rename(&history, &fixture, &old, &new);

    assert_eq!(
        texts(&fixture.history_of("Идея.md")),
        [("текст".to_owned(), Source::Agent), ("текст!".to_owned(), Source::Agent)]
    );
    let names = fs::read_dir(fixture.history_of(""))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    assert_eq!(names, ["Идея.md"]);
}

#[test]
fn a_renamed_folder_leaves_no_empty_history_folders() {
    let fixture = Fixture::new();
    let history = fixture.service();
    for name in ["А.md", "Б.md"] {
        let note = fixture.vault.join("Старая").join(name);
        fixture.save(&history, &note, name, Source::Agent);
        fixture.save(&history, &note, &format!("{name}!"), Source::Agent);
    }
    fs::rename(fixture.vault.join("Старая"), fixture.vault.join("Новая")).unwrap();

    for name in ["А.md", "Б.md"] {
        history.moved(&|| fixture.roots(), &fixture.vault.join("Старая").join(name), &fixture.vault.join("Новая").join(name));
    }

    assert!(!fixture.history_of("Старая").exists());
    assert_eq!(
        texts(&fixture.history_of("Новая/Б.md")),
        [("Б.md".to_owned(), Source::Agent), ("Б.md!".to_owned(), Source::Agent)]
    );
}

#[test]
fn the_history_goes_to_the_trash_with_its_note_and_comes_back() {
    let fixture = Fixture::new();
    let history = fixture.service();
    let note = fixture.vault.join("Удаляемая.md");
    fixture.save(&history, &note, "начало", Source::Agent);
    fixture.save(&history, &note, "начало и середина", Source::Me);

    let trashed = move_to_trash_impl(&fixture.vault, &note).unwrap().root;
    history.trashed(&|| fixture.roots(), &note, &trashed);

    let in_trash = history_dir(&trash_path(&fixture.vault), Path::new("Удаляемая.md"));
    assert_eq!(
        texts(&in_trash),
        [("начало".to_owned(), Source::Agent), ("начало и середина".to_owned(), Source::Me)]
    );
    assert!(!fixture.vault.join(".aquilum").exists());

    let restored = restore_from_trash_impl(&fixture.vault, &trashed).unwrap().root;
    history.restored(&|| fixture.roots(), &trashed, &restored);
    fixture.save(&history, &restored, "начало и середина от агента", Source::Agent);

    assert_eq!(
        texts(&fixture.history_of("Удаляемая.md")),
        [
            ("начало".to_owned(), Source::Agent),
            ("начало и середина".to_owned(), Source::Me),
            ("начало и середина от агента".to_owned(), Source::Agent),
        ]
    );
    assert!(!trash_path(&fixture.vault).join(".aquilum").exists());
}

#[test]
fn a_second_copy_in_the_trash_keeps_its_own_history() {
    let fixture = Fixture::new();
    let history = fixture.service();
    let note = fixture.vault.join("Идея.md");
    let mut trashed = Vec::new();
    for text in ["первая", "вторая"] {
        fixture.save(&history, &note, text, Source::Agent);
        fixture.save(&history, &note, &format!("{text}!"), Source::Agent);
        let moved = move_to_trash_impl(&fixture.vault, &note).unwrap().root;
        history.trashed(&|| fixture.roots(), &note, &moved);
        trashed.push(moved);
    }

    let restored = restore_from_trash_impl(&fixture.vault, &trashed[1]).unwrap().root;
    history.restored(&|| fixture.roots(), &trashed[1], &restored);

    assert_eq!(
        texts(&fixture.history_of("Идея.md")),
        [("вторая".to_owned(), Source::Agent), ("вторая!".to_owned(), Source::Agent)]
    );
    let first = history_dir(&trash_path(&fixture.vault), Path::new("Идея.md"));
    assert_eq!(
        texts(&first),
        [("первая".to_owned(), Source::Agent), ("первая!".to_owned(), Source::Agent)]
    );
}
