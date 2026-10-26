use masonry::core::keyboard::{Key, Modifiers};

use super::chord::{Chord, Name, Shift};
use super::table::BINDINGS;
use super::{Command, Scope, command};

const WIDGETS: &[&[Scope]] = &[
    &[Scope::Text, Scope::Editor, Scope::Navigation, Scope::Caret],
    &[Scope::Text, Scope::Navigation, Scope::Table, Scope::Caret],
    &[Scope::Image],
    &[Scope::Text, Scope::Image],
    &[Scope::List, Scope::Navigation],
    &[Scope::List],
    &[Scope::Find],
    &[Scope::Text, Scope::Reader],
    &[Scope::Dialog],
    &[Scope::Slider],
    &[Scope::Choice],
    &[Scope::Number],
];

fn press(chord: &Chord) -> (Key, Modifiers) {
    let key = match chord.name {
        Name::Char(c) => Key::Character(c.to_string()),
        Name::Named(named) => Key::Named(named),
    };
    let mut modifiers = Modifiers::empty();
    if chord.action {
        modifiers |= crate::test_dir::command();
    }
    if chord.ctrl {
        modifiers |= Modifiers::CONTROL;
    }
    if chord.alt {
        modifiers |= Modifiers::ALT;
    }
    if chord.shift == Shift::Yes {
        modifiers |= Modifiers::SHIFT;
    }
    (key, modifiers)
}

#[test]
fn every_chord_runs_its_own_command_and_no_global_one_hides_a_field_key() {
    for scopes in [&[Scope::Global][..]].iter().chain(WIDGETS) {
        for binding in BINDINGS.iter().filter(|b| scopes.contains(&b.scope)) {
            for chord in binding.chords.iter().chain(binding.hidden).filter(|c| c.here()) {
                let (key, modifiers) = press(chord);
                assert_eq!(command(scopes, &key, modifiers), Some(binding.command), "{} в {scopes:?} перехвачено", chord.label());
                if !scopes.contains(&Scope::Global) {
                    assert_eq!(command(&[Scope::Global], &key, modifiers), None, "глобальное сочетание прячет {}", chord.label());
                }
            }
        }
    }
}

#[test]
fn every_shortcut_is_listed_in_settings_with_both_translations() {
    let listed: Vec<&str> = super::groups().into_iter().flat_map(|(title, rows)| std::iter::once(title).chain(rows.into_iter().map(|(label, _)| label))).collect();
    for locale in [include_str!("../../../assets/locales/ru.json"), include_str!("../../../assets/locales/en.json")] {
        let json: serde_json::Value = serde_json::from_str(locale).unwrap();
        for key in &listed {
            assert!(json["settings"]["shortcuts"][key].is_string(), "нет перевода settings.shortcuts.{key}");
        }
        for binding in BINDINGS {
            if !binding.chords.is_empty() {
                assert!(json["settings"]["shortcuts"][binding.label].is_string(), "нет перевода settings.shortcuts.{}", binding.label);
            }
        }
    }
    assert!(BINDINGS.iter().all(|b| !b.chords.is_empty() || !b.hidden.is_empty() || !b.gestures.is_empty()));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn insert_and_delete_chords_work_like_ctrl_letters() {
    let named = |key| Key::Named(key);
    let letter = |c: &str| Key::Character(c.into());
    use masonry::core::keyboard::NamedKey;
    let text = |key: &Key, modifiers| command(&[Scope::Text], key, modifiers);
    assert_eq!(text(&letter("C"), Modifiers::CONTROL), Some(Command::Copy));
    assert_eq!(text(&named(NamedKey::Insert), Modifiers::CONTROL), Some(Command::Copy));
    assert_eq!(text(&named(NamedKey::Insert), Modifiers::SHIFT), Some(Command::Paste));
    assert_eq!(text(&named(NamedKey::Delete), Modifiers::SHIFT), Some(Command::Cut));
    assert_eq!(text(&named(NamedKey::Delete), Modifiers::empty()), None);
    assert_eq!(text(&letter("c"), Modifiers::CONTROL | Modifiers::ALT), None, "AltGr не должен копировать");
    assert_eq!(super::label(Scope::Text, Command::Copy), "Ctrl + C / Ctrl + Insert");
    assert_eq!(super::label(Scope::Global, Command::FocusMode), "Ctrl + Shift + F");
}

#[cfg(target_os = "macos")]
#[test]
fn mac_navigation_chords() {
    use masonry::core::keyboard::NamedKey;
    let named = |key| Key::Named(key);
    let editor = |key: &Key, modifiers| command(&[Scope::Text, Scope::Editor, Scope::Navigation, Scope::Caret], key, modifiers);
    let meta = Modifiers::META;
    let alt = Modifiers::ALT;
    // ⌥←/→ — по словам
    assert_eq!(editor(&named(NamedKey::ArrowLeft), alt), Some(Command::WordLeft));
    assert_eq!(editor(&named(NamedKey::ArrowRight), alt), Some(Command::WordRight));
    assert_eq!(editor(&named(NamedKey::ArrowLeft), alt | Modifiers::SHIFT), Some(Command::WordLeft));
    assert_eq!(editor(&named(NamedKey::ArrowRight), alt | Modifiers::SHIFT), Some(Command::WordRight));
    // ⌘←/→ — начало/конец строки
    assert_eq!(editor(&named(NamedKey::ArrowLeft), meta), Some(Command::LineStart));
    assert_eq!(editor(&named(NamedKey::ArrowRight), meta), Some(Command::LineEnd));
    assert_eq!(editor(&named(NamedKey::ArrowLeft), meta | Modifiers::SHIFT), Some(Command::LineStart));
    assert_eq!(editor(&named(NamedKey::ArrowRight), meta | Modifiers::SHIFT), Some(Command::LineEnd));
    // ⌘↑/↓ — начало/конец заметки
    assert_eq!(editor(&named(NamedKey::ArrowUp), meta), Some(Command::DocumentStart));
    assert_eq!(editor(&named(NamedKey::ArrowDown), meta), Some(Command::DocumentEnd));
    assert_eq!(editor(&named(NamedKey::ArrowUp), meta | Modifiers::SHIFT), Some(Command::DocumentStart));
    assert_eq!(editor(&named(NamedKey::Home), meta), Some(Command::DocumentStart));
    assert_eq!(editor(&named(NamedKey::End), meta), Some(Command::DocumentEnd));
    // ⌥⌫ — удалить слово, ⌘⌫ — удалить до начала строки
    assert_eq!(editor(&named(NamedKey::Backspace), alt), Some(Command::DeleteWord));
    assert_eq!(editor(&named(NamedKey::Backspace), Modifiers::CONTROL), Some(Command::DeleteWord));
    assert_eq!(editor(&named(NamedKey::Backspace), meta), Some(Command::LineDeleteBackward));
    // ⌥⌦ — удалить слово вперёд
    assert_eq!(editor(&named(NamedKey::Delete), alt), Some(Command::DeleteWordForward));
    assert_eq!(editor(&named(NamedKey::Delete), Modifiers::CONTROL), Some(Command::DeleteWordForward));
    // ⌘ не заменяет Ctrl для переключения вкладок
    assert_eq!(command(&[Scope::Global], &named(NamedKey::Tab), Modifiers::CONTROL), Some(Command::NextTab));
    assert_eq!(command(&[Scope::Global], &named(NamedKey::Tab), meta), None);
    // Подпись сочетаний на Mac
    assert_eq!(super::label(Scope::Global, Command::FocusMode), "⌘ + Shift + F");
    assert_eq!(super::label(Scope::Navigation, Command::WordLeft), "⌥ + ←");
    assert_eq!(super::label(Scope::Navigation, Command::DocumentStart), "⌘ + ↑");
}
