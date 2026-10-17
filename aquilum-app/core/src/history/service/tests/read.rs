use super::{texts, write, Fixture};
use crate::history::milestone::Source;
use crate::history::service::VersionView;
use std::fs;

#[test]
fn the_page_lists_past_versions_newest_first_and_a_version_comes_with_the_one_before() {
    let fixture = Fixture::new();
    let history = fixture.service();
    let note = fixture.vault.join("Заметка.md");
    for text in ["первая", "вторая", "третья"] {
        fixture.save(&history, &note, text, Source::Agent);
    }

    let page = history.page(&|| fixture.roots(), &note, None, 0, 1);

    assert_eq!(page.total, 3);
    let full = |id: &str| match history.view(&|| fixture.roots(), &note, id) {
        Some(VersionView::Full(texts)) => (texts.text, texts.previous),
        _ => panic!("маленькая версия показывается целиком"),
    };
    assert_eq!(full(&page.versions[0].id), ("третья".to_owned(), Some("вторая".to_owned())));
    let second = history.page(&|| fixture.roots(), &note, None, 1, 1).versions.remove(0);
    assert_eq!(full(&second.id).1.as_deref(), Some("первая"));
    let first = history.page(&|| fixture.roots(), &note, None, 2, 1).versions.remove(0);
    assert_eq!(full(&first.id).1, None);
}

#[test]
fn reading_progress_after_a_restart_adds_no_version() {
    let fixture = Fixture::new();
    let note = fixture.vault.join("Книга.md");
    let first = fixture.service();
    fixture.save(&first, &note, "книга", Source::Agent);
    fixture.save(&first, &note, "книга и заметка", Source::Agent);
    fs::write(&note, "книга и заметка позиция 1").unwrap();

    let history = fixture.service();
    fixture.save(&history, &note, "книга и заметка позиция 2", Source::Reading);

    assert_eq!(
        texts(&fixture.history_of("Книга.md")),
        [("книга".to_owned(), Source::Agent), ("книга и заметка".to_owned(), Source::Agent)]
    );
}

#[test]
fn a_restore_creates_a_restored_version_immediately() {
    let fixture = Fixture::new();
    let history = fixture.service();
    let note = fixture.vault.join("Заметка.md");
    let roots = || fixture.roots();
    fixture.save(&history, &note, "первая", Source::Agent);
    fixture.save(&history, &note, "первая и правка", Source::Me);
    let first = history.page(&roots, &note, None, 1, 1).versions.remove(0);

    fixture.save(&history, &note, "первая", Source::Restore(first.at_ms));

    assert_eq!(
        texts(&fixture.history_of("Заметка.md")),
        [
            ("первая".to_owned(), Source::Agent),
            ("первая и правка".to_owned(), Source::Me),
            ("первая".to_owned(), Source::Restore(first.at_ms)),
        ]
    );
    let page = history.page(&roots, &note, None, 0, 10);
    assert_eq!(page.versions[0].from_ms, Some(first.at_ms));
    let current = page.versions.iter().map(|version| version.is_current).collect::<Vec<_>>();
    assert_eq!(current, [true, false, true]);
}

#[test]
fn a_version_gets_a_name_and_latest_can_be_named() {
    let fixture = Fixture::new();
    let history = fixture.service();
    let note = fixture.vault.join("Заметка.md");
    let roots = || fixture.roots();
    fixture.save(&history, &note, "первая", Source::Agent);
    fixture.save(&history, &note, "вторая", Source::Agent);
    let first = history.page(&roots, &note, None, 1, 1).versions.remove(0);

    assert_eq!(history.name(&roots, &note, Some(&first.id), "Начало"), Some(first.id.clone()));
    assert_eq!(history.name(&roots, &note, Some("0000000000000_aaaaaaaa_me.md"), "Нет"), None);
    let live = history.name(&roots, &note, None, "Черновик").unwrap();

    let page = history.page(&roots, &note, None, 0, 10);
    let named = page.versions.iter().map(|version| version.name.clone()).collect::<Vec<_>>();
    assert_eq!(named, [Some("Черновик".to_owned()), Some("Начало".to_owned())]);
    assert_eq!(page.versions[0].id, live);
}

#[test]
fn a_write_with_a_name_names_the_version_it_produces() {
    let fixture = Fixture::new();
    let history = fixture.service();
    let note = fixture.vault.join("Заметка.md");
    let roots = || fixture.roots();

    let hash = crate::files::document::hash_bytes("от агента".as_bytes());
    history.saved(&roots, &note, write(&"от агента", Source::Agent, &hash, Some("План")));
    history.saved(&roots, &note, write(&"от агента", Source::Agent, &hash, Some("План, ещё раз")));

    let page = history.page(&roots, &note, None, 0, 10);
    assert_eq!(page.total, 1);
    assert_eq!(page.versions[0].name.as_deref(), Some("План, ещё раз"));
}

#[test]
fn only_a_listed_version_can_be_read() {
    let fixture = Fixture::new();
    let history = fixture.service();
    let note = fixture.vault.join("Заметка.md");
    fixture.save(&history, &note, "текст", Source::Agent);
    fs::write(fixture.vault.join("Секрет.md"), "не отсюда").unwrap();

    assert!(history.view(&|| fixture.roots(), &note, "../../../Секрет.md").is_none());
    assert!(history.content(&|| fixture.roots(), &note, "0000000000000_aaaaaaaa_me.md").is_none());
}

#[test]
fn a_note_outside_known_workspaces_has_no_history() {
    let fixture = Fixture::new();
    let history = fixture.service();
    let stray = fixture.data.join("Чужая.md");
    fs::create_dir_all(&fixture.data).unwrap();
    fs::write(&stray, "текст").unwrap();

    history.saved(&|| fixture.roots(), &stray, write(&"текст!", Source::Agent, "hash", None));

    assert!(!fixture.vault.join(".aquilum").exists());
    assert!(!fixture.data.join(".aquilum").exists());
}
