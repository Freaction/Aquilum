use super::*;
use crate::ui::file_tree::{FileTree, Sort};
use crate::ui::tokens::size;
use masonry::core::WidgetId;
use masonry::kurbo::Rect;

fn rect(h: &Harness, id: WidgetId) -> Rect {
    h.root.get_widget(id).unwrap().ctx().bounding_box()
}

fn press(h: &mut Harness, id: WidgetId) {
    let center = rect(h, id).center();
    h.click(center.x, center.y);
}

fn touch(path: &std::path::Path, seconds: u64) {
    let file = std::fs::File::options().write(true).open(path).unwrap();
    file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds)).unwrap();
}

#[test]
fn toolbar_is_four_small_buttons_above_the_tree() {
    let h = Harness::new();
    let ids = h.app.tree.toolbar_ids().expect("ряд кнопок над деревом");
    let rects: Vec<Rect> = ids.iter().map(|&id| rect(&h, id)).collect();
    for r in &rects {
        assert_eq!((r.width(), r.height()), (size::SIZE_24, size::SIZE_24), "кнопка не размера S");
    }
    assert_eq!(rects[1].x0 - rects[0].x1, size::SPACE_6);
    assert_eq!(rects[2].x0 - rects[1].x1, size::SPACE_6);
    let tree = rect(&h, h.app.tree.scroll_id().unwrap());
    assert_eq!(rects[3].x1, tree.x1 - size::SPACE_8, "«Свернуть всё» не прижата вправо");
    assert_eq!(rects[0].x0, tree.x0 + size::SPACE_8);
    assert!(rects.iter().all(|r| r.y0 == rects[0].y0));
    let first = rect(&h, h.app.tree.row_id("a").expect("строка заметки"));
    let field = rect(&h, h.app.tree.filter_field().expect("поле поиска"));
    assert_eq!(field.y0 - size::INPUT_XS_PADDING_Y - rects[0].y1, size::SPACE_4 + size::SPACE_8, "поле поиска не под кнопками с отступом 8");
    assert_eq!(first.y0 - (field.y1 + size::INPUT_XS_PADDING_Y), size::SPACE_8, "между полем и деревом нет линии, только отступы");
}

#[test]
fn new_note_goes_into_clicked_folder_and_starts_rename() {
    let mut h = Harness::new();
    let folder = h.vault.path().join("Проекты");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("план.md"), "").unwrap();
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&folder));
    let row = h.app.tree.row_id("Проекты").expect("строка папки");
    press(&mut h, row);
    let [new_note, ..] = h.app.tree.toolbar_ids().unwrap();
    press(&mut h, new_note);
    let created = folder.join("Без названия.md");
    assert!(created.is_file(), "заметка не создана в выбранной папке");
    assert_eq!(h.app.tabs.active().path.as_deref(), Some(created.as_path()), "новая заметка не открылась");
    assert_eq!(h.app.tree.renaming(), Some(created), "строка новой заметки не перешла в переименование");
}

#[test]
fn new_note_rename_field_has_focus_and_selected_name() {
    let mut h = Harness::new();
    let [new_note, ..] = h.app.tree.toolbar_ids().unwrap();
    press(&mut h, new_note);
    let created = h.vault.path().join("Без названия.md");
    for round in 0..2 {
        let field = h.app.tree.rename_field().expect("поле переименования");
        assert_eq!(h.root.focused_widget(), Some(field), "фокус не в поле переименования (проход {round})");
        let editor = h.root.get_widget(field).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap();
        assert_eq!(editor.inner().selected_text().as_deref(), Some("Без названия"), "имя не выделено (проход {round})");
        h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&created));
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
    }
}

#[test]
fn new_folder_without_selection_goes_to_root_and_starts_rename() {
    let mut h = Harness::new();
    let [_, new_folder, ..] = h.app.tree.toolbar_ids().unwrap();
    press(&mut h, new_folder);
    let created = h.vault.path().join("Без названия");
    assert!(created.is_dir(), "папка не создана в корне");
    assert_eq!(h.app.tree.renaming(), Some(created.clone()));
    press(&mut h, new_folder);
    assert!(h.vault.path().join("Без названия 1").is_dir(), "вторая папка не получила свободное имя");
}

#[test]
fn collapse_all_closes_folders_and_remembers_it() {
    let mut h = Harness::new();
    let folder = h.vault.path().join("Проекты");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("план.md"), "").unwrap();
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&folder));
    let row = h.app.tree.row_id("Проекты").unwrap();
    press(&mut h, row);
    assert!(h.app.tree.names().contains(&"план".to_owned()), "папка не раскрылась");
    let [.., collapse] = h.app.tree.toolbar_ids().unwrap();
    press(&mut h, collapse);
    assert_eq!(h.app.tree.names(), ["Проекты", "a"]);
    let reopened = FileTree::new(Some(h.vault.path().to_path_buf()), h._data.path(), std::sync::Arc::new(|| {}));
    assert_eq!(reopened.names(), ["Проекты", "a"], "свёрнутое дерево не запомнилось");
}

#[test]
fn sort_menu_reorders_the_tree_and_remembers_the_choice() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    for (name, seconds) in [("b.md", 2_000), ("c.md", 3_000)] {
        std::fs::write(vault.join(name), "").unwrap();
        touch(&vault.join(name), seconds);
    }
    touch(&vault.join("a.md"), 1_000);
    h.app.tree.paths_changed(&mut h.root, &[vault.join("b.md"), vault.join("c.md")]);
    assert_eq!(h.app.tree.names(), ["a", "b", "c"]);
    let [_, _, sort, _] = h.app.tree.toolbar_ids().unwrap();
    press(&mut h, sort);
    let Some(Popup::SortMenu { layer, .. }) = h.app.popup else { panic!("меню сортировки не открылось") };
    let labels: Vec<String> = Sort::ALL.iter().map(|s| crate::i18n::t(s.label())).collect();
    let items: Vec<&str> = labels.iter().map(String::as_str).collect();
    let plain = crate::ui::menu::Menu::new("", &items);
    let plain_id = plain.id();
    h.root.add_layer(plain, masonry::kurbo::Point::ORIGIN);
    let _ = h.root.redraw();
    let content = rect(&h, plain_id).width();
    h.root.remove_layer(plain_id);
    let list = rect(&h, layer).width();
    assert!(list >= content + size::SIZE_20 + size::GAP_MD, "список сортировки уже подписей: {list} < {content} + галочка");
    let modified_new = Sort::ALL.iter().position(|s| *s == Sort::ModifiedNew).unwrap();
    let choice = masonry::core::ErasedAction::from(Box::new(crate::ui::menu::MenuChoice::Item(modified_new)));
    h.app.on_action(&mut h.root, layer, choice);
    assert!(h.app.popup.is_none(), "меню не закрылось после выбора");
    assert_eq!(h.app.tree.names(), ["c", "b", "a"]);
    let reopened = FileTree::new(Some(vault), h._data.path(), std::sync::Arc::new(|| {}));
    assert_eq!(reopened.sort(), Sort::ModifiedNew, "выбор сортировки не запомнился");
    press(&mut h, sort);
    assert!(matches!(h.app.popup, Some(Popup::SortMenu { .. })));
    press(&mut h, sort);
    assert!(h.app.popup.is_none(), "повторный щелчок по кнопке не закрыл меню");
}

#[test]
fn search_button_in_the_rail_opens_the_omni_search() {
    let mut h = Harness::new();
    let search = h.app.chrome.as_ref().expect("оболочка").search_button;
    assert!(rect(&h, search).x1 <= rect(&h, h.app.tree.scroll_id().unwrap()).x0, "кнопка поиска не в левой колонке иконок");
    assert!(!matches!(h.app.popup, Some(Popup::Search(_))));
    press(&mut h, search);
    assert!(matches!(h.app.popup, Some(Popup::Search(_))), "омнипоиск не открылся");
}

#[test]
fn new_notes_go_to_the_folder_from_settings() {
    let mut h = Harness::new();
    let mut config = h.config();
    config.files.notes = " Входящие/ ".into();
    h.app.workspace.core.settings.update_config(config).unwrap();
    let (vault, inbox) = (h.vault.path().to_path_buf(), h.vault.path().join("Входящие"));

    assert!(h.app.new_note(&mut h.root));
    let [new_note, ..] = h.app.tree.toolbar_ids().unwrap();
    press(&mut h, new_note);
    h.app.open_target(&mut h.root, crate::links::Target::Wiki("Идея".into()), false);
    h.app.open_target(&mut h.root, crate::links::Target::Wiki("Проекты/План".into()), false);

    for created in [inbox.join("Без названия.md"), inbox.join("Без названия 1.md"), inbox.join("Идея.md"), vault.join("Проекты").join("План.md")] {
        assert!(created.is_file(), "{}", created.display());
    }
    assert!(!vault.join("Без названия.md").exists() && !vault.join("Идея.md").exists(), "заметка попала в корень");
}
