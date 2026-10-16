use super::super::markdown::Block;
use super::Document;
use super::tests::markdown;

fn lines(n: usize) -> String {
    (0..n).map(|i| format!("строка {i}\n")).collect()
}

fn windowed(text: &str) -> Document {
    let mut doc = Document::windowed(crop::Rope::from(text));
    doc.set_markdown(markdown("").markdown.take());
    doc
}

fn agrees(doc: &Document, text: &str) {
    let (base, window) = doc.loaded();
    assert_eq!(doc.whole(), text);
    assert_eq!(window, &text[base..base + window.len()]);
}

#[test]
fn a_far_position_moves_the_window_instead_of_stretching_it() {
    let text = lines(50_000);
    let mut doc = windowed(&text);
    let size = doc.loaded().1.len();
    assert!(size < text.len() / 4);
    doc.include(text.len() - 3);
    let (base, end) = doc.loaded_range();
    assert!(base <= text.len() - 3 && text.len() - 3 <= end);
    assert!(end - base <= size * 2, "окно растянулось: {} байт", end - base);
    agrees(&doc, &text);
}

#[test]
fn edits_outside_the_window_land_in_the_text() {
    let mut text = lines(50_000);
    let mut doc = windowed(&text);
    doc.include(text.len() / 2);
    let edits: [fn(&str) -> (std::ops::Range<usize>, &'static str); 3] = [|_| (0..0, "до\n"), |t| (0..t.find('\n').unwrap_or(0) + 1, ""), |t| (t.len()..t.len(), "после")];
    for edit in edits {
        let (range, insert) = edit(&text);
        doc.replace(range.clone(), insert);
        text.replace_range(range, insert);
        agrees(&doc, &text);
    }
    let end = text.len();
    doc.replace(0..end, "всё");
    agrees(&doc, "всё");
}

#[test]
fn a_window_inside_a_long_code_block_knows_it_is_code() {
    let text = format!("```\n{}```\nтекст\n", lines(20_000));
    let mut doc = windowed(&text);
    doc.include(text.len() / 2);
    let code = (0..300).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(10));
        doc.track((0.0, 0.0));
        doc.block(0).is_code()
    });
    assert!(code, "окно в середине блока кода разобрано как текст: {:?}", doc.block(0));
    assert!(matches!(doc.block(doc.line_count() - 1), Block::Code { .. } | Block::Text));
}

#[test]
fn lines_in_view_stay_laid_when_the_window_starts_below_the_top() {
    use masonry::parley::{FontContext, LayoutContext};
    let text = lines(50_000);
    let mut doc = windowed(&text);
    let (mut fcx, mut lcx, styles) = (FontContext::default(), LayoutContext::new(), masonry::core::StyleSet::new(16.0));
    doc.layout(&mut fcx, &mut lcx, &styles, Some(600.0), None, None);
    doc.include(text.len() / 2);
    let y = doc.y_of(text.len() / 2);
    doc.keep = (y - 400.0, y + 400.0);
    doc.layout(&mut fcx, &mut lcx, &styles, Some(600.0), None, None);
    let line = doc.at_y(y);
    assert!(doc.above > 0.0, "окно не ушло от начала заметки");
    assert!(doc.laid.get(line).is_some(), "видимая строка выброшена из раскладки: keep в координатах заметки, высоты — в координатах окна");
}
