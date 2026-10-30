use super::workspaces::{choose, menu_layer, open_dialog, places, remember, rename_field, row, rows, state};
use super::*;
use crate::app::book::Picked;
use crate::app::workspaces::manage::{DELETE, DUPLICATE, RENAME, SYNC};
use crate::i18n::t;
use crate::ui::dialog::DialogAnswer;
use crate::ui::editor::{TextAction, TextEditor};
use aquilum_core::search::paths::same_path;

fn synced(h: &Harness, path: &std::path::Path) -> bool {
    h.app.workspace.core.ui_state.workspace(path).unwrap().sync.enabled
}

fn answer_delete(h: &mut Harness, path: &std::path::Path, answer: DialogAnswer) {
    choose(h, path, DELETE);
    let (confirm, cancel) = state(h).cancel();
    assert!(cancel.downcast_ref::<DialogAnswer>().is_some() && confirm != state(h).dialog.layer, "нет вопроса об удалении");
    h.act(confirm, answer);
    crate::disk::wait();
}

#[test]
fn new_vaults_in_one_folder_get_different_names_and_open_for_renaming() {
    let (places, vaults) = places(&["Первая"]);
    let mut h = Harness::with(Some(&vaults[0]), 1.0);
    open_dialog(&mut h);
    let made: Vec<_> = (0..2).map(|_| crate::file_ops::create_folder(places.path(), &t("workspaces.newName"), 1).unwrap()).collect();
    let names: Vec<_> = made.iter().map(|path| path.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["База знаний 1", "База знаний 2"]);

    h.app.picked_tx.send(Picked::NewVault(Ok(made[1].clone()))).unwrap();
    h.step();
    let field = rename_field(&h).expect("новая база не перешла в переименование");
    assert_eq!(h.root.focused_widget(), Some(field));
    let editor = h.root.get_widget(field).unwrap().downcast::<TextEditor>().unwrap();
    assert_eq!(editor.inner().selection(), 0..editor.inner().len(), "имя не выделено целиком");
    h.settle();
    h.snapshot("vault-rename.png");

    h.type_text("Проекты");
    h.key(Key::Named(NamedKey::Enter), Modifiers::empty());
    let renamed = places.path().join("Проекты");
    assert!(renamed.is_dir() && !made[1].exists());
    assert!(rename_field(&h).is_none());
    assert!(rows(&h).iter().any(|(_, path, _)| same_path(path, &renamed)));
    assert!(same_path(h.app.tree.root().unwrap(), &vaults[0]), "открытая база не должна меняться");
}

#[test]
fn renaming_the_open_vault_moves_it_and_keeps_its_tabs() {
    let (places, vaults) = places(&["Первая"]);
    let mut h = Harness::with(Some(&vaults[0]), 1.0);
    h.open(vaults[0].join("a.md"));
    let id = h.app.workspace.core.ui_state.workspace(&vaults[0]).unwrap().id;
    open_dialog(&mut h);
    choose(&mut h, &vaults[0], RENAME);
    let field = rename_field(&h).unwrap();
    h.act(field, TextAction::Entered("Вторая".into()));

    let renamed = places.path().join("Вторая");
    assert_eq!(state(&h).error.as_deref(), None);
    assert!(renamed.is_dir() && !vaults[0].exists());
    assert!(same_path(h.app.tree.root().unwrap(), &renamed));
    assert!(h.app.shows(&renamed.join("a.md")), "вкладка не вернулась после переименования");
    assert_eq!(h.app.workspace.core.ui_state.workspace(&renamed).unwrap().id, id);
    assert_eq!(rows(&h).len(), 1);
}

#[test]
fn a_taken_name_is_reported_and_escape_or_back_cancel_only_the_renaming() {
    let (_places, vaults) = places(&["Первая", "Вторая"]);
    let mut h = Harness::with(Some(&vaults[0]), 1.0);
    remember(&h, &vaults[1]);
    open_dialog(&mut h);
    choose(&mut h, &vaults[1], RENAME);
    h.act(rename_field(&h).unwrap(), TextAction::Entered("Первая".into()));
    assert!(vaults[1].is_dir());
    assert!(state(&h).error.as_deref().unwrap().contains(&t("workspaces.nameTaken")));

    choose(&mut h, &vaults[1], RENAME);
    assert_eq!(state(&h).error.as_deref(), None);
    h.escape();
    assert!(matches!(h.app.popup, Some(Popup::Workspaces(_))), "Escape закрыл окно баз");
    assert!(rename_field(&h).is_none());

    choose(&mut h, &vaults[1], RENAME);
    h.type_text("Третья");
    h.app.back(&mut h.root);
    assert!(matches!(h.app.popup, Some(Popup::Workspaces(_))), "«Назад» закрыл окно баз");
    assert!(rename_field(&h).is_none());
    assert!(vaults[1].is_dir(), "«Назад» должен отменять, а не сохранять имя");
}

#[test]
fn deleting_a_vault_asks_first_and_then_removes_its_folder() {
    let (_places, vaults) = places(&["Первая", "Вторая"]);
    let mut h = Harness::with(Some(&vaults[0]), 1.0);
    remember(&h, &vaults[1]);
    open_dialog(&mut h);
    answer_delete(&mut h, &vaults[1], DialogAnswer::Cancel);
    assert!(vaults[1].is_dir());
    assert_eq!(rows(&h).len(), 2);

    answer_delete(&mut h, &vaults[1], DialogAnswer::Confirm);
    assert!(!vaults[1].exists());
    assert_eq!(rows(&h).len(), 1);
    assert!(same_path(h.app.tree.root().unwrap(), &vaults[0]));
}

#[test]
fn deleting_the_open_vault_moves_on_and_the_last_one_leaves_the_empty_screen() {
    let (_places, vaults) = places(&["Первая", "Вторая"]);
    let mut h = Harness::with(Some(&vaults[0]), 1.0);
    h.open(vaults[0].join("a.md"));
    remember(&h, &vaults[1]);
    open_dialog(&mut h);
    answer_delete(&mut h, &vaults[0], DialogAnswer::Confirm);
    assert!(!vaults[0].exists());
    assert!(same_path(h.app.tree.root().unwrap(), &vaults[1]));

    answer_delete(&mut h, &vaults[1], DialogAnswer::Confirm);
    assert!(!vaults[1].exists());
    assert!(h.app.tree.root().is_none());
    assert!(rows(&h).is_empty());
    let buttons = &h.app.chrome.as_ref().unwrap().vault_buttons;
    assert_eq!(buttons.open.is_some(), crate::system::DESKTOP);
    h.escape();
    assert!(h.app.popup.is_none());
    h.snapshot("no-vault.png");
}

#[test]
fn duplicating_a_vault_copies_its_notes_next_to_it() {
    let (places, vaults) = places(&["Первая"]);
    let mut h = Harness::with(Some(&vaults[0]), 1.0);
    open_dialog(&mut h);
    choose(&mut h, &vaults[0], DUPLICATE);
    assert!(h.wait_for(|h| rename_field(h).is_some()), "копия не появилась в списке");
    let copy = places.path().join("Первая (1)");
    assert_eq!(std::fs::read_to_string(copy.join("a.md")).unwrap(), "Первая");
    assert_eq!(rows(&h).len(), 2);
}

#[test]
fn sync_is_toggled_per_vault_and_needs_dropbox_to_turn_on() {
    let (_places, vaults) = places(&["Первая", "Вторая"]);
    let mut h = Harness::with(Some(&vaults[0]), 1.0);
    remember(&h, &vaults[1]);
    let on = aquilum_core::settings::models::SyncSettings { enabled: true, ..Default::default() };
    h.app.workspace.core.ui_state.set_sync(&vaults[1].to_string_lossy(), &on).unwrap();
    open_dialog(&mut h);
    choose(&mut h, &vaults[1], SYNC);
    assert!(!synced(&h, &vaults[1]));
    assert!(matches!(h.app.popup, Some(Popup::Workspaces(_))));

    choose(&mut h, &vaults[1], SYNC);
    assert!(synced(&h, &vaults[1]) && !synced(&h, &vaults[0]));
    assert_eq!(h.section(), Some(Section::Sync), "без входа в Dropbox должен открыться раздел синхронизации");
}

#[test]
fn the_vault_menu_opens_from_the_dots_and_a_right_click_and_back_closes_it() {
    let (_places, vaults) = places(&["Первая"]);
    let mut h = Harness::with(Some(&vaults[0]), 1.0);
    open_dialog(&mut h);
    h.settle();
    let bounds = h.root.get_widget(row(&h, &vaults[0])).unwrap().ctx().bounding_box();
    h.pointer_move(bounds.center().x, bounds.center().y);
    h.click(bounds.x1 - crate::ui::tokens::size::SPACE_8 - 12.0, bounds.center().y);
    assert!(menu_layer(&h).is_some(), "«⋯» не открыли меню");
    h.snapshot("vault-menu.png");

    h.app.back(&mut h.root);
    assert!(menu_layer(&h).is_none(), "«Назад» не закрыл меню");
    assert!(matches!(h.app.popup, Some(Popup::Workspaces(_))));

    h.press(pointer_at(bounds.center().x, bounds.center().y, 1.0), PointerButton::Secondary, MOUSE);
    assert!(menu_layer(&h).is_some(), "правый клик не открыл меню");
}
