use super::*;
use crate::frontmatter::{BOOK_FILE, PAGES, READ_PERCENT, READER_POSITION};
use crate::ui::reader::ReaderView;

fn position(h: &Harness) -> (u32, String, f64) {
    let Some(Popup::Reader(state)) = &h.app.popup else { panic!("reader did not open") };
    h.root.get_widget(state.layer).unwrap().downcast::<ReaderView>().unwrap().inner().position().unwrap()
}

#[test]
fn every_book_format_saves_and_resumes_after_application_restart() {
    for extension in crate::app::book::BOOK_EXTENSIONS {
        let mut h = Harness::new();
        let file = h.vault.path().join(format!("book.{extension}"));
        std::fs::write(&file, crate::reader::tests::sample_book(extension)).unwrap();
        let page = h.write_note("Book.md", &format!("---\ntype: book\n{BOOK_FILE}: book.{extension}\npages: 0/1\n---\nNotes"));
        h.open(page.clone());
        open_book(&mut h, &file, Some(&page), "Book");
        if extension == "pdf" {
            super::pdf::wait_pdf(&mut h);
        }
        turn_pages(&mut h, 4);
        let before = position(&h);
        assert!(before.2 > 0.0, "{extension}: reading did not advance");
        h.app.save_reader(&mut h.root);
        h.save_now();
        let saved = std::fs::read_to_string(&page).unwrap();
        let fields = crate::frontmatter::parse(&saved).unwrap();
        assert_eq!(fields.text(READER_POSITION), Some(before.1.as_str()), "{extension}: position was not saved");
        let current = crate::frontmatter::parse_pages(fields.text(PAGES).unwrap()).unwrap().0;
        assert!(current > 0, "{extension}: page progress was lost");
        assert!(fields.text(READ_PERCENT).unwrap().parse::<u32>().unwrap() > 0);
        h.escape();
        let mut h = h.restarted(1.0);
        h.open(page.clone());
        open_book(&mut h, &file, Some(&page), "Book");
        if extension == "pdf" {
            super::pdf::wait_pdf(&mut h);
        }
        assert_eq!(position(&h).1, before.1, "{extension}: last position changed after restart");
        h.escape();
    }
}
