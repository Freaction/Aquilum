use masonry::core::keyboard::{Key, Modifiers, NamedKey};

use super::support::{command_key, markdown_root, named, press, text};

#[test]
fn ctrl_e_wraps_the_word_under_the_caret_or_the_selection_in_backticks() {
    let mut root = markdown_root("вызов функции здесь");
    for _ in 0..8 {
        named(&mut root, NamedKey::ArrowRight);
    }
    command_key(&mut root, "e");
    assert_eq!(text(&root), "вызов `функции` здесь");
    command_key(&mut root, "e");
    assert_eq!(text(&root), "вызов функции здесь", "повторное Ctrl+E снимает код");
    command_key(&mut root, "a");
    command_key(&mut root, "e");
    assert_eq!(text(&root), "`вызов функции здесь`");
}

#[test]
fn shortcuts_toggle_bold_italic_and_strike() {
    let mut root = markdown_root("слово");
    command_key(&mut root, "a");
    command_key(&mut root, "b");
    assert_eq!(text(&root), "**слово**");
    command_key(&mut root, "b");
    assert_eq!(text(&root), "слово");
    command_key(&mut root, "i");
    assert_eq!(text(&root), "*слово*");
    command_key(&mut root, "b");
    assert_eq!(text(&root), "***слово***");
    command_key(&mut root, "i");
    assert_eq!(text(&root), "**слово**");
    press(&mut root, Key::Character("s".into()), crate::test_dir::command() | Modifiers::SHIFT);
    assert_eq!(text(&root), "**~~слово~~**");
    named(&mut root, NamedKey::End);
    command_key(&mut root, "b");
    assert_eq!(text(&root), "**~~слово~~**", "каретка вне разметки и без выделения ничего не меняет");
    for _ in 0..5 {
        named(&mut root, NamedKey::ArrowLeft);
    }
    command_key(&mut root, "b");
    assert_eq!(text(&root), "~~слово~~", "каретка внутри жирного снимает его");
}
