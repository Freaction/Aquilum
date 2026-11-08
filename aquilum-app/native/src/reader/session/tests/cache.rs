use super::*;
use crate::reader::Point;
use crate::reader::session::Anchor;

fn epub() -> (Session, tempfile::TempDir) {
    let dir = crate::test_dir::tempdir();
    let file = dir.path().join("book.epub");
    std::fs::write(&file, include_bytes!("../../../../tests/fixtures/reader/scroll-audit.epub")).unwrap();
    (Session::open(&file, Target::Start).unwrap(), dir)
}

#[test]
fn epub_chapters_are_released_and_reloaded_scroll_audit() {
    let (mut session, _dir) = epub();
    assert!(session.book.sections.len() >= 2);
    let (mut fcx, mut lcx) = (FontContext::new(), LayoutContext::new());
    for index in [0, 1, 0, 1, 0] {
        session.enter(index, Anchor::Start);
        session.ensure(VIEW, &settings(18.0, false), &mut fcx, &mut lcx);
        assert!(session.location().is_some());
        assert_eq!(session.book.sections.iter().filter(|s| s.dom.get().is_some()).count(), 1, "DOM of a previous chapter remains loaded");
    }
}

#[test]
fn quotes_of_other_chapters_do_not_load_their_dom_scroll_audit() {
    let (mut session, _dir) = epub();
    let other = session.book.document(1).unwrap();
    let node = (0..other.len() as u32).find(|&node| other.text_of(node).is_some_and(|t| !t.is_empty())).unwrap();
    let cfi = session.book.cfi(1, (node, 0), (node, 1)).unwrap();
    assert!(matches!(session.book.resolve(&cfi), Some((1, Point::At(..), _))));
    session.book.sections[1].dom.take();
    assert!(session.resolve(&cfi).is_none());
    assert!(session.book.sections[1].dom.get().is_none(), "a quote from a hidden chapter decoded its DOM");
}
