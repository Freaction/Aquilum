use std::time::{Duration, Instant};

use super::change::Change;

const GROUP: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Typing,
    Deleting,
    Other,
    Block,
    Outside,
}

#[derive(Clone, Debug)]
pub struct Step {
    pub changes: Vec<Change>,
    pub before: (usize, usize),
    pub after: (usize, usize),
    pub kind: Kind,
    pub time: Instant,
}

#[derive(Default)]
pub struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
}

fn merge(last: &mut Change, next: &Change, kind: Kind) -> bool {
    let merged = match kind {
        Kind::Typing => next.removed.is_empty() && !next.inserted.contains('\n') && last.at + last.inserted.len() == next.at,
        Kind::Deleting => next.at + next.removed.len() == last.at || next.at == last.at,
        Kind::Block => next.at == last.at && last.inserted == next.removed,
        Kind::Other | Kind::Outside => false,
    };
    if !merged {
        return false;
    }
    match kind {
        Kind::Typing => last.inserted.push_str(&next.inserted),
        Kind::Deleting if next.at < last.at => {
            last.removed.insert_str(0, &next.removed);
            last.at = next.at;
        }
        Kind::Deleting => last.removed.push_str(&next.removed),
        _ => last.inserted.clone_from(&next.inserted),
    }
    true
}

impl History {
    pub fn record(&mut self, step: Step) {
        self.redo.clear();
        if let Some(last) = self.undo.last_mut()
            && last.kind == step.kind
            && step.time.duration_since(last.time) < GROUP
            && let ([previous], [next]) = (last.changes.as_mut_slice(), step.changes.as_slice())
            && merge(previous, next, step.kind)
        {
            last.after = step.after;
            last.time = step.time;
            return;
        }
        self.undo.push(step);
    }

    pub fn has(&self, redo: bool) -> bool {
        let steps = if redo { &self.redo } else { &self.undo };
        !steps.is_empty()
    }

    pub fn take(&mut self, redo: bool) -> Option<Step> {
        let (from, to) = if redo { (&mut self.redo, &mut self.undo) } else { (&mut self.undo, &mut self.redo) };
        let step = from.pop()?;
        to.push(step.clone());
        Some(step)
    }
}
