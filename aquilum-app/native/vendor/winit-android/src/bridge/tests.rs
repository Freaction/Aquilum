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

fn bridge(marked: &str) -> Bridge {
    let mut bridge = Bridge::new(String::new());
    report(&mut bridge, marked);
    assert_eq!(bridge.take_push(), Some(parse(marked)));
    bridge
}

fn report(bridge: &mut Bridge, marked: &str) {
    let state = parse(marked);
    bridge.app_changed(state.text, state.cursor, state.anchor);
}

fn feed(bridge: &mut Bridge, marked: &str) -> Vec<Edit> {
    bridge.ime_changed(&parse(marked))
}

fn commit(text: &str) -> Edit {
    Edit::Commit(text.into())
}

fn preedit(text: &str, at: usize) -> Edit {
    Edit::Preedit(text.into(), Some((at, at)))
}

fn clear() -> Edit {
    Edit::Preedit(String::new(), None)
}

fn delete(before: usize, after: usize) -> Edit {
    Edit::DeleteSurrounding { before, after }
}

#[test]
fn letters_typed_one_by_one_are_commits() {
    let mut b = bridge("|");
    assert_eq!(feed(&mut b, "a|"), [commit("a")]);
    assert_eq!(feed(&mut b, "ab|"), [commit("b")]);
    assert_eq!(b.take_push(), None);
}

#[test]
fn a_composed_word_is_preedit_until_the_space() {
    let mut b = bridge("|");
    assert_eq!(feed(&mut b, "[п|]"), [preedit("п", 2)]);
    assert_eq!(feed(&mut b, "[пр|]"), [preedit("пр", 4)]);
    assert_eq!(feed(&mut b, "[при|]"), [preedit("при", 6)]);
    assert_eq!(feed(&mut b, "при |"), [clear(), commit("при ")]);
}

#[test]
fn a_swiped_word_commits_the_previous_one() {
    let mut b = bridge("|");
    assert_eq!(feed(&mut b, "[hello|]"), [preedit("hello", 5)]);
    assert_eq!(feed(&mut b, "hello [world|]"), [clear(), commit("hello "), preedit("world", 5)]);
}

#[test]
fn autocorrect_rewrites_the_word_through_its_composing_region() {
    let mut b = bridge("teh|");
    assert_eq!(feed(&mut b, "[teh|]"), []);
    assert_eq!(feed(&mut b, "[the|]"), [delete(3, 0), preedit("the", 3)]);
    assert_eq!(feed(&mut b, "the|"), [clear(), commit("the")]);
    let mut b = bridge("teh |");
    assert_eq!(feed(&mut b, "[teh] |"), []);
    assert_eq!(feed(&mut b, "the |"), [delete(3, 0), commit("he ")]);
}

#[test]
fn backspace_deletes_around_the_caret() {
    let mut b = bridge("hel|lo");
    assert_eq!(feed(&mut b, "he|lo"), [delete(1, 0)]);
    let mut b = bridge("a|a");
    assert_eq!(feed(&mut b, "|a"), [delete(1, 0)]);
    let mut b = bridge("a|a");
    assert_eq!(feed(&mut b, "a|"), [delete(0, 1)]);
    let mut b = bridge("|");
    feed(&mut b, "[hel|]");
    assert_eq!(feed(&mut b, "[he|]"), [preedit("he", 2)]);
}

#[test]
fn deleting_a_selection_commits_nothing_over_it() {
    let mut b = bridge("^hello| world");
    assert_eq!(feed(&mut b, "| world"), [commit("")]);
    let mut b = bridge("|hello^ world");
    assert_eq!(feed(&mut b, "x| world"), [commit("x")]);
}

#[test]
fn cyrillic_and_emoji_are_counted_in_bytes() {
    let mut b = bridge("мир 👋|");
    assert_eq!(feed(&mut b, "мир |"), [delete(4, 0)]);
    assert_eq!(feed(&mut b, "ми|"), [delete(3, 0)]);
    let state = ImeState::from_utf16("я👋ы".into(), (3, 3), Some((3, 1)));
    assert_eq!((state.anchor, state.cursor, state.compose.clone()), (6, 6, Some(2..6)));
    assert_eq!(state.utf16(), ((3, 3), Some((1, 3))));
}

#[test]
fn a_typed_newline_is_the_enter_key() {
    let mut b = bridge("abc|");
    assert_eq!(feed(&mut b, "abc\n|"), [Edit::Enter]);
    let mut b = bridge("|");
    feed(&mut b, "[word|]");
    assert_eq!(feed(&mut b, "word\n|"), [clear(), commit("word"), Edit::Enter]);
    assert_eq!(feed(&mut b, "word\na\nb\n|"), [commit("a\nb\n")]);
}

#[test]
fn caret_moves_become_arrows() {
    let mut b = bridge("abc|");
    assert_eq!(feed(&mut b, "a|bc"), [Edit::Left(2)]);
    assert_eq!(feed(&mut b, "ab|c"), [Edit::Right(1)]);
    let mut b = bridge("👋x|");
    assert_eq!(feed(&mut b, "|👋x"), [Edit::Left(2)]);
    let mut b = bridge("abc|");
    assert_eq!(feed(&mut b, "^ab|c"), []);
    assert_eq!(b.take_push(), Some(parse("abc|")));
}

#[test]
fn the_ime_is_restarted_only_when_the_app_disagrees() {
    let mut b = bridge("- a|");
    assert_eq!(feed(&mut b, "- a\n|"), [Edit::Enter]);
    report(&mut b, "- a\n|");
    assert_eq!(b.take_push(), None);
    report(&mut b, "- a\n- |");
    assert_eq!(b.take_push(), Some(parse("- a\n- |")));
    let mut b = bridge("abc|def");
    report(&mut b, "bc|de");
    assert_eq!(b.take_push(), None);
    report(&mut b, "xabc|def");
    assert_eq!(b.take_push(), Some(parse("xabc|def")));
    b.restart();
    assert_eq!(b.take_push(), Some(parse("xabc|def")));
}

#[test]
fn the_preedit_survives_a_restart() {
    let mut b = bridge("a |");
    feed(&mut b, "a [bc|]");
    report(&mut b, "x a |");
    assert_eq!(b.take_push(), Some(parse("x a [bc|]")));
}

#[test]
fn ime_states_older_than_a_restart_are_dropped() {
    let mut b = bridge("one|");
    assert_eq!(feed(&mut b, "onex|"), [commit("x")]);
    report(&mut b, "onex\n- |");
    assert!(b.take_push().is_some());
    assert_eq!(feed(&mut b, "onexy|"), []);
    assert_eq!(feed(&mut b, "onex\n- z|"), [commit("z")]);
    report(&mut b, "onex\n- z\n- |");
    assert!(b.take_push().is_some());
    assert_eq!(feed(&mut b, "onex\n- z|"), []);
    assert_eq!(feed(&mut b, "onex\n- zw|"), [delete(3, 0), commit("w")]);
}
