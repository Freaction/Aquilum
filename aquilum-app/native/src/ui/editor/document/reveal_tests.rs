use masonry::core::StyleSet;
use masonry::parley::{FontContext, LayoutContext};

use super::super::embed::panel::PAD;
use super::super::markdown::x_at;
use super::tests::markdown;
use super::{Document, Fold};

#[test]
fn a_formula_being_edited_shows_plain_source_with_the_preview_below() {
    let mut doc = markdown("$$\nx^2\n$$\nтекст");
    doc.keep = (0.0, 10_000.0);
    let (mut fcx, mut lcx, styles) = (FontContext::default(), LayoutContext::new(), StyleSet::new(16.0));
    let mut folds = |doc: &mut Document, caret: Option<usize>| {
        doc.layout(&mut fcx, &mut lcx, &styles, Some(600.0), None, caret.map(|c| c..c));
        doc.paragraphs().map(|p| p.fold).collect::<Vec<_>>()
    };
    let preview = [Fold::Head, Fold::Member, Fold::Member, Fold::None];
    assert_eq!(folds(&mut doc, None), preview);
    assert_eq!(folds(&mut doc, Some(4)), [Fold::None, Fold::None, Fold::Below, Fold::None]);
    let body = doc.paragraph(1).decor();
    assert!(!body.nowrap && body.x == PAD, "исходник формулы оформлен как код: перенос {}, отступ {}", !body.nowrap, body.x);
    assert!(doc.paragraph(0).embed.is_some() && doc.bottom(2) - doc.top(2) > doc.heights.text(2) + 10.0, "под исходником нет предпросмотра");
    let key = |doc: &Document| doc.paragraph(0).embed.map(|e| e.key);
    let typed = key(&doc);
    doc.replace(6..6, "+y");
    assert_eq!(folds(&mut doc, Some(8))[2], Fold::Below);
    assert_ne!(key(&doc), typed, "предпросмотр не обновился при наборе");
    assert_eq!(folds(&mut doc, None), preview);
}

#[test]
fn a_code_block_keeps_its_height_and_closing_line_when_entered() {
    let mut doc = markdown("```rust\nlet x = 1;\n```\nтекст");
    doc.keep = (0.0, 10_000.0);
    let (mut fcx, mut lcx, styles) = (FontContext::default(), LayoutContext::new(), StyleSet::new(16.0));
    let mut heights = |doc: &mut Document, caret: Option<usize>| {
        doc.layout(&mut fcx, &mut lcx, &styles, Some(600.0), None, caret.map(|c| c..c));
        (0..3).map(|i| doc.bottom(i) - doc.top(i)).collect::<Vec<_>>()
    };
    let outside = heights(&mut doc, None);
    let line = doc.markdown.as_ref().map_or(0.0, |fonts| f64::from(fonts.line));
    assert!(outside[2] >= line + PAD - 0.5, "нижнее ограждение схлопнулось: {outside:?}");
    assert_eq!(heights(&mut doc, Some(9)), outside, "блок кода меняет высоту при входе каретки");
}

#[test]
fn a_caret_moving_through_plain_words_keeps_the_paragraph_layout() {
    let mut doc = markdown("слово **жирное** слово");
    doc.keep = (0.0, 10_000.0);
    let (mut fcx, mut lcx, styles) = (FontContext::default(), LayoutContext::new(), StyleSet::new(16.0));
    let mut laid_for = |doc: &mut Document, caret: usize| {
        doc.layout(&mut fcx, &mut lcx, &styles, Some(600.0), None, Some(caret..caret));
        doc.paragraph(0).decor().selection.clone()
    };
    assert_eq!(laid_for(&mut doc, 2), Some(2..2));
    assert_eq!(laid_for(&mut doc, 6), Some(2..2), "каретка в простом слове пересобрала абзац");
    let bold = "слово ".len();
    assert_eq!(laid_for(&mut doc, bold), Some(bold..bold), "каретка у разметки её не раскрыла");
}

#[test]
fn a_selection_reveals_every_element_it_touches() {
    let text = "**жирный** текст\n```\nкод\n```\n> [!note] Заголовок\n> тело\n**конец**";
    let mut doc = markdown(text);
    doc.keep = (0.0, 10_000.0);
    let (mut fcx, mut lcx, styles) = (FontContext::default(), LayoutContext::new(), StyleSet::new(16.0));
    let mut width = |doc: &mut Document, selection: Option<std::ops::Range<usize>>| {
        doc.layout(&mut fcx, &mut lcx, &styles, Some(600.0), None, selection);
        [0, 6].map(|i| doc.paragraph(i).layout().width())
    };
    let hidden = width(&mut doc, Some(19..19));
    assert!(!doc.inside(2) && !doc.inside(4), "каретка вне блоков раскрыла их");
    let tail = text.find("тело").unwrap();
    let shown = width(&mut doc, Some(4..tail));
    assert!((0..=5).all(|i| doc.inside(i)), "выделение не раскрыло блок кода и выноску");
    assert!(shown[0] > hidden[0], "звёздочки под выделением не показались");
    assert_eq!(shown[1], hidden[1], "строка вне выделения раскрылась");
}

#[test]
fn a_setext_underline_shows_only_while_its_heading_is_touched() {
    let text = "Заголовок\n=====\nтекст";
    let mut doc = markdown(text);
    doc.keep = (0.0, 10_000.0);
    let (mut fcx, mut lcx, styles) = (FontContext::default(), LayoutContext::new(), StyleSet::new(16.0));
    let mut syntax = |doc: &mut Document, caret: usize| {
        doc.layout(&mut fcx, &mut lcx, &styles, Some(600.0), None, Some(caret..caret));
        doc.paragraph(1).layout().lines().flat_map(|line| line.items().collect::<Vec<_>>()).any(|item| matches!(item, masonry::parley::PositionedLayoutItem::GlyphRun(run) if run.style().brush == super::super::markdown::SYNTAX))
    };
    let end = text.len();
    assert!(!syntax(&mut doc, end), "подчёркивание видно, когда каретка ниже заголовка");
    assert!(syntax(&mut doc, 2), "подчёркивание не проявилось с кареткой в тексте заголовка");
    assert!(syntax(&mut doc, text.find('=').unwrap()), "подчёркивание не проявилось с кареткой на нём");
    let height = doc.bottom(1) - doc.top(1);
    syntax(&mut doc, end);
    assert_eq!(doc.bottom(1) - doc.top(1), height, "строка подчёркивания меняет высоту");
}

#[test]
fn quote_markers_stay_by_their_bars_in_list_items_and_definitions() {
    let text = "> текст\n> - пункт\n>   - вложенный\n> > текст\n> > - пункт\nтермин\n> : определение";
    let mut doc = markdown(text);
    doc.keep = (0.0, 10_000.0);
    let (mut fcx, mut lcx, styles) = (FontContext::default(), LayoutContext::new(), StyleSet::new(16.0));
    doc.layout(&mut fcx, &mut lcx, &styles, Some(600.0), None, Some(0..text.len()));
    let x = |doc: &Document, line: usize, at: usize| doc.origin(line).x + x_at(doc.paragraph(line).layout(), at);
    for line in [1, 2, 3, 4, 6] {
        assert_eq!(x(&doc, line, 0), x(&doc, 0, 0), "первый `>` строки {line} уехал от полосы");
    }
    assert_eq!(x(&doc, 4, 2), x(&doc, 3, 2), "второй `>` пункта во вложенной цитате уехал от полосы");
}
