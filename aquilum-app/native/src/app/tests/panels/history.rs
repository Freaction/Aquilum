use super::*;
use crate::ui::widgets::{ItemAction, Pressed};
use aquilum_core::files::document::hash_file;
use aquilum_core::files::gate;
use aquilum_core::history::Source;

fn with_history(texts: &[&str]) -> (Harness, std::path::PathBuf) {
    let mut h = Harness::new();
    let note = aquilum_core::search::paths::canonical_path(h.vault.path()).join("история.md");
    std::fs::write(&note, texts[0]).unwrap();
    for text in texts {
        let hash = hash_file(&note).unwrap();
        gate::write(&h.app.workspace.core, &note, text, Some(&hash), Source::Agent, None).unwrap();
    }
    h.open(note.clone());
    let button = h.app.chrome.as_ref().unwrap().panel_parts.modes.last().unwrap().0.id();
    h.send(button, Pressed);
    (h, note)
}

fn revert_from_panel(h: &mut Harness, index: usize) {
    let rows = h.app.history_body.as_ref().expect("список версий").rows.clone();
    let row = rows.iter().find(|(_, at)| *at == index).expect("строка версии").0;
    h.send(row, ItemAction::Open { new_tab: false });
    let button = h.app.version_view.as_ref().and_then(|view| view.revert).expect("у версии есть кнопка отмены");
    h.send(button, Pressed);
}

#[test]
fn a_revert_keeps_unsaved_typing_and_brings_back_a_deleted_line() {
    let (mut h, note) = with_history(&["раз\nдва\nтри\n", "раз\nтри\n"]);
    h.replace_body("раз\nтри\nнабрано\n");
    revert_from_panel(&mut h, 0);
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "раз\nдва\nтри\nнабрано\n", "удалённая строка вернулась, несохранённый набор остался");
    assert_eq!(h.app.text.to_string(), "раз\nдва\nтри\nнабрано\n", "редактор показывает результат отмены");
    assert!(h.app.version_view.is_none(), "после отмены открыта сама заметка");
    let newest = h.app.history.as_ref().unwrap().versions[0].clone();
    assert!(matches!(newest.source, Source::Revert(_)), "отмена — новая версия сверху списка: {:?}", newest.source);
}

#[test]
fn reverting_the_same_version_twice_from_the_panel_changes_nothing() {
    let (mut h, note) = with_history(&["раз\nдва\nтри\n", "раз\nтри\n"]);
    revert_from_panel(&mut h, 0);
    let total = h.app.history.as_ref().unwrap().versions.len();
    revert_from_panel(&mut h, 1);
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "раз\nдва\nтри\n", "повторная отмена не удваивает строку");
    assert_eq!(h.app.text.to_string(), "раз\nдва\nтри\n");
    assert_eq!(h.app.history.as_ref().unwrap().versions.len(), total, "повторная отмена не добавляет версию");
}
