use super::super::markdown::{self, Fonts, x_at};
use super::tests::{blocks, laid, markdown};
use super::Document;

fn tagged(text: &str, tags: bool) -> Document {
    let mut doc = Document::new(text);
    doc.set_markdown(Some(Fonts::new(16.0, true, tags)));
    laid(doc)
}

#[test]
fn indented_text_lines_keep_their_inline_markup() {
    let doc = laid(markdown("текст\n    `код` и #тег\n\tещё `код`"));
    assert_eq!((doc.paragraph(1).decor().codes.len(), doc.paragraph(1).decor().tags.len()), (1, 1), "{:?}", blocks(&doc));
    assert_eq!(doc.paragraph(2).decor().codes.len(), 1, "{:?}", blocks(&doc));
}

#[test]
fn chips_of_neighbouring_tags_do_not_touch_but_text_around_them_is_left_alone() {
    let text = "слово #один #два #три. И #четыре, конец";
    let doc = tagged(text, true);
    let p = doc.paragraph(0);
    let layout = p.laid().expect("абзац свёрстан");
    let chips = markdown::tag_chips(layout, p.decor(), masonry::kurbo::Vec2::ZERO);
    assert_eq!(chips.len(), 4);
    for pair in chips.windows(2) {
        assert!(pair[0].0.rect().x1 + 0.5 <= pair[1].0.rect().x0, "плашки тегов наезжают друг на друга: {:?} и {:?}", pair[0].0.rect(), pair[1].0.rect());
    }
    let plain = tagged(text.replace('#', "").as_str(), true);
    let plain_layout = plain.paragraph(0).laid().expect("абзац свёрстан");
    assert!((x_at(layout, "слово ".len()) - x_at(plain_layout, "слово ".len())).abs() < 0.5, "текст перед первым тегом сдвинут");
}

#[test]
fn tags_get_palette_colors_or_the_accent_when_colors_are_off() {
    let colored = tagged("#один #два #один #Один", true);
    let slots: Vec<Option<usize>> = colored.paragraph(0).decor().tags.iter().map(|(_, slot)| *slot).collect();
    assert!(slots.iter().all(Option::is_some));
    assert_eq!((slots[0], slots[0]), (slots[2], slots[3]), "один и тот же тег получил разные цвета");
    let plain = tagged("#один #два", false);
    assert!(plain.paragraph(0).decor().tags.iter().all(|(_, slot)| slot.is_none()), "при выключенных цветах теги не акцентные");
}
