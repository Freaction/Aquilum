use std::path::{Path, PathBuf};

use super::*;
use crate::app::workspaces::WorkspacesState;
use crate::ui::menu::MenuChoice;
use crate::ui::widgets::Pressed;
use crate::ui::workspaces::RowAction;
use aquilum_core::search::paths::same_path;
use masonry::core::WidgetId;

pub(super) fn state(h: &Harness) -> &WorkspacesState {
    match &h.app.popup {
        Some(Popup::Workspaces(state)) => state,
        _ => panic!("окно баз не открыто"),
    }
}

pub(super) fn open_dialog(h: &mut Harness) {
    let button = h.app.chrome.as_ref().unwrap().workspaces;
    h.act(button, Pressed);
}

pub(super) fn rows(h: &Harness) -> Vec<(WidgetId, PathBuf, uuid::Uuid)> {
    let state = state(h);
    let rows: Vec<_> = state.rows.ids.iter().zip(&state.vaults).map(|(row, vault)| (*row, PathBuf::from(&vault.path), vault.id)).collect();
    assert!(rows.iter().all(|(id, _, _)| h.root.has_widget(*id)), "строки баз не показаны в окне");
    rows
}

pub(super) fn rename_field(h: &Harness) -> Option<WidgetId> {
    state(h).rows.rename.map(|field| field.id())
}

pub(super) fn row(h: &Harness, path: &Path) -> WidgetId {
    rows(h).into_iter().find(|(_, listed, _)| same_path(listed, path)).unwrap_or_else(|| panic!("нет строки {}", path.display())).0
}

pub(super) fn choose(h: &mut Harness, path: &Path, item: usize) {
    let row = row(h, path);
    h.act(row, RowAction::Menu(masonry::kurbo::Point::new(200.0, 200.0)));
    let menu = menu_layer(h).expect("меню базы не открылось");
    h.act(menu, MenuChoice::Item(item));
}

pub(super) fn menu_layer(h: &Harness) -> Option<WidgetId> {
    let (layer, action) = state(h).cancel();
    action.downcast_ref::<MenuChoice>().is_some().then_some(layer)
}

pub(super) fn remember(h: &Harness, path: &Path) {
    h.app.workspace.core.ui_state.resolve_workspace(&path.to_string_lossy(), 1).unwrap();
}

pub(super) fn places(names: &[&str]) -> (tempfile::TempDir, Vec<PathBuf>) {
    let places = crate::test_dir::tempdir();
    let vaults = names
        .iter()
        .map(|name| {
            let vault = places.path().join(name);
            std::fs::create_dir(&vault).unwrap();
            std::fs::write(vault.join("a.md"), *name).unwrap();
            vault
        })
        .collect();
    (places, vaults)
}

#[test]
fn workspace_dialog_switches_vaults() {
    let mut h = Harness::new();
    let first = h.vault.path().to_path_buf();
    h.open(first.join("a.md"));
    let second = crate::test_dir::tempdir();
    std::fs::write(second.path().join("b.md"), "второй").unwrap();
    remember(&h, second.path());

    open_dialog(&mut h);
    assert_eq!(rows(&h).len(), 2);
    let outcome = h.act(row(&h, second.path()), RowAction::Open);
    assert!(outcome.title);
    assert!(h.app.popup.is_none());
    assert!(same_path(h.app.tree.root().unwrap(), second.path()));
    assert!(h.app.note.is_none());
    assert!(h.app.tabs.active().path.is_none());

    open_dialog(&mut h);
    h.act(row(&h, &first), RowAction::Open);
    assert!(same_path(h.app.tree.root().unwrap(), &first));
    assert!(h.app.shows(&first.join("a.md")));

    open_dialog(&mut h);
    h.escape();
    assert!(h.app.popup.is_none());
    h.app.switch_workspace(&mut h.root, second.path().join(".").join("..").join(second.path().file_name().unwrap()));
    let reopened = Workspace::open(Some(h._data.path()), None, || {}).unwrap();
    assert!(same_path(reopened.root.as_deref().unwrap(), second.path()));
    let known = h.app.workspace.core.ui_state.list_workspaces().unwrap();
    assert_eq!(known.iter().filter(|w| same_path(Path::new(&w.path), second.path())).count(), 1);
}

#[test]
fn sidebar_collapses_smoothly() {
    let mut h = Harness::new();
    let chrome = h.app.chrome.as_ref().unwrap();
    let (toggle, sidebar) = (chrome.toggle_sidebar, chrome.sidebar.id());
    let width = |h: &mut Harness| {
        let _ = h.root.redraw();
        h.root.get_widget(sidebar).unwrap().ctx().bounding_box().width()
    };
    let full = width(&mut h);
    assert!(full > 100.0);
    h.act(toggle, Pressed);
    h.anim(std::time::Duration::from_millis(100));
    let middle = width(&mut h);
    assert!(middle > 0.0 && middle < full, "{middle}");
    h.anim(std::time::Duration::from_millis(150));
    assert_eq!(width(&mut h), 0.0);
    h.act(toggle, Pressed);
    h.anim(std::time::Duration::from_millis(250));
    assert_eq!(width(&mut h), full);
}
