use super::model::{Align, Range2};
use super::session::{Outcome, Session};
use super::view::TableView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Merge,
    Unmerge,
    AlignLeft,
    AlignCenter,
    AlignRight,
    DeleteRow,
    DeleteColumn,
    DeleteTable,
}

impl Command {
    pub fn icon(self) -> crate::ui::icons::Icon {
        use crate::ui::icons;
        match self {
            Command::Merge => icons::TABLE_MERGE,
            Command::Unmerge => icons::TABLE_SPLIT,
            Command::AlignLeft => icons::ALIGN_LEFT,
            Command::AlignCenter => icons::ALIGN_CENTER,
            Command::AlignRight => icons::ALIGN_RIGHT,
            Command::DeleteRow => icons::ROWS,
            Command::DeleteColumn => icons::COLUMNS,
            Command::DeleteTable => icons::TRASH,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Command::Merge => "editor.table.merge",
            Command::Unmerge => "editor.table.unmerge",
            Command::AlignLeft => "editor.table.alignLeft",
            Command::AlignCenter => "editor.table.alignCenter",
            Command::AlignRight => "editor.table.alignRight",
            Command::DeleteRow => "editor.table.deleteRow",
            Command::DeleteColumn => "editor.table.deleteColumn",
            Command::DeleteTable => "editor.table.deleteTable",
        }
    }
}

impl Session {
    pub fn context(&mut self, view: &TableView, r: usize, c: usize) -> Vec<(Command, bool)> {
        if !self.range.is_some_and(|g| g.contains(r, c)) {
            let (r, c) = view.model.anchor(r, c);
            self.active = None;
            self.range = Some(view.model.region(r, c));
        }
        self.menu_cell = Some((r, c));
        let model = &view.model;
        let range = self.range.unwrap_or(Range2::cell(r, c));
        let align = model.aligns.get(c).copied().flatten();
        vec![
            (Command::Merge, !range.single() && model.can_merge(range) && model.merge_at(range.top, range.left) != Some(range)),
            (Command::Unmerge, model.merge_at(r, c).is_some()),
            (Command::AlignLeft, align != Some(Align::Left)),
            (Command::AlignCenter, align != Some(Align::Center)),
            (Command::AlignRight, align != Some(Align::Right)),
            (Command::DeleteRow, model.rows() > 2),
            (Command::DeleteColumn, model.cols() > 1),
            (Command::DeleteTable, true),
        ]
    }

    pub fn run(&mut self, view: &TableView, command: Command) -> Outcome {
        let Some((r, c)) = self.menu_cell.take() else { return Outcome::None };
        let mut model = view.model.clone();
        let (ar, ac) = model.anchor(r, c);
        let resume = match command {
            Command::Merge => {
                let Some(range) = self.range else { return Outcome::None };
                if !model.merge(range) {
                    return Outcome::None;
                }
                (range.top, range.left)
            }
            Command::Unmerge => {
                model.unmerge(r, c);
                (ar, ac)
            }
            Command::AlignLeft | Command::AlignCenter | Command::AlignRight => {
                let align = match command {
                    Command::AlignLeft => Align::Left,
                    Command::AlignCenter => Align::Center,
                    _ => Align::Right,
                };
                model.set_align(ac, Some(align));
                (0, ac)
            }
            Command::DeleteRow => {
                if !model.delete_row(r) {
                    return Outcome::None;
                }
                (r.min(model.rows() - 1), c.min(model.cols() - 1))
            }
            Command::DeleteColumn => {
                if !model.delete_col(c) {
                    return Outcome::None;
                }
                (r.min(model.rows() - 1), c.min(model.cols() - 1))
            }
            Command::DeleteTable => return Outcome::Remove,
        };
        self.range = None;
        self.active = Some(model.anchor(resume.0, resume.1));
        self.caret = model.value(resume.0, resume.1).len();
        self.anchor = self.caret;
        Outcome::Write(model)
    }
}
