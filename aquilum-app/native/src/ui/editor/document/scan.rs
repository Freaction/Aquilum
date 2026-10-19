use std::sync::mpsc::Sender;

use super::super::markdown::State;

const EVERY: usize = 1024;
const BATCH: usize = 64;

pub(super) fn walk(text: &crop::Rope, from: usize, mut state: State, front: usize, mut visit: impl FnMut(usize, &State) -> bool) -> State {
    let (mut prev, mut start, mut at, mut go) = (String::new(), None, from, true);
    crate::rope::each_line(text.byte_slice(from..), |line| {
        if let Some(begin) = start.filter(|begin| *begin >= front) {
            go = visit(begin, &state);
            if !go {
                return false;
            }
            state.step(&prev, Some(line));
        }
        prev.clear();
        prev.push_str(line);
        start = Some(at);
        at += line.len() + 1;
        true
    });
    if go && start.is_some_and(|begin| begin >= front && visit(begin, &state)) {
        state.step(&prev, None);
    }
    state
}

pub(super) fn scan(rope: crop::Rope, from: usize, state: State, until: usize, tx: Sender<Vec<(usize, State)>>) {
    let front = if from == 0 { crate::frontmatter::bounds_of(&crate::rope::Text(&rope)).map_or(0, |found| found.body) } else { 0 };
    let (mut batch, mut lines) = (Vec::new(), 0usize);
    walk(&rope, from, state, front, |start, state| {
        if lines % EVERY == 0 {
            batch.push((start, *state));
            if start >= until {
                return false;
            }
            if batch.len() == BATCH && tx.send(std::mem::take(&mut batch)).is_err() {
                return false;
            }
        }
        lines += 1;
        true
    });
    let _ = tx.send(batch);
}
