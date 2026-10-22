use super::*;
use crate::ui::widgets::Pressed;
use crate::ui::workspaces::RowAction;
use aquilum_core::search::paths::same_path;

impl Harness {
    pub(super) fn workspace_rows(&self) -> Vec<(masonry::core::WidgetId, std::path::PathBuf)> {
        match &self.app.popup {
            Some(Popup::Workspaces(state)) => state.dialog().rows.iter().map(|(id, path, _)| (*id, path.clone())).collect(),
            _ => Vec::new(),
        }
    }
}

#[test]
fn workspace_dialog_switches_and_forgets_vaults() {
    let mut h = Harness::new();
    let first = h.vault.path().to_path_buf();
    h.open(first.join("a.md"));
    let second = crate::test_dir::tempdir();
    std::fs::write(second.path().join("b.md"), "второй").unwrap();
    h.app.workspace.core.ui_state.resolve_workspace(&second.path().to_string_lossy(), 1).unwrap();

    let button = h.app.chrome.as_ref().unwrap().workspaces;
    h.act(button, Pressed);
    let rows = h.workspace_rows();
    assert_eq!(rows.len(), 2);
    let target = rows.iter().find(|(_, p)| same_path(p, second.path())).unwrap().0;
    let outcome = h.act(target, RowAction::Open);
    assert!(outcome.title);
    assert!(h.app.popup.is_none());
    assert!(same_path(h.app.tree.root().unwrap(), second.path()));
    assert!(h.app.note.is_none());
    assert!(h.app.tabs.active().path.is_none());

    h.act(h.app.chrome.as_ref().unwrap().workspaces, Pressed);
    let back = h.workspace_rows().into_iter().find(|(_, p)| same_path(p, &first)).unwrap().0;
    h.act(back, RowAction::Open);
    assert!(same_path(h.app.tree.root().unwrap(), &first));
    assert!(h.app.shows(&first.join("a.md")));

    h.act(h.app.chrome.as_ref().unwrap().workspaces, Pressed);
    let other = h.workspace_rows().into_iter().find(|(_, p)| same_path(p, second.path())).unwrap().0;
    h.act(other, RowAction::Forget);
    let confirm = match &h.app.popup {
        Some(Popup::Workspaces(state)) => state.forget_layer().unwrap(),
        _ => panic!("нет окна баз"),
    };
    h.act(confirm, crate::ui::dialog::DialogAnswer::Confirm);
    assert_eq!(h.workspace_rows().len(), 1);
    h.escape();
    assert!(h.app.popup.is_none());

    h.act(h.app.chrome.as_ref().unwrap().workspaces, Pressed);
    h.escape();
    h.app.switch_workspace(&mut h.root, second.path().join(".").join("..").join(second.path().file_name().unwrap()));
    let reopened = Workspace::open(Some(h._data.path()), None, || {}).unwrap();
    assert!(same_path(reopened.root.as_deref().unwrap(), second.path()));
    let known = h.app.workspace.core.ui_state.list_workspaces().unwrap();
    assert_eq!(known.iter().filter(|w| same_path(std::path::Path::new(&w.path), second.path())).count(), 1);
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
