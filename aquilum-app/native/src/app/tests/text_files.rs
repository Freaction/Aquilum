use super::*;
use crate::ui::editor::TextAction;

#[test]
fn text_files_are_listed_opened_edited_and_saved() {
    let mut h = Harness::new();
    let path = h.write_note("заметка.txt", "первая строка");
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&path));
    assert!(h.app.tree.names().iter().any(|name| name == "заметка.txt"), "текстовый файл не показан в дереве");
    assert!(h.app.open(&mut h.root, path.clone()), "текстовый файл не открылся");
    assert_eq!(h.body_text(), "первая строка");
    let body = h.app.note_ids.as_ref().unwrap().body.id();
    h.act(body, TextAction::Changed("первая строка\nвторая".into()));
    h.app.tick(&mut h.root, std::time::Instant::now() + std::time::Duration::from_secs(3600));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "первая строка\nвторая", "правка не сохранилась в .txt");
}

#[test]
fn text_file_keeps_its_extension_when_the_title_is_edited() {
    use std::path::Path;
    let (to, _) = crate::file_ops::title_target(Path::new("/база/заметка.txt"), "Новое: имя").unwrap();
    assert_eq!(to, Path::new("/база/Новое имя.txt"));
    let (to, _) = crate::file_ops::title_target(Path::new("/база/заметка.md"), "Новое").unwrap();
    assert_eq!(to, Path::new("/база/Новое.md"));
}

#[test]
fn text_file_follows_outside_edits_and_has_properties_like_a_note() {
    let mut h = Harness::new();
    h.watch_vault();
    let path = h.write_note("заметка.txt", "---\nтег: один\n---\nтекст");
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&path));
    assert!(h.app.open(&mut h.root, path.clone()));
    assert!(h.app.metadata_state().is_some(), "у .txt нет блока свойств, как у заметки");
    std::fs::write(&path, "---\nтег: один\n---\nизменён снаружи").unwrap();
    assert!(
        wait_until(300, 20, || {
            h.app.on_core_events(&mut h.root);
            h.body_text().contains("изменён снаружи")
        }),
        "правка .txt снаружи не подтянулась в редактор"
    );
}
