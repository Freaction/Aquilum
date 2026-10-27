use masonry::core::keyboard::{Key, Modifiers, NamedKey};

use super::support::{click, command_key, markdown_root, named, press, text, typing};
use crate::test_dir::command;

#[test]
fn list_lines_inside_code_blocks_stay_plain_text() {
    let mut root = markdown_root("```\n- один\n```");
    click(&mut root, 100.0, 10.0);
    named(&mut root, NamedKey::ArrowDown);
    named(&mut root, NamedKey::End);
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "```\n- один\n\n```", "Enter внутри блока кода не продолжает список");
    let mut root = markdown_root("```\n1. а\n3. б\n```");
    click(&mut root, 100.0, 10.0);
    named(&mut root, NamedKey::ArrowDown);
    named(&mut root, NamedKey::ArrowDown);
    named(&mut root, NamedKey::End);
    typing(&mut root, "x");
    assert_eq!(text(&root), "```\n1. а\n3. бx\n```", "номера в блоке кода не пересчитываются");
}

#[test]
fn tab_indents_and_shift_tab_outdents_lines() {
    let mut root = markdown_root("- один\n- два\nтекст");
    click(&mut root, 590.0, 30.0);
    named(&mut root, NamedKey::Tab);
    assert_eq!(text(&root), "- один\n\t- два\nтекст");
    typing(&mut root, "!");
    assert_eq!(text(&root), "- один\n\t- два!\nтекст");
    press(&mut root, Key::Named(NamedKey::Tab), Modifiers::SHIFT);
    assert_eq!(text(&root), "- один\n- два!\nтекст");
    command_key(&mut root, "a");
    named(&mut root, NamedKey::Tab);
    assert_eq!(text(&root), "\t- один\n\t- два!\n\tтекст");
    command_key(&mut root, "z");
    assert_eq!(text(&root), "- один\n- два!\nтекст");
}

#[test]
fn tab_stops_at_the_deepest_level_so_shift_tab_works_at_once() {
    let source = (0..8).map(|n| format!("{}- {n}", "\t".repeat(n))).collect::<Vec<_>>().join("\n");
    let mut root = markdown_root(&source);
    press(&mut root, Key::Named(NamedKey::End), command());
    named(&mut root, NamedKey::Tab);
    named(&mut root, NamedKey::Tab);
    assert_eq!(text(&root), source);
    press(&mut root, Key::Named(NamedKey::Tab), Modifiers::SHIFT);
    assert!(text(&root).ends_with("\n\t\t\t\t\t\t- 7"), "{:?}", text(&root));
}

#[test]
fn enter_continues_lists_and_quotes() {
    let cases = [
        ("- один", "- один\n- "),
        ("\t- один", "\t- один\n\t- "),
        ("1. один", "1. один\n2. "),
        ("3) один", "3) один\n4) "),
        ("- [ ] дело", "- [ ] дело\n- [ ] "),
        ("> цитата", "> цитата\n> "),
        ("текст", "текст\n"),
    ];
    for (source, expected) in cases {
        let mut root = markdown_root(source);
        click(&mut root, 590.0, 10.0);
        named(&mut root, NamedKey::Enter);
        assert_eq!(text(&root), expected, "{source:?}");
    }
    let mut root = markdown_root("\t- один");
    click(&mut root, 590.0, 10.0);
    named(&mut root, NamedKey::Enter);
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "\t- один\n- ");
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "\t- один\n");
    let mut root = markdown_root("- один");
    click(&mut root, 590.0, 10.0);
    press(&mut root, Key::Named(NamedKey::Enter), Modifiers::SHIFT);
    typing(&mut root, "x");
    assert_eq!(text(&root), "- один\n  x");
}

#[test]
fn ordered_lists_renumber_in_one_undo_step() {
    let mut root = markdown_root("1. a\n2. b\n3. c");
    named(&mut root, NamedKey::ArrowDown);
    named(&mut root, NamedKey::End);
    named(&mut root, NamedKey::Enter);
    typing(&mut root, "x");
    assert_eq!(text(&root), "1. a\n2. b\n3. x\n4. c");
    command_key(&mut root, "z");
    command_key(&mut root, "z");
    assert_eq!(text(&root), "1. a\n2. b\n3. c");
}

#[test]
fn enter_continues_star_plus_and_task_items() {
    let mut root = markdown_root("* a");
    named(&mut root, NamedKey::End);
    named(&mut root, NamedKey::Enter);
    typing(&mut root, "b");
    assert_eq!(text(&root), "* a\n* b");
    let mut root = markdown_root("+ [x] a");
    named(&mut root, NamedKey::End);
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "+ [x] a\n+ [ ] ");
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "+ [x] a\n");
}

#[test]
fn pasted_paragraphs_become_items_of_the_list() {
    let mut root = markdown_root("- первый");
    named(&mut root, NamedKey::End);
    super::support::paste(&mut root, " пункт\n\nвторой\n- третий\n");
    assert_eq!(text(&root), "- первый пункт\n- второй\n- третий");
    let mut root = markdown_root("1. [ ] раз");
    named(&mut root, NamedKey::End);
    super::support::paste(&mut root, "\nдва\nтри");
    assert_eq!(text(&root), "1. [ ] раз\n2. [ ] два\n3. [ ] три");
}
