use masonry::core::keyboard::{Key, Modifiers, NamedKey};

use super::support::{markdown_root, named, press, text};

#[test]
fn shortcuts_toggle_bold_italic_and_strike() {
    let action = if cfg!(target_os = "macos") { Modifiers::META } else { Modifiers::CONTROL };
    let mut root = markdown_root("слово");
    press(&mut root, Key::Character("a".into()), action);
    press(&mut root, Key::Character("b".into()), action);
    assert_eq!(text(&root), "**слово**");
    press(&mut root, Key::Character("b".into()), action);
    assert_eq!(text(&root), "слово");
    press(&mut root, Key::Character("i".into()), action);
    assert_eq!(text(&root), "*слово*");
    press(&mut root, Key::Character("b".into()), action);
    assert_eq!(text(&root), "***слово***");
    press(&mut root, Key::Character("i".into()), action);
    assert_eq!(text(&root), "**слово**");
    press(&mut root, Key::Character("s".into()), action | Modifiers::SHIFT);
    assert_eq!(text(&root), "**~~слово~~**");
    named(&mut root, NamedKey::End);
    press(&mut root, Key::Character("b".into()), action);
    assert_eq!(text(&root), "**~~слово~~**", "каретка вне разметки и без выделения ничего не меняет");
    for _ in 0..5 {
        named(&mut root, NamedKey::ArrowLeft);
    }
    press(&mut root, Key::Character("b".into()), action);
    assert_eq!(text(&root), "~~слово~~", "каретка внутри жирного снимает его");
}
