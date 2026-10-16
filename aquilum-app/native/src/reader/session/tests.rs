use masonry::kurbo::Size;
use masonry::parley::{FontContext, LayoutContext};

use super::{Session, Spot, Target};
use crate::reader::tests::{book_file, sample_fb2, settings};

const VIEW: Size = Size::new(1200.0, 800.0);

fn open() -> (Session, tempfile::TempDir) {
    let (dir, file) = book_file(sample_fb2());
    (Session::open(&file, Target::Start).unwrap(), dir)
}

fn assert_only_screen(session: &Session) {
    assert!(!session.needs_layout(), "на экране абзац без раскладки");
    let shown = session.on_screen();
    assert!(!shown.is_empty());
    assert!(session.layouts.iter().all(|(item, _)| shown.contains(item)), "держится раскладка абзаца вне экрана");
}

#[test]
fn only_the_page_on_screen_keeps_its_layout() {
    let (mut fcx, mut lcx) = (FontContext::new(), LayoutContext::new());
    for scrolled in [false, true] {
        let (mut session, _dir) = open();
        for _ in 0..30 {
            session.ensure(VIEW, &settings(18.0, scrolled), &mut fcx, &mut lcx);
            assert_only_screen(&session);
            if scrolled {
                session.scroll_by(500.0);
            } else {
                session.next();
            }
        }
    }
}

#[test]
fn a_new_font_size_keeps_the_reading_place() {
    let (mut fcx, mut lcx) = (FontContext::new(), LayoutContext::new());
    let (mut session, _dir) = open();
    session.ensure(VIEW, &settings(18.0, false), &mut fcx, &mut lcx);
    for _ in 0..12 {
        session.next();
        session.ensure(VIEW, &settings(18.0, false), &mut fcx, &mut lcx);
    }
    let (block, _) = session.start().unwrap();
    for size in [26.0, 14.0] {
        session.ensure(VIEW, &settings(size, false), &mut fcx, &mut lcx);
        assert_only_screen(&session);
        let (first, last) = (session.start().unwrap().0, session.end().unwrap().0);
        assert!(first <= block && block <= last, "после смены шрифта страница {first}..{last} без абзаца {block}");
    }
}

#[test]
fn a_selection_over_both_columns_and_beyond_the_screen_makes_a_whole_quote() {
    let (mut fcx, mut lcx) = (FontContext::new(), LayoutContext::new());
    let (mut session, _dir) = open();
    session.ensure(VIEW, &settings(18.0, false), &mut fcx, &mut lcx);
    let shown = session.on_screen();
    let (from, to) = (Spot { block: shown[1], byte: 0 }, Spot { block: *shown.last().unwrap(), byte: 10 });
    let rects = session.rects(from, to);
    assert!(rects.iter().any(|r| r.x0 < VIEW.width / 2.0) && rects.iter().any(|r| r.x0 > VIEW.width / 2.0), "выделение не в обеих колонках");
    let far = Spot { block: 300, byte: "Абзац номер".len() };
    let text = session.text(from, far);
    assert!(text.starts_with("Абзац номер 1.") && text.ends_with("Абзац номер"), "{}", &text[text.len() - 40..]);
    assert!(session.range_cfi(from, far).is_some_and(|cfi| cfi.starts_with("epubcfi(/6/2!")));
}
