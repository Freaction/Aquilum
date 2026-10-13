use masonry::core::keyboard::{Key, Modifiers, NamedKey};
use unicode_segmentation::UnicodeSegmentation;

use super::support::{named, press, root, selected, text};

const SAMPLE: &str = "a👨‍👩‍👧‍👦b🇷🇺c⚠️d❤️e👍f🏳️‍🌈g";

#[test]
fn shift_arrows_select_whole_emoji_one_at_a_time() {
    let mut root = root(SAMPLE);
    named(&mut root, NamedKey::Home);
    let mut wanted = String::new();
    for grapheme in SAMPLE.graphemes(true) {
        press(&mut root, Key::Named(NamedKey::ArrowRight), Modifiers::SHIFT);
        wanted.push_str(grapheme);
        assert_eq!(selected(&root), wanted, "выделение остановилось внутри эмодзи");
    }
}

#[test]
fn arrows_back_walk_over_whole_emoji() {
    let mut root = root(SAMPLE);
    named(&mut root, NamedKey::End);
    let mut wanted = String::new();
    for grapheme in SAMPLE.graphemes(true).rev() {
        press(&mut root, Key::Named(NamedKey::ArrowLeft), Modifiers::SHIFT);
        wanted.insert_str(0, grapheme);
        assert_eq!(selected(&root), wanted, "выделение назад остановилось внутри эмодзи");
    }
}

#[test]
fn backspace_and_delete_remove_a_whole_emoji() {
    let graphemes: Vec<&str> = SAMPLE.graphemes(true).collect();
    let mut root = root(SAMPLE);
    named(&mut root, NamedKey::End);
    named(&mut root, NamedKey::Backspace);
    named(&mut root, NamedKey::Backspace);
    assert_eq!(text(&root), graphemes[..graphemes.len() - 2].concat());
    named(&mut root, NamedKey::Home);
    named(&mut root, NamedKey::ArrowRight);
    named(&mut root, NamedKey::Delete);
    assert_eq!(text(&root), format!("{}{}", graphemes[0], graphemes[2..graphemes.len() - 2].concat()));
}

#[test]
fn a_click_never_puts_the_caret_inside_an_emoji() {
    use unicode_segmentation::GraphemeCursor;

    use super::support::{click, typing};

    let mut landed = std::collections::BTreeSet::new();
    for x in (0..360).step_by(2) {
        let mut root = root(SAMPLE);
        click(&mut root, f64::from(x), 12.0);
        typing(&mut root, "X");
        let edited = text(&root);
        let at = edited.find('X').expect("буква вставлена");
        assert_eq!(edited.replacen('X', "", 1), SAMPLE);
        assert!(GraphemeCursor::new(at, SAMPLE.len(), true).is_boundary(SAMPLE, 0).unwrap(), "клик на x={x} поставил каретку внутри эмодзи (смещение {at})");
        landed.insert(at);
    }
    assert!(landed.len() >= 10, "клики попадают не в разные места: {landed:?}");
}
