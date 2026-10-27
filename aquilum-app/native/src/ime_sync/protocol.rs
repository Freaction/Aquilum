use std::collections::VecDeque;

use super::keyboard::{App, Keyboard};
use super::{Edit, ImeState, Message, Session};

const SEEDS: u64 = 500;

enum Key {
    Finish,
    Commit(&'static str),
    Compose(&'static str),
    Backspace,
    Join,
}

struct Run {
    app: App,
    keyboard: Keyboard,
    session: Session,
    id: i32,
    to_keyboard: VecDeque<(i32, ImeState)>,
    seed: u64,
}

impl Run {
    fn new(text: &str, seed: u64) -> Self {
        let mut session = Session::default();
        let id = session.start();
        Run { app: App::new(text), keyboard: Keyboard::new(id), session, id, to_keyboard: VecDeque::new(), seed }
    }

    fn roll(&mut self, sides: u64) -> u64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed % sides
    }

    fn to_app(&mut self) -> bool {
        let Some(message) = self.keyboard.outbox.pop_front() else { return false };
        self.session.received(message);
        true
    }

    fn frame(&mut self) -> bool {
        for edit in self.session.take() {
            self.app.apply(edit);
        }
        let report = self.session.report(0, self.app.state());
        let sent = report.is_some();
        self.to_keyboard.extend(report);
        sent
    }

    fn to_keyboard(&mut self) -> bool {
        let Some((applied, ime)) = self.to_keyboard.pop_front() else { return false };
        self.keyboard.apply(applied, 0, ime);
        true
    }

    fn press(&mut self, key: &Key) {
        match key {
            Key::Finish => self.keyboard.finish_composing(),
            Key::Commit(text) => self.keyboard.write(text, false),
            Key::Compose(text) => self.keyboard.write(text, true),
            Key::Backspace => self.keyboard.backspace(),
            Key::Join => self.keyboard.start(self.id),
        }
    }

    fn play(&mut self, keys: &[Key], check: impl Fn(&App)) {
        let mut keys = keys.iter();
        loop {
            match self.roll(4) {
                0 => _ = self.to_app(),
                1 => _ = self.frame(),
                2 => _ = self.to_keyboard(),
                _ => match keys.next() {
                    Some(key) => self.press(key),
                    None => break,
                },
            }
            check(&self.app);
        }
        self.settle();
    }

    fn settle(&mut self) {
        for _ in 0..100 {
            while self.to_app() {}
            let sent = self.frame();
            let mut delivered = false;
            while self.to_keyboard() {
                delivered = true;
            }
            if !sent && !delivered && self.keyboard.outbox.is_empty() {
                assert_eq!(self.keyboard.field, self.app.state().to_ime(), "клавиатура и приложение разошлись");
                assert_eq!(self.session.base(), Some(0), "окно текста не подтверждено");
                return;
            }
        }
        panic!("клавиатура и приложение не сошлись");
    }
}

fn each_order(text: &str, keys: &[Key], check: impl Fn(&App, u64)) -> impl Iterator<Item = (u64, Run)> {
    (1..=SEEDS).map(move |seed| {
        let mut run = Run::new(text, seed);
        run.play(keys, |app| check(app, seed));
        (seed, run)
    })
}

#[test]
fn opening_the_keyboard_never_changes_the_note() {
    let note = "- заметка\nвторая строка";
    for (seed, run) in each_order(note, &[Key::Finish, Key::Finish, Key::Finish], |app, seed| assert_eq!(app.text(), note, "порядок {seed}")) {
        assert_eq!(run.keyboard.field.text, note, "порядок {seed}");
    }
}

#[test]
fn typed_letters_land_once_in_any_order() {
    for (seed, run) in each_order("- a", &[Key::Finish, Key::Commit("x"), Key::Commit("\n"), Key::Commit("y")], |_, _| {}) {
        assert_eq!(run.app.text(), "- ax\n- y", "порядок {seed}");
    }
}

#[test]
fn a_composed_word_lands_once_in_any_order() {
    let keys = [Key::Finish, Key::Compose("с"), Key::Compose("сл"), Key::Compose("сло"), Key::Commit("слово"), Key::Commit(" ")];
    for (seed, run) in each_order("текст ", &keys, |app, seed| assert!(app.text().starts_with("текст "), "порядок {seed}: {}", app.text())) {
        assert_eq!(run.app.text(), "текст слово ", "порядок {seed}");
    }
}

#[test]
fn backspace_after_the_note_arrived_deletes_one_letter() {
    for seed in 1..=SEEDS {
        let mut run = Run::new("кот", seed);
        run.play(&[Key::Finish], |_| {});
        run.play(&[Key::Backspace, Key::Commit("д")], |app| assert!(app.text().starts_with("ко"), "порядок {seed}: {}", app.text()));
        assert_eq!(run.app.text(), "код", "порядок {seed}");
    }
}

#[test]
fn letters_meant_for_another_field_never_land_in_the_note() {
    for seed in 1..=SEEDS {
        let mut run = Run::new("заметка", seed);
        run.keyboard.write("Заголовок", false);
        run.id = run.session.start();
        run.play(&[Key::Commit(" новый"), Key::Join, Key::Finish, Key::Commit("!")], |app| assert!(!app.text().contains("Заг"), "порядок {seed}: {}", app.text()));
        assert_eq!(run.app.text(), "заметка!", "порядок {seed}");
    }
}

#[test]
fn a_caret_move_from_the_keyboard_counts_from_the_window_it_saw() {
    let mut session = Session::default();
    let id = session.start();
    let state = |cursor| ImeState { text: "abcdef".into(), anchor: cursor, cursor, compose: None };
    session.received(Message { session: id, seq: 1, echo: true, base: Some(100), state: state(6) });
    session.received(Message { session: id, seq: 2, echo: false, base: Some(100), state: state(2) });
    assert_eq!(session.take(), [Edit::Select { anchor: 102, cursor: 102 }]);
    assert!(session.report(100, &super::AppState { text: "abcdef".into(), anchor: 2, cursor: 2, preedit: None }).is_none());
}
