use super::*;
use crate::frontmatter::{BOOK_FILE, PAGES, READ_PERCENT, READER_POSITION};
use crate::ui::menu::MenuChoice;
use aquilum_core::ui_state::models::SaveReaderStateInput;

fn book(h: &mut Harness) -> (std::path::PathBuf, std::path::PathBuf, uuid::Uuid) {
    let file = h.vault.path().join("old.fb2");
    std::fs::write(&file, crate::reader::tests::sample_fb2()).unwrap();
    let page = h.write_note("Book.md", &format!("---\ntype: book\n{BOOK_FILE}: old.fb2\npages: 40/80\nreader_position: old-position\nread_percent: 50\n---\nBook notes"));
    h.open(page.clone());
    h.settle();
    let workspace = h.app.workspace.core.ui_state.resolve_workspace(&h.vault.path().to_string_lossy(), 1).unwrap();
    h.app
        .workspace
        .core
        .ui_state
        .save_reader_state(&SaveReaderStateInput { workspace_id: workspace, book_file: "old.fb2".into(), current: 40, cfi: Some("old-position".into()), now_ms: 1 })
        .unwrap();
    (file, page, workspace)
}

#[test]
fn book_file_menu_uses_joined_base_buttons_and_removal_resets_progress() {
    let mut h = Harness::new();
    let (_, _, workspace) = book(&mut h);
    let ids = &h.app.note_ids.as_ref().unwrap().hero.1;
    let (read, menu) = (ids.book_button.unwrap().id(), ids.book_menu.unwrap());
    let left = h.root.get_widget(read).unwrap().ctx().bounding_box();
    let right = h.root.get_widget(menu).unwrap().ctx().bounding_box();
    assert!((left.x1 - right.x0).abs() < 0.1, "split button has a gap");
    assert_eq!(left.height(), right.height());
    assert_eq!(left.height(), crate::ui::widgets::ControlSize::M.fit().height());
    assert!(h.root.get_widget(menu).unwrap().downcast::<crate::ui::widgets::TextButton>().is_some());
    h.snapshot("book-split-button.png");
    h.click(right.center().x, right.center().y);
    let Some(Popup::Menu { layer, commands }) = &h.app.popup else { panic!("book menu did not open") };
    assert_eq!(commands.len(), 2);
    let layer = *layer;
    h.snapshot("book-file-menu.png");
    h.act(layer, MenuChoice::Item(1));
    let fields = h.app.fields().unwrap();
    for key in [BOOK_FILE, PAGES, READER_POSITION, READ_PERCENT] {
        assert!(fields.text(key).is_none(), "{key} survived removal");
    }
    assert!(h.app.workspace.core.ui_state.load_reader_state(workspace, "old.fb2").unwrap().is_none());
    assert!(h.app.note_ids.as_ref().unwrap().hero.1.book_menu.is_some(), "upload state lost the dropdown");
}

#[test]
fn book_file_replacement_resets_progress_but_picker_cancellation_keeps_it() {
    let mut h = Harness::new();
    let (_, page, workspace) = book(&mut h);
    h.app.on_picked(&mut h.root, crate::app::book::Picked::Book(page.clone(), None));
    assert_eq!(h.app.fields().unwrap().text(PAGES), Some("40/80"));
    assert!(h.app.workspace.core.ui_state.load_reader_state(workspace, "old.fb2").unwrap().is_some());
    let new_file = h.vault.path().join("new.pdf");
    std::fs::write(&new_file, crate::reader::pdf::tests::sample_pdf()).unwrap();
    h.app.on_picked(&mut h.root, crate::app::book::Picked::Book(page, Some(new_file)));
    let fields = h.app.fields().unwrap();
    assert!(fields.text(BOOK_FILE).unwrap().ends_with("new.pdf"));
    assert!(fields.text(PAGES).is_none());
    assert!(fields.text(READER_POSITION).is_none());
    assert_eq!(fields.text(READ_PERCENT), Some("0"));
    assert!(h.app.workspace.core.ui_state.load_reader_state(workspace, "old.fb2").unwrap().is_none());
    h.snapshot("book-file-replaced.png");
}

#[test]
fn book_file_lifecycle_persists_for_every_supported_extension() {
    for extension in crate::app::book::BOOK_EXTENSIONS {
        let mut h = Harness::new();
        let (_, page, workspace) = book(&mut h);
        let new_file = h.vault.path().join(format!("replacement.{extension}"));
        std::fs::write(&new_file, crate::reader::tests::sample_book(extension)).unwrap();
        if extension != "pdf" {
            assert!(crate::reader::Book::open(&new_file).is_some(), "invalid {extension} fixture");
        }
        h.app.on_picked(&mut h.root, crate::app::book::Picked::Book(page.clone(), Some(new_file)));
        h.save_now();
        let replaced = std::fs::read_to_string(&page).unwrap();
        let fields = crate::frontmatter::parse(&replaced).unwrap();
        let relative = fields.text(BOOK_FILE).unwrap().to_owned();
        assert!(relative.ends_with(&format!("replacement.{extension}")), "{extension}: {replaced}");
        assert_eq!(fields.text(READ_PERCENT), Some("0"));
        assert!(fields.text(READER_POSITION).is_none());
        if extension != "pdf" {
            assert_eq!(crate::frontmatter::parse_pages(fields.text(PAGES).unwrap()).unwrap().0, 0);
        }
        assert!(h.app.workspace.core.ui_state.load_reader_state(workspace, "old.fb2").unwrap().is_none());
        let mut h = h.restarted(1.0);
        h.open(page.clone());
        assert_eq!(h.app.fields().unwrap(), fields, "{extension}: replacement was not persisted");
        h.app
            .workspace
            .core
            .ui_state
            .save_reader_state(&SaveReaderStateInput { workspace_id: workspace, book_file: relative.clone(), current: 10, cfi: Some("old-position".into()), now_ms: 2 })
            .unwrap();
        h.app.book_command(&mut h.root, crate::app::book::BookCommand::Remove);
        h.save_now();
        let removed = std::fs::read_to_string(&page).unwrap();
        for key in [BOOK_FILE, PAGES, READ_PERCENT, READER_POSITION] {
            assert!(!removed.lines().any(|line| line.starts_with(&format!("{key}:"))), "{extension}: metadata row survived removal: {removed}");
        }
        assert!(removed.ends_with("Book notes"), "{extension}: removal changed note text");
        assert!(h.app.workspace.core.ui_state.load_reader_state(workspace, &relative).unwrap().is_none());
        let mut h = h.restarted(1.0);
        h.open(page);
        for key in [BOOK_FILE, PAGES, READ_PERCENT, READER_POSITION] {
            assert!(h.app.fields().unwrap().text(key).is_none(), "{extension}: {key} returned after restart");
        }
    }
}

#[test]
fn failed_book_file_import_does_not_reset_the_previous_book() {
    let mut h = Harness::new();
    let (_, page, workspace) = book(&mut h);
    let missing = h.vault.path().join("missing.pdf");
    h.app.on_picked(&mut h.root, crate::app::book::Picked::Book(page, Some(missing)));
    let fields = h.app.fields().unwrap();
    assert_eq!(fields.text(BOOK_FILE), Some("old.fb2"));
    assert_eq!(fields.text(PAGES), Some("40/80"));
    assert_eq!(fields.text(READER_POSITION), Some("old-position"));
    assert_eq!(fields.text(READ_PERCENT), Some("50"));
    assert!(h.app.workspace.core.ui_state.load_reader_state(workspace, "old.fb2").unwrap().is_some());
}

#[test]
fn legacy_book_file_alias_is_updated_and_removed_without_duplicate_metadata_rows() {
    let mut h = Harness::new();
    let (_, page, _) = book(&mut h);
    let legacy = h.app.text.to_string().replace(BOOK_FILE, "book_file");
    h.app.commit_text(&mut h.root, crop::Rope::from(legacy));
    let new_file = h.vault.path().join("new.epub");
    std::fs::write(&new_file, crate::reader::tests::sample_book("epub")).unwrap();
    h.app.on_picked(&mut h.root, crate::app::book::Picked::Book(page.clone(), Some(new_file)));
    h.save_now();
    let updated = std::fs::read_to_string(&page).unwrap();
    assert_eq!(updated.lines().filter(|line| line.starts_with("book_file:") || line.starts_with(&format!("{BOOK_FILE}:"))).count(), 1);
    assert!(h.app.fields().unwrap().text(BOOK_FILE).unwrap().ends_with("new.epub"));
    h.app.book_command(&mut h.root, crate::app::book::BookCommand::Remove);
    h.save_now();
    let removed = std::fs::read_to_string(page).unwrap();
    assert!(!removed.contains("book_file:"));
    assert!(!removed.contains(&format!("{BOOK_FILE}:")));
}
