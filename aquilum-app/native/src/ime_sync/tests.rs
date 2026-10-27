use super::*;

fn parse(marked: &str) -> ImeState {
    let mut state = ImeState::default();
    let (mut anchor, mut start) = (None, None);
    for c in marked.chars() {
        match c {
            '|' => state.cursor = state.text.len(),
            '^' => anchor = Some(state.text.len()),
            '[' => start = Some(state.text.len()),
            ']' => state.compose = start.map(|start| start..state.text.len()),
            _ => state.text.push(c),
        }
    }
    state.anchor = anchor.unwrap_or(state.cursor);
    state
}

fn sync(marked: &str) -> Mirror {
    let mut sync = Mirror::default();
    sync.echoed(&parse(marked));
    sync
}

fn feed(sync: &mut Mirror, marked: &str) -> Vec<Edit> {
    sync.ime_changed(&parse(marked))
}

fn commit(text: &str) -> Edit {
    Edit::Ime(Ime::Commit(text.into()))
}

fn preedit(text: &str, at: usize) -> Edit {
    Edit::Ime(Ime::Preedit(text.into(), Some((at, at))))
}

fn clear() -> Edit {
    Edit::Ime(Ime::Preedit(String::new(), None))
}

fn delete(before_bytes: usize, after_bytes: usize) -> Edit {
    Edit::Ime(Ime::DeleteSurrounding { before_bytes, after_bytes })
}

fn select(anchor: usize, cursor: usize) -> Edit {
    Edit::Select { anchor, cursor }
}

#[test]
fn letters_typed_one_by_one_are_commits() {
    let mut s = sync("|");
    assert_eq!(feed(&mut s, "a|"), [commit("a")]);
    assert_eq!(feed(&mut s, "ab|"), [commit("b")]);
}

#[test]
fn a_composed_word_is_preedit_until_the_space() {
    let mut s = sync("|");
    assert_eq!(feed(&mut s, "[п|]"), [preedit("п", 2)]);
    assert_eq!(feed(&mut s, "[пр|]"), [preedit("пр", 4)]);
    assert_eq!(feed(&mut s, "[при|]"), [preedit("при", 6)]);
    assert_eq!(feed(&mut s, "при |"), [clear(), commit("при ")]);
}

#[test]
fn a_swiped_word_commits_the_previous_one() {
    let mut s = sync("|");
    assert_eq!(feed(&mut s, "[hello|]"), [preedit("hello", 5)]);
    assert_eq!(feed(&mut s, "hello [world|]"), [clear(), commit("hello "), preedit("world", 5)]);
}

#[test]
fn a_word_marked_for_composing_becomes_the_preedit() {
    let mut s = sync("teh|");
    assert_eq!(feed(&mut s, "[teh|]"), [delete(3, 0), preedit("teh", 3)]);
    assert_eq!(feed(&mut s, "[the|]"), [preedit("the", 3)]);
    assert_eq!(feed(&mut s, "the|"), [clear(), commit("the")]);
    let mut s = sync("teh |");
    assert_eq!(feed(&mut s, "the |"), [delete(3, 0), commit("he ")]);
}

#[test]
fn backspace_deletes_around_the_caret() {
    let mut s = sync("hel|lo");
    assert_eq!(feed(&mut s, "he|lo"), [delete(1, 0)]);
    let mut s = sync("a|a");
    assert_eq!(feed(&mut s, "|a"), [delete(1, 0)]);
    let mut s = sync("a|a");
    assert_eq!(feed(&mut s, "a|"), [delete(0, 1)]);
    let mut s = sync("|");
    feed(&mut s, "[hel|]");
    assert_eq!(feed(&mut s, "[he|]"), [preedit("he", 2)]);
}

#[test]
fn deleting_a_selection_commits_nothing_over_it() {
    let mut s = sync("^hello| world");
    assert_eq!(feed(&mut s, "| world"), [commit("")]);
    let mut s = sync("|hello^ world");
    assert_eq!(feed(&mut s, "x| world"), [commit("x")]);
}

#[test]
fn cyrillic_and_emoji_are_counted_in_bytes() {
    let mut s = sync("мир 👋|");
    assert_eq!(feed(&mut s, "мир |"), [delete(4, 0)]);
    assert_eq!(feed(&mut s, "ми|"), [delete(3, 0)]);
    let state = ImeState::from_utf16("я👋ы".into(), (3, 3), Some((3, 1)));
    assert_eq!((state.anchor, state.cursor, state.compose.clone()), (6, 6, Some(2..6)));
    assert_eq!(state.utf16(), ((3, 3), Some((1, 3))));
}

#[test]
fn a_typed_newline_is_the_enter_key() {
    let mut s = sync("abc|");
    assert_eq!(feed(&mut s, "abc\n|"), [Edit::Enter]);
    let mut s = sync("|");
    feed(&mut s, "[word|]");
    assert_eq!(feed(&mut s, "word\n|"), [clear(), commit("word"), Edit::Enter]);
    assert_eq!(feed(&mut s, "word\na\nb\n|"), [commit("a\nb\n")]);
}

#[test]
fn caret_moves_and_selections_from_the_keyboard_are_selections() {
    let mut s = sync("abc|");
    assert_eq!(feed(&mut s, "a|bc"), [select(1, 1)]);
    assert_eq!(feed(&mut s, "ab|c"), [select(2, 2)]);
    assert_eq!(feed(&mut s, "^ab|c"), [select(0, 2)]);
    let mut s = sync("👋x|");
    assert_eq!(feed(&mut s, "|👋x"), [select(0, 0)]);
}

#[test]
fn only_a_different_app_state_is_reported() {
    let mut s = sync("- a|");
    assert_eq!(feed(&mut s, "- a\n|"), [Edit::Enter]);
    let same = AppState { text: "- a\n".into(), anchor: 4, cursor: 4, preedit: None };
    assert!(!s.differs(&same));
    let continued = AppState { text: "- a\n- ".into(), anchor: 6, cursor: 6, preedit: None };
    assert!(s.differs(&continued));
    assert_eq!(continued.to_ime(), parse("- a\n- |"));
    let composing = AppState { text: "a ".into(), anchor: 2, cursor: 2, preedit: Some(("bc".into(), 2)) };
    assert_eq!(composing.to_ime(), parse("a [bc|]"));
}

#[test]
fn an_empty_keyboard_before_the_note_arrives_deletes_nothing() {
    let mut s = Mirror::default();
    let note = AppState { text: "заметка".into(), anchor: 14, cursor: 14, preedit: None };
    assert!(s.differs(&note));
    assert_eq!(feed(&mut s, "|"), []);
    assert!(s.differs(&note), "приложение должно отправить текст ещё раз");
    s.echoed(&note.to_ime());
    assert!(!s.differs(&note));
    assert_eq!(feed(&mut s, "заметкаx|"), [commit("x")]);
}

#[test]
fn keyboard_edits_count_from_what_the_keyboard_had() {
    let mut s = sync("- a|");
    assert_eq!(feed(&mut s, "- a\n|"), [Edit::Enter]);
    assert!(s.differs(&AppState { text: "- a\n- ".into(), anchor: 6, cursor: 6, preedit: None }));
    assert_eq!(feed(&mut s, "- a\nb|"), [commit("b")]);
}
