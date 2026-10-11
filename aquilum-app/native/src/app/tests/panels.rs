use super::*;

impl Harness {
    pub(super) fn wait_listing(&mut self) -> crate::links::Listing {
        for _ in 0..250 {
            self.app.on_core_events(&mut self.root);
            if let Some(listing) = self.app.listing.as_ref().filter(|l| l.generation == self.app.panel_generation) {
                return listing.clone();
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("панель не получила ответ");
    }
}

#[test]
fn backlinks_panel_lists_and_opens_mentions() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("a.md"), "Ссылка на [[b]]").unwrap();
    std::fs::write(vault.join("b.md"), "Текст").unwrap();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("b.md")]);
    h.app.open(&mut h.root, vault.join("b.md"));
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
    let open = |new_tab| masonry::core::ErasedAction::from(Box::new(crate::ui::widgets::ItemAction::Open { new_tab }));
    h.app.on_action(&mut h.root, item, open(true), Size::new(1000.0, 600.0));
    assert_eq!(h.tab_paths(), [Some("b".to_owned()), Some("a".to_owned())]);

    let outgoing = h.app.chrome.as_ref().unwrap().panel_parts.modes[1].0.id();
    let pressed = || masonry::core::ErasedAction::from(Box::new(crate::ui::widgets::Pressed));
    h.app.on_action(&mut h.root, outgoing, pressed(), Size::new(1000.0, 600.0));
    let listing = h.wait_listing();
    assert_eq!(listing.items.len(), 1);
    assert!(matches!(&listing.items[0].target, crate::links::Target::Note(p, _) if p.ends_with("b.md")));

    let toggle = h.app.chrome.as_ref().unwrap().panel_toggle;
    h.app.on_action(&mut h.root, toggle, pressed(), Size::new(1000.0, 600.0));
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
    h.app.open(&mut h.root, note.clone());
    for (text, source) in [("один\nдва\n", Source::Me), ("один\nдва\nтри\n", Source::Agent)] {
        let hash = read_file_snapshot_impl(&note).unwrap().hash;
        gate::write(&h.app.workspace.core, &note, text, Some(&hash), source, None).unwrap();
    }
    let history_button = h.app.chrome.as_ref().unwrap().panel_parts.modes.last().unwrap().0.id();
    let pressed = || masonry::core::ErasedAction::from(Box::new(crate::ui::widgets::Pressed));
    h.app.on_action(&mut h.root, history_button, pressed(), Size::new(1000.0, 600.0));
    let rows = h.app.history_body.as_ref().expect("список версий").rows.clone();
    assert!(rows.len() >= 2, "строк версий: {}", rows.len());

    let versions = h.app.history.as_ref().unwrap().versions.clone();
    let agent = versions.iter().position(|v| v.source == Source::Agent).expect("версия агента");
    let row = rows.iter().find(|(_, index)| *index == agent).unwrap().0;
    let open = masonry::core::ErasedAction::from(Box::new(crate::ui::widgets::ItemAction::Open { new_tab: false }));
    h.app.on_action(&mut h.root, row, open, Size::new(1000.0, 600.0));
    let view = h.app.version_view.as_ref().expect("версия открыта");
    let revert = view.revert.expect("у версии есть предыдущая");

    h.app.on_action(&mut h.root, revert, pressed(), Size::new(1000.0, 600.0));
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "один\nдва\n");
    assert!(h.app.version_view.is_none());
    assert_eq!(h.app.editor_text, "один\nдва\n");
}

impl Harness {
    pub(super) fn type_search(&mut self, text: &str) {
        let field = match &self.app.popup {
            Some(Popup::Search(state)) => state.field_id(),
            _ => panic!("поиск не открыт"),
        };
        let changed = masonry::core::ErasedAction::from(Box::new(crate::ui::editor::TextAction::Changed(text.into())));
        self.app.on_action(&mut self.root, field, changed, Size::new(1000.0, 600.0));
    }

    pub(super) fn wait_results(&mut self, at_least: usize) -> usize {
        for _ in 0..250 {
            self.app.on_core_events(&mut self.root);
            if let Some(Popup::Search(state)) = &self.app.popup
                && state.result_count() >= at_least
            {
                return state.result_count();
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("поиск не дал результатов");
    }
}

#[test]
fn search_finds_opens_and_creates_notes() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("Кофе.md"), "Заметка про обжарку кофе").unwrap();
    std::fs::write(vault.join("Чай.md"), "Кофе тоже упомянут").unwrap();
    h.app.start_index();
    for _ in 0..250 {
        let status = h.app.workspace.core.search.status(Some(&vault.to_string_lossy()));
        if status.state == aquilum_core::search::models::SearchIndexState::Ready && !status.updating && status.indexed_documents >= 3 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    h.app.open_search(&mut h.root);
    h.type_search("кофе");
    assert_eq!(h.wait_results(2), 2);
    h.app.search_key(&mut h.root, &Key::Named(NamedKey::ArrowDown), Modifiers::empty());
    let second = match &h.app.popup {
        Some(Popup::Search(state)) => state.selected_path(),
        _ => unreachable!(),
    };
    h.app.search_key(&mut h.root, &Key::Named(NamedKey::Enter), Modifiers::empty());
    assert!(h.app.popup.is_none());
    assert_eq!(h.app.tabs.active().path.as_ref(), Some(&second));

    h.app.open_search(&mut h.root);
    h.type_search("Новая идея");
    h.app.search_key(&mut h.root, &Key::Named(NamedKey::Enter), Modifiers::SHIFT);
    assert!(vault.join("Новая идея.md").exists());
    assert_eq!(h.app.tabs.active().path.as_ref(), Some(&vault.join("Новая идея.md")));
}

impl Harness {
    pub(super) fn workspace_rows(&self) -> Vec<(masonry::core::WidgetId, std::path::PathBuf)> {
        match &self.app.popup {
            Some(Popup::Workspaces(state)) => state.dialog().rows.iter().map(|(id, path, _)| (*id, path.clone())).collect(),
            _ => Vec::new(),
        }
    }

    pub(super) fn act(&mut self, id: masonry::core::WidgetId, action: impl std::any::Any + std::fmt::Debug + Send) -> Outcome {
        let outcome = self.app.on_action(&mut self.root, id, masonry::core::ErasedAction::from(Box::new(action)), Size::new(1000.0, 600.0));
        let _ = self.root.redraw();
        outcome
    }
}

#[test]
fn workspace_dialog_switches_and_forgets_vaults() {
    use crate::ui::workspaces::{RowAction, same_path};
    let mut h = Harness::new();
    let first = h.vault.path().to_path_buf();
    h.app.open(&mut h.root, first.join("a.md"));
    let second = tempfile::tempdir().unwrap();
    std::fs::write(second.path().join("b.md"), "второй").unwrap();
    h.app.workspace.core.ui_state.resolve_workspace(&second.path().to_string_lossy(), 1).unwrap();

    let button = h.app.chrome.as_ref().unwrap().workspaces;
    h.act(button, crate::ui::widgets::Pressed);
    let rows = h.workspace_rows();
    assert_eq!(rows.len(), 2);
    let target = rows.iter().find(|(_, p)| same_path(p, second.path())).unwrap().0;
    let outcome = h.act(target, RowAction::Open);
    assert!(outcome.title);
    assert!(h.app.popup.is_none());
    assert!(same_path(h.app.tree.root().unwrap(), second.path()));
    assert!(h.app.note.is_none());
    assert!(h.app.tabs.active().path.is_none());

    h.act(h.app.chrome.as_ref().unwrap().workspaces, crate::ui::widgets::Pressed);
    let back = h.workspace_rows().into_iter().find(|(_, p)| same_path(p, &first)).unwrap().0;
    h.act(back, RowAction::Open);
    assert!(same_path(h.app.tree.root().unwrap(), &first));
    assert!(h.app.note.as_ref().is_some_and(|n| n.is(&first.join("a.md"))));

    h.act(h.app.chrome.as_ref().unwrap().workspaces, crate::ui::widgets::Pressed);
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

    h.act(h.app.chrome.as_ref().unwrap().workspaces, crate::ui::widgets::Pressed);
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
    h.act(toggle, crate::ui::widgets::Pressed);
    h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_millis(100)));
    let middle = width(&mut h);
    assert!(middle > 0.0 && middle < full, "{middle}");
    h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_millis(150)));
    assert_eq!(width(&mut h), 0.0);
    h.act(toggle, crate::ui::widgets::Pressed);
    h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_millis(250)));
    assert_eq!(width(&mut h), full);
}
