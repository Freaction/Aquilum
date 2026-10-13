use super::*;
use crate::ui::titlebar::TitlebarAction;

#[test]
fn tabs_open_new_select_and_close() {
    let mut h = Harness::new();
    let note = h.vault.path().join("a.md");
    h.app.open(&mut h.root, note.clone());
    assert_eq!(h.app.tabs.tabs().len(), 1);
    assert!(h.app.note.is_some());

    let plus = h.root.get_widget(h.app.chrome.as_ref().unwrap().new_tab).unwrap().ctx().bounding_box().center();
    let outcome = h.click(plus.x, plus.y);
    assert!(outcome.title);
    assert_eq!(h.app.tabs.tabs().len(), 2);
    assert!(h.app.tabs.active().path.is_none());
    assert!(h.app.note.is_none());

    let first = h.app.tabs.tabs()[0].id;
    let strip = h.app.chrome.as_ref().unwrap().tab_strip.id();
    h.send(strip, TitlebarAction::Select(first));
    assert_eq!(h.app.tabs.active_id(), first);
    assert!(h.app.note.as_ref().is_some_and(|n| n.is(&note)));

    let second = h.app.tabs.tabs()[1].id;
    h.send(strip, TitlebarAction::Close(second));
    assert_eq!(h.app.tabs.tabs().len(), 1);

    let outcome = h.send(strip, TitlebarAction::CloseWindow);
    assert_eq!(outcome.window, Some(crate::app::WindowCommand::Close));
}

#[test]
fn ctrl_shift_click_opens_note_in_new_tab_and_tabs_reorder_by_drag() {
    let mut h = Harness::new();
    h.write_note("b.md", "");
    let vault = h.vault.path().to_path_buf();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("b.md")]);
    let center = |h: &Harness, name: &str| h.root.get_widget(h.app.tree.row_id(name).unwrap()).unwrap().ctx().bounding_box().center();
    let a = center(&h, "a");
    h.click(a.x, a.y);
    assert_eq!(h.tab_paths(), [Some("a".to_owned())]);
    let b = center(&h, "b");
    h.click_with(b.x, b.y, crate::test_dir::command() | Modifiers::SHIFT);
    assert_eq!(h.tab_paths(), [Some("a".to_owned()), Some("b".to_owned())]);
    assert_eq!(h.app.tabs.active().path.as_deref(), Some(vault.join("b.md").as_path()));

    let first = 266.0 + 40.0;
    h.drag((first, 20.0), (first + 200.0, 20.0));
    assert_eq!(h.tab_paths(), [Some("b".to_owned()), Some("a".to_owned())]);
}

#[test]
fn tab_menu_renames_note_in_tab() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    h.app.open(&mut h.root, vault.join("a.md"));
    let id = h.app.tabs.active_id();
    let strip = h.app.chrome.as_ref().unwrap().tab_strip.id();
    h.send(strip, TitlebarAction::Menu(id, masonry::kurbo::Point::new(300.0, 20.0)));
    let layer = match &h.app.popup {
        Some(Popup::FileMenu { layer, tab: Some(_), .. }) => *layer,
        _ => panic!("меню вкладки"),
    };
    h.send(layer, crate::ui::menu::MenuChoice::Item(0));
    let field = h.app.rename_field.expect("поле переименования во вкладке");
    assert_eq!(h.root.focused_widget(), Some(field.id()));
    h.send(field.id(), crate::ui::editor::TextAction::Entered("Новое имя".into()));
    assert!(h.app.rename_field.is_none());
    let renamed = vault.join("Новое имя.md");
    wait_until(250, 20, || renamed.exists() && !vault.join("a.md").exists());
    assert!(renamed.exists());
    assert!(!vault.join("a.md").exists());
}
