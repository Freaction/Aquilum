use masonry::core::keyboard::{Key, Modifiers, NamedKey};

use super::support::{command_key, markdown_root, named, press, text, typing};
use crate::test_dir::command;

fn to_line_end(root: &mut masonry::app::RenderRoot, line: usize) {
    press(root, Key::Named(NamedKey::Home), command());
    for _ in 0..line {
        named(root, NamedKey::ArrowDown);
    }
    named(root, NamedKey::End);
}

fn shift_tab(root: &mut masonry::app::RenderRoot) {
    press(root, Key::Named(NamedKey::Tab), Modifiers::SHIFT);
}

#[test]
fn a_list_keeps_the_number_it_starts_with() {
    let mut root = markdown_root("текст");
    to_line_end(&mut root, 0);
    named(&mut root, NamedKey::Enter);
    typing(&mut root, "11. a");
    named(&mut root, NamedKey::Enter);
    typing(&mut root, "b");
    assert_eq!(text(&root), "текст\n11. a\n12. b");
    let mut root = markdown_root("1. a\n\nабзац\n");
    to_line_end(&mut root, 3);
    typing(&mut root, "13. c");
    assert_eq!(text(&root), "1. a\n\nабзац\n13. c");
}

#[test]
fn removing_the_space_after_a_number_and_typing_it_back_keeps_the_number() {
    let mut root = markdown_root("7. a\n8. b\n9. c\n10. d");
    to_line_end(&mut root, 3);
    named(&mut root, NamedKey::Enter);
    named(&mut root, NamedKey::Backspace);
    assert_eq!(text(&root), "7. a\n8. b\n9. c\n10. d\n11.");
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "7. a\n8. b\n9. c\n10. d\n11.\n", "строка без пробела — уже не пункт");
    named(&mut root, NamedKey::Backspace);
    typing(&mut root, " x");
    assert_eq!(text(&root), "7. a\n8. b\n9. c\n10. d\n11. x");
}

#[test]
fn tab_nests_and_shift_tab_brings_the_number_back() {
    let mut root = markdown_root("9. a\n10. b\n11. c");
    to_line_end(&mut root, 1);
    named(&mut root, NamedKey::Tab);
    assert_eq!(text(&root), "9. a\n\t1. b\n10. c");
    named(&mut root, NamedKey::Tab);
    typing(&mut root, "!");
    assert_eq!(text(&root), "9. a\n\t\t1. b!\n10. c");
    shift_tab(&mut root);
    shift_tab(&mut root);
    typing(&mut root, "?");
    assert_eq!(text(&root), "9. a\n10. b!?\n11. c");
    command_key(&mut root, "z");
    command_key(&mut root, "z");
    assert_eq!(text(&root), "9. a\n\t1. b!\n10. c");
}

#[test]
fn shift_tab_keeps_the_caret_on_its_line_when_the_number_grows() {
    let mut root = markdown_root("1. a\n2. b\n3. c\n4. d\n5. e\n6. f\n7. g\n8. h\n9. i\n\t1. j");
    to_line_end(&mut root, 9);
    shift_tab(&mut root);
    typing(&mut root, "!");
    assert!(text(&root).ends_with("9. i\n10. j!"), "{}", text(&root));
}

#[test]
fn typed_and_deleted_items_keep_the_list_consecutive() {
    let mut root = markdown_root("1. a\n2. b");
    to_line_end(&mut root, 1);
    named(&mut root, NamedKey::Enter);
    for _ in 0..3 {
        named(&mut root, NamedKey::Backspace);
    }
    typing(&mut root, "7. c");
    assert_eq!(text(&root), "1. a\n2. b\n3. c");
    let mut root = markdown_root("1. a\n2. b\n3. c");
    to_line_end(&mut root, 0);
    named(&mut root, NamedKey::ArrowRight);
    press(&mut root, Key::Named(NamedKey::ArrowDown), Modifiers::SHIFT);
    named(&mut root, NamedKey::Backspace);
    assert_eq!(text(&root), "1. a\n2. c");
    command_key(&mut root, "z");
    assert_eq!(text(&root), "1. a\n2. b\n3. c");
}

#[test]
fn nested_lists_count_from_one_and_continuations_do_not_break_them() {
    let mut root = markdown_root("5. a\n\t3. b\n\t   ещё b\n\t7. c\n9. d");
    to_line_end(&mut root, 0);
    typing(&mut root, "x");
    assert_eq!(text(&root), "5. ax\n\t1. b\n\t   ещё b\n\t2. c\n6. d");
}

#[test]
fn enter_on_an_empty_item_steps_out_one_level_at_a_time() {
    let mut root = markdown_root("1. a\n\t1. b");
    to_line_end(&mut root, 1);
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "1. a\n\t1. b\n\t2. ");
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "1. a\n\t1. b\n2. ");
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "1. a\n\t1. b\n");
}

#[test]
fn enter_on_a_continuation_line_opens_the_next_item() {
    let mut root = markdown_root("1. a\n2. b");
    to_line_end(&mut root, 0);
    press(&mut root, Key::Named(NamedKey::Enter), Modifiers::SHIFT);
    typing(&mut root, "x");
    named(&mut root, NamedKey::Enter);
    typing(&mut root, "y");
    assert_eq!(text(&root), "1. a\n   x\n2. y\n3. b");
}
