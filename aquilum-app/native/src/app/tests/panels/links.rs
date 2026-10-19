use super::*;
use crate::ui::widgets::{ItemAction, Pressed};

impl Harness {
    pub(super) fn wait_listing(&mut self) -> crate::links::Listing {
        let ready = wait_until(250, 20, || {
            self.app.on_core_events(&mut self.root);
            self.app.listing.as_ref().is_some_and(|l| l.generation == self.app.panel_generation)
        });
        assert!(ready, "панель не получила ответ");
        self.app.listing.clone().unwrap()
    }
}

#[test]
fn backlinks_panel_lists_and_opens_mentions() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    h.write_note("a.md", "Ссылка на [[b]]");
    h.write_note("b.md", "Текст");
    h.app.tree.paths_changed(&mut h.root, &[vault.join("b.md")]);
    h.open(vault.join("b.md"));
    let mut listing = h.wait_listing();
    for _ in 0..50 {
        if !listing.items.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
        h.app.request_panel(&mut h.root);
        listing = h.wait_listing();
    }
    assert_eq!(listing.items.len(), 1);
    assert_eq!(listing.items[0].title, "a");

    let item = h.app.panel_body.as_ref().unwrap().items[0].0;
    h.send(item, ItemAction::Open { new_tab: true });
    assert_eq!(h.tab_paths(), [Some("b".to_owned()), Some("a".to_owned())]);

    let outgoing = h.app.chrome.as_ref().unwrap().panel_parts.modes[1].0.id();
    h.send(outgoing, Pressed);
    let listing = h.wait_listing();
    assert_eq!(listing.items.len(), 1);
    assert!(matches!(&listing.items[0].target, crate::links::Target::Note(p, _) if p.ends_with("b.md")));

    let toggle = h.app.chrome.as_ref().unwrap().panel_toggle;
    h.send(toggle, Pressed);
    assert!(!h.app.interface.right_open());
    assert!(h.app.listing.is_none());
}

#[test]
fn history_panel_opens_version_and_reverts_it() {
    use aquilum_core::files::document::read_file_snapshot_impl;
    use aquilum_core::files::gate;
    use aquilum_core::history::Source;

    let mut h = Harness::new();
    let note = h.vault.path().join("a.md");
    h.open(note.clone());
    for (text, source) in [("один\nдва\n", Source::Me), ("один\nдва\nтри\n", Source::Agent)] {
        let hash = read_file_snapshot_impl(&note).unwrap().hash;
        gate::write(&h.app.workspace.core, &note, text, Some(&hash), source, None).unwrap();
    }
    let history_button = h.app.chrome.as_ref().unwrap().panel_parts.modes.last().unwrap().0.id();
    h.send(history_button, Pressed);
    let rows = h.app.history_body.as_ref().expect("список версий").rows.clone();
    assert!(rows.len() >= 2, "строк версий: {}", rows.len());

    let versions = h.app.history.as_ref().unwrap().versions.clone();
    let agent = versions.iter().position(|v| v.source == Source::Agent).expect("версия агента");
    let row = rows.iter().find(|(_, index)| *index == agent).unwrap().0;
    h.send(row, ItemAction::Open { new_tab: false });
    let view = h.app.version_view.as_ref().expect("версия открыта");
    let revert = view.revert.expect("у версии есть предыдущая");

    h.send(revert, Pressed);
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "один\nдва\n");
    assert!(h.app.version_view.is_none());
    assert_eq!(h.app.text.to_string(), "один\nдва\n");
}
