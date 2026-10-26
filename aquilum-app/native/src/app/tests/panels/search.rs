use super::*;

impl Harness {
    pub(super) fn type_search(&mut self, text: &str) {
        let field = match &self.app.popup {
            Some(Popup::Search(state)) => state.field_id(),
            _ => panic!("поиск не открыт"),
        };
        self.send(field, crate::ui::editor::TextAction::Changed(text.into()));
    }

    fn wait_indexed(&mut self, documents: u64) {
        let vault = self.vault.path().to_string_lossy().into_owned();
        self.app.start_index();
        wait_until(250, 20, || {
            let status = self.app.workspace.core.search.status(Some(&vault));
            status.state == aquilum_core::search::models::SearchIndexState::Ready && !status.updating && status.indexed_documents >= documents
        });
    }

    pub(super) fn wait_results(&mut self, at_least: usize) -> usize {
        let count = |h: &Harness| match &h.app.popup {
            Some(Popup::Search(state)) => state.result_count(),
            _ => 0,
        };
        let found = wait_until(250, 20, || {
            self.app.on_core_events(&mut self.root);
            count(self) >= at_least
        });
        assert!(found, "поиск не дал результатов");
        count(self)
    }
}

#[test]
fn search_finds_opens_and_creates_notes() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    h.write_note("Кофе.md", "Заметка про обжарку кофе");
    h.write_note("Чай.md", "Кофе тоже упомянут");
    h.wait_indexed(3);
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

#[test]
fn an_unsaved_edit_is_found_as_soon_as_the_search_opens() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_string_lossy().into_owned();
    let note = h.write_note("Дневник.md", "Начало дня");
    h.app.start_index();
    wait_until(250, 20, || {
        let status = h.app.workspace.core.search.status(Some(&vault));
        status.state == aquilum_core::search::models::SearchIndexState::Ready && !status.updating && status.indexed_documents >= 1
    });
    h.app.open(&mut h.root, note);
    h.parked();
    let body = h.app.note_ids.as_ref().unwrap().body;
    h.root.focus_on(Some(body.id()));
    let end = body.get(&h.root).map_or(0, |b| b.inner().len());
    body.edit(&mut h.root, |mut b| crate::ui::editor::TextEditor::select_byte_range(&mut b, end, end));
    h.type_text(" щукаплывёт");

    h.app.open_search(&mut h.root);
    h.type_search("щукаплывёт");
    assert_eq!(h.wait_results(1), 1, "правка ищется сразу, без ожидания автосохранения");
}

#[test]
fn a_word_still_being_composed_already_searches() {
    let mut h = Harness::new();
    h.write_note("Кофе.md", "Заметка про обжарку кофе");
    h.wait_indexed(2);
    h.app.open_search(&mut h.root);
    let field = match &h.app.popup {
        Some(Popup::Search(state)) => state.field_id(),
        _ => panic!("поиск не открыт"),
    };
    h.root.focus_on(Some(field));
    h.root.handle_text_event(masonry::core::TextEvent::Ime(masonry::core::Ime::Preedit("кофе".into(), Some((8, 8)))));
    h.deliver();
    assert_eq!(h.wait_results(1), 1);
}
