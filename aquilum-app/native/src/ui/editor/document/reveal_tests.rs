use masonry::core::StyleSet;
use masonry::parley::{FontContext, LayoutContext};

use super::super::embed::panel::PAD;
use super::tests::markdown;
use super::{Document, Fold};

#[test]
fn a_formula_being_edited_shows_plain_source_with_the_preview_below() {
    let mut doc = markdown("$$\nx^2\n$$\nтекст");
    doc.keep = (0.0, 10_000.0);
    let (mut fcx, mut lcx, styles) = (FontContext::default(), LayoutContext::new(), StyleSet::new(16.0));
    let mut folds = |doc: &mut Document, caret| {
        doc.layout(&mut fcx, &mut lcx, &styles, Some(600.0), None, caret);
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
