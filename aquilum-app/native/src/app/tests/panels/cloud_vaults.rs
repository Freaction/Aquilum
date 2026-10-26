use super::*;
use crate::app::workspaces::cloud::CloudEvent;
use crate::i18n::{t, t_with};
use crate::ui::widgets::{Label, Pressed};
use aquilum_core::search::paths::same_path;
use aquilum_core::settings::models::SyncSettings;
use aquilum_core::sync::SyncStatus;
use masonry::core::{Widget, WidgetRef};

fn sign_in(h: &Harness) {
    let sync = h._data.path().join("sync");
    std::fs::create_dir_all(&sync).unwrap();
    std::fs::write(sync.join("dropbox.json"), "{}").unwrap();
}

fn state(h: &Harness) -> &crate::app::workspaces::WorkspacesState {
    match &h.app.popup {
        Some(Popup::Workspaces(state)) => state,
        _ => panic!("окно баз не открыто"),
    }
}

fn texts(widget: WidgetRef<'_, dyn Widget>) -> Vec<String> {
    let own = widget.downcast::<Label>().map(|label| label.inner().text().to_owned());
    own.into_iter().chain(widget.children().into_iter().flat_map(texts)).collect()
}

fn cloud_texts(h: &Harness) -> Vec<String> {
    let slot = state(h).dialog().cloud.expect("раздел «В облаке» не показан");
    texts(h.root.get_widget(slot.id()).unwrap())
}

fn open_workspaces(h: &mut Harness) {
    let button = h.app.chrome.as_ref().unwrap().workspaces;
    h.act(button, Pressed);
}

#[test]
fn without_dropbox_the_dialog_has_no_cloud_section() {
    let mut h = Harness::new();
    open_workspaces(&mut h);

    assert!(state(&h).dialog().cloud.is_none());
    assert!(state(&h).cloud_events().is_none());
}

#[test]
fn a_vault_from_the_cloud_is_created_opened_and_synced() {
    let places = crate::test_dir::tempdir();
    let mut h = Harness::new();
    sign_in(&h);
    let here = places.path().join("NeuroNet");
    std::fs::create_dir(&here).unwrap();
    h.app.workspace.core.ui_state.resolve_workspace(&here.to_string_lossy(), 1).unwrap();
    open_workspaces(&mut h);
    assert_eq!(cloud_texts(&h), [t("workspaces.cloud"), t("workspaces.cloudLoading")]);
    let failed = t_with("workspaces.cloudFailed", &[("reason", "")]);
    let shown = wait_until(500, 10, || {
        h.app.on_core_events(&mut h.root);
        cloud_texts(&h).iter().any(|text| text.starts_with(&failed))
    });
    assert!(shown, "ошибка списка не показана: {:?}", cloud_texts(&h));

    let events = state(&h).cloud_events().unwrap();
    events.send(CloudEvent::Listed(Ok(vec!["Aquilum".into(), "neuroNET".into(), "Работа".into()]))).unwrap();
    h.app.on_core_events(&mut h.root);

    let names: Vec<String> = state(&h).cloud_rows().into_iter().map(|(_, name)| name).collect();
    assert_eq!(names, ["Aquilum", "Работа"]);
    assert_eq!(cloud_texts(&h), [t("workspaces.cloud")]);
    h.anim(std::time::Duration::from_millis(300));
    h.snapshot("cloud-vaults.png");
    let target = places.path().join("Работа");
    events.send(CloudEvent::Chosen(target.clone())).unwrap();
    assert!(h.app.on_core_events(&mut h.root));

    assert!(target.is_dir());
    assert!(same_path(h.app.tree.root().unwrap(), &target));
    assert!(h.app.popup.is_none());
    let sync = h.app.workspace.core.ui_state.workspace(&target).unwrap().sync;
    assert_eq!(sync, SyncSettings { enabled: true, ..SyncSettings::default() });
    let core = std::sync::Arc::clone(&h.app.workspace.core);
    assert!(wait_until(500, 10, || matches!(core.sync.status(), SyncStatus::Failed(_))), "проход для новой базы не запущен: {:?}", core.sync.status());
}
