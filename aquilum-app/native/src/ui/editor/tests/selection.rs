use masonry::core::keyboard::{Key, Modifiers, NamedKey};

use super::support::{command_key, focused, markdown_root, named, press, root, selected, tap, text, typing};
use crate::test_dir::{command, word_modifier};
use crate::ui::editor::{Smart, TextEditor};

#[test]
fn arrows_cross_paragraphs_and_keep_column() {
    let mut root = root("abcdef\nab\nabcdef");
    for _ in 0..5 {
        named(&mut root, NamedKey::ArrowRight);
    }
    named(&mut root, NamedKey::ArrowDown);
    named(&mut root, NamedKey::ArrowDown);
    typing(&mut root, "X");
    assert_eq!(text(&root), "abcdef\nab\nabcdeXf");
    named(&mut root, NamedKey::Home);
    named(&mut root, NamedKey::ArrowLeft);
    typing(&mut root, "Y");
    assert_eq!(text(&root), "abcdef\nabY\nabcdeXf");
}

#[test]
fn selection_is_replaced_by_typing() {
    let mut root = root("hello world");
    named(&mut root, NamedKey::End);
    for _ in 0..5 {
        press(&mut root, Key::Named(NamedKey::ArrowLeft), Modifiers::SHIFT);
    }
    typing(&mut root, "there");
    assert_eq!(text(&root), "hello there");
    press(&mut root, Key::Named(NamedKey::Backspace), word_modifier());
    assert_eq!(text(&root), "hello ");
    command_key(&mut root, "a");
    named(&mut root, NamedKey::Delete);
    assert_eq!(text(&root), "");
}

#[test]
fn double_tap_selects_word_everywhere() {
    for (source, word) in [("- & Буллит один\n\nтекст", "Буллит"), ("1. Буллит один\n\nтекст", "Буллит"), ("> [!quote] Буллит один\n\nтекст", "Буллит"), ("Буллит один\n\nтекст", "Буллит"), ("\t- Буллит один\n\nтекст", "Буллит")] {
        let mut root = markdown_root(source);
        let x = (0..400).step_by(4).map(f64::from).find(|x| {
            tap(&mut root, *x, 10.0, 2, 0.0);
            selected(&root) == word
        });
        let Some(x) = x else { panic!("{source:?}: слово не выделяется двойным щелчком") };
        tap(&mut root, x, 10.0, 1, 0.0);
        tap(&mut root, x, 10.0, 2, 3.0);
        assert_eq!(selected(&root), word, "{source:?}: после касания с дрожанием");
    }
}

fn picture(text: &str, matches: bool) -> Vec<u32> {
    let mut root = focused(TextEditor::new(text).with_smart(Smart { matches, ..Smart::default() }), 400, 200);
    for _ in 0..3 {
        press(&mut root, Key::Named(NamedKey::ArrowRight), Modifiers::SHIFT);
    }
    assert_eq!(selected(&root), "кот");
    let (layers, _) = root.redraw();
    let mut renderer = crate::render::Renderer::new(1);
    renderer.render(&layers, 400, 200, 1.0, crate::ui::theme::background()).unwrap();
    renderer.frame_pixels().to_vec()
}

fn frame(text: &str, select: bool) -> Vec<u32> {
    let mut root = focused(TextEditor::new(text).markdown(), 400, 200);
    press(&mut root, Key::Named(NamedKey::End), command());
    if select {
        command_key(&mut root, "a");
    }
    let (layers, _) = root.redraw();
    let mut renderer = crate::render::Renderer::new(1);
    renderer.render(&layers, 400, 200, 1.0, crate::ui::theme::background()).unwrap();
    renderer.frame_pixels().to_vec()
}

#[test]
fn selection_is_painted_over_code_backgrounds() {
    let _lock = crate::ui::theme::test_lock();
    let before = crate::ui::theme::is_dark();
    crate::ui::theme::apply_setting("light", false, "#1471eb");
    let tm = crate::ui::theme::current();
    for (text, backdrop) in [("`код в строке`\n\nтекст", tm.editor_code_bg), ("```\nблок кода\n```\nтекст", crate::ui::editor::embed::panel::Tone::Surface.fill())] {
        let (c, b) = (backdrop.to_rgba8(), crate::ui::theme::background().to_rgba8());
        let a = f32::from(c.a) / 255.0;
        let target = [(c.r, b.r), (c.g, b.g), (c.b, b.b)].map(|(c, b)| f32::from(c) * a + f32::from(b) * (1.0 - a));
        let is_backdrop = |p: &u32| [(p >> 16) as u8, (p >> 8) as u8, *p as u8].iter().zip(target).all(|(p, t)| (f32::from(*p) - t).abs() <= 1.0);
        let (plain, selected) = (frame(text, false), frame(text, true));
        let backdrop = plain.iter().filter(|p| is_backdrop(p)).count();
        let covered = plain.iter().zip(&selected).filter(|(p, s)| is_backdrop(p) && p != s).count();
        assert!(backdrop > 100 && covered > 100, "{text:?}: выделение под подложкой (подложка {backdrop}, под выделением {covered})");
    }
    crate::ui::theme::apply_setting(if before { "dark" } else { "light" }, before, "#1471eb");
}

#[test]
fn matches_of_the_selection_light_up_only_when_switched_on() {
    assert!(!Smart::default().matches, "подсветка совпадений включена по умолчанию");
    assert_ne!(picture("кот и ещё кот", true), picture("кот и ещё кот", false), "переключатель ничего не меняет");
}
