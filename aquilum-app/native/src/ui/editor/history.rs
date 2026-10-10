use std::time::{Duration, Instant};

const GROUP: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Typing,
    Deleting,
    Other,
    Table,
}

#[derive(Clone, Debug)]
pub struct Edit {
    pub at: usize,
    pub removed: String,
    pub inserted: String,
    pub before: (usize, usize),
    pub after: (usize, usize),
    pub kind: Kind,
    pub time: Instant,
}

#[derive(Default)]
pub struct History {
    undo: Vec<Edit>,
    redo: Vec<Edit>,
}

impl History {
    pub fn record(&mut self, edit: Edit) {
        self.redo.clear();
        if let Some(last) = self.undo.last_mut()
            && last.kind == edit.kind
            && edit.time.duration_since(last.time) < GROUP
        {
            let typed = edit.kind == Kind::Typing && edit.removed.is_empty() && !edit.inserted.contains('\n') && last.at + last.inserted.len() == edit.at;
            let backward = edit.kind == Kind::Deleting && edit.at + edit.removed.len() == last.at;
            let forward = edit.kind == Kind::Deleting && edit.at == last.at;
            let table = edit.kind == Kind::Table && edit.at == last.at && last.inserted == edit.removed;
            if table {
                last.inserted = edit.inserted;
                last.after = edit.after;
                last.time = edit.time;
                return;
            }
            if typed {
                last.inserted.push_str(&edit.inserted);
            } else if backward {
                last.removed.insert_str(0, &edit.removed);
                last.at = edit.at;
            } else if forward {
                last.removed.push_str(&edit.removed);
            }
            if typed || backward || forward {
                last.after = edit.after;
                last.time = edit.time;
                return;
            }
        }
        self.undo.push(edit);
    }

    pub fn undo(&mut self) -> Option<Edit> {
        let edit = self.undo.pop()?;
        self.redo.push(edit.clone());
        Some(edit)
    }

    pub fn redo(&mut self) -> Option<Edit> {
        let edit = self.redo.pop()?;
        self.undo.push(edit.clone());
        Some(edit)
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
