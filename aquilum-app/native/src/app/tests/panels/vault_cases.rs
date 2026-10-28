use super::workspaces::{choose, open_dialog, places, remember, rename_field, state};
use super::*;
use crate::app::book::Picked;
use crate::app::workspaces::manage::{DELETE, RENAME};
use crate::ui::dialog::DialogAnswer;
use crate::ui::editor::TextAction;
use crate::ui::scroll::Scrolled;
use aquilum_core::search::paths::same_path;

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[test]
fn scrolling_keeps_the_renaming_and_a_click_commits_it() {
    let (places, vaults) = places(&["Первая", "Вторая"]);
    let mut h = Harness::with(Some(&vaults[0]), 1.0);
    remember(&h, &vaults[1]);
    open_dialog(&mut h);
    choose(&mut h, &vaults[1], RENAME);
    let field = rename_field(&h).unwrap();
    h.type_text("Третья");
    let layer = state(&h).dialog.layer;
    h.act(layer, Scrolled(10.0));
    assert_eq!(rename_field(&h), Some(field), "прокрутка подтвердила имя");

    h.act(layer, DialogAnswer::Cancel);
    assert!(matches!(h.app.popup, Some(Popup::Workspaces(_))), "клик мимо поля должен только подтвердить имя");
    assert!(rename_field(&h).is_none());
    assert_eq!(names(places.path()), ["Первая", "Третья"]);
}

#[test]
fn only_the_letter_case_of_a_vault_can_change() {
    let (places, vaults) = places(&["Первая", "Вторая"]);
    let mut h = Harness::with(Some(&vaults[0]), 1.0);
    remember(&h, &vaults[1]);
    open_dialog(&mut h);
    choose(&mut h, &vaults[1], RENAME);
    h.act(rename_field(&h).unwrap(), TextAction::Entered("вторая".into()));
    assert_eq!(state(&h).error.as_deref(), None);
    assert_eq!(names(places.path()), ["Первая", "вторая"]);
}

#[test]
fn a_vault_made_in_the_background_does_not_close_other_windows() {
    let (places, vaults) = places(&["Первая"]);
    let mut h = Harness::with(Some(&vaults[0]), 1.0);
    h.app.open_settings(&mut h.root);
    let made = crate::file_ops::create_folder(places.path(), "База знаний", 1).unwrap();
    h.app.picked_tx.send(Picked::NewVault(Ok(made.clone()))).unwrap();
    h.step();
    assert!(matches!(h.app.popup, Some(Popup::Settings { .. })), "настройки закрылись");
    assert!(h.app.workspace.core.ui_state.workspace(&made).is_some(), "новая база не запомнена");
}

#[test]
fn deleting_a_vault_that_holds_the_open_one_moves_outside_it() {
    let (_places, vaults) = places(&["Внешняя", "Третья"]);
    let inner = vaults[0].join("Внутренняя");
    std::fs::create_dir(&inner).unwrap();
    let mut h = Harness::with(Some(&inner), 1.0);
    remember(&h, &vaults[0]);
    remember(&h, &vaults[1]);
    open_dialog(&mut h);
    choose(&mut h, &vaults[0], DELETE);
    let (confirm, _) = state(&h).cancel();
    h.act(confirm, DialogAnswer::Confirm);
    crate::disk::wait();
    assert!(!vaults[0].exists());
    assert!(same_path(h.app.tree.root().unwrap(), &vaults[1]));
}
