use super::*;

#[test]
fn conflicting_edit_is_kept_as_a_copy() {
    let mut h = Harness::new();
    let path = h.vault.path().join("a.md");
    h.open(path.clone());
    std::fs::write(&path, "снаружи").unwrap();
    h.app.text = crop::Rope::from("моя правка");
    h.app.note.as_mut().unwrap().save_soon();
    h.save_now();
    let copies: Vec<_> = std::fs::read_dir(h.vault.path()).unwrap().flatten().map(|e| e.path()).filter(|p| p.to_string_lossy().contains("конфликт")).collect();
    assert_eq!(copies.len(), 1, "копия-конфликт не записана");
    assert!(std::fs::read_to_string(&copies[0]).unwrap().contains("моя правка"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "снаружи");
    assert!(h.app.text.to_string().contains("снаружи"), "{}", h.app.text.to_string());
}

#[test]
fn inserted_text_reaches_the_disk() {
    let mut h = Harness::new();
    let path = h.vault.path().join("a.md");
    h.open(path.clone());
    h.app.commit_text(&mut h.root, crop::Rope::from("---\ntags: []\n---\nтекст"));
    h.save_now();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "---\ntags: []\n---\nтекст");
    assert_eq!(h.app.metadata_state(), Some(true), "вставленные метаданные раскрыты");
}

#[test]
fn unsaved_text_is_written_on_shutdown() {
    let mut h = Harness::new();
    let path = h.vault.path().join("a.md");
    h.open(path.clone());
    h.app.commit_text(&mut h.root, crop::Rope::from("несохранённая правка"));
    h.app.shutdown();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "несохранённая правка");
}

#[test]
fn typed_edits_reach_the_file_behind_hidden_frontmatter() {
    let mut h = Harness::new();
    let path = h.write_note("правки.md", "---\ntags: [a]\n---\nпривет мир");
    h.open(path.clone());
    assert!(h.app.hidden > 0, "метаданные должны быть скрыты");
    let body = h.app.note_ids.as_ref().unwrap().body;
    let end = h.body_text().len();
    h.root.focus_on(Some(body.id()));
    body.edit(&mut h.root, |mut b| crate::ui::editor::TextEditor::select_byte_range(&mut b, end, end));
    h.type_text("!ё");
    h.key(Key::Named(NamedKey::Backspace), Modifiers::empty());
    body.edit(&mut h.root, |mut b| crate::ui::editor::TextEditor::select_byte_range(&mut b, 0, 0));
    h.key(Key::Character("Ж".into()), Modifiers::empty());
    assert_eq!(h.app.text.to_string(), "---\ntags: [a]\n---\nЖпривет мир!");
    h.save_now();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "---\ntags: [a]\n---\nЖпривет мир!");
}

#[test]
fn leaving_the_window_or_the_tab_writes_the_pending_edit() {
    let mut h = Harness::new();
    let path = h.vault.path().join("a.md");
    h.open(path.clone());
    h.app.commit_text(&mut h.root, crop::Rope::from("правка до ухода из окна"));
    h.app.window_unfocused();
    crate::disk::wait();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "правка до ухода из окна");

    h.app.commit_text(&mut h.root, crop::Rope::from("правка до смены вкладки"));
    let other = h.write_note("b.md", "другая");
    h.app.open_in(&mut h.root, other, true);
    h.parked();
    crate::disk::wait();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "правка до смены вкладки");
}
