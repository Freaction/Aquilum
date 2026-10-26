use masonry::core::keyboard::KeyboardEvent;

use super::*;
use crate::ui::keymap::{self, Command, Scope};

impl TextEditor {
    pub(in super::super) fn table_key(&mut self, ctx: &mut EventCtx<'_>, event: &KeyboardEvent) -> bool {
        if !self.table_active() || event.state != KeyState::Down {
            return self.table_active();
        }
        let extend = keymap::extend(event.modifiers);
        let command = keymap::command(&[Scope::Text, Scope::Navigation, Scope::Table, Scope::Caret], &event.key, event.modifiers);
        let editing = self.table.as_ref().is_some_and(|s| s.active.is_some());
        if matches!(command, Some(Command::Undo | Command::Redo)) {
            self.leave_table();
            self.sync_editing();
            return false;
        }
        let Some((session, view)) = self.table_parts() else {
            self.table = None;
            return false;
        };
        let mut clipboard = None;
        let outcome = match (editing, command, &event.key) {
            (_, Some(Command::SelectAll), _) => {
                session.select_all(view);
                Outcome::Repaint
            }
            (false, Some(Command::Copy), _) => {
                clipboard = session.range_text(view);
                Outcome::None
            }
            (false, Some(Command::Cut), _) => {
                clipboard = session.range_text(view);
                session.clear_range(view)
            }
            (false, Some(Command::DeleteBackward | Command::DeleteForward | Command::DeleteWord | Command::DeleteWordForward), _) => session.clear_range(view),
            (false, Some(Command::Close), _) => {
                session.range = None;
                Outcome::Repaint
            }
            (false, _, _) => Outcome::None,
            (true, Some(Command::Copy), _) => {
                clipboard = session.selected(view);
                Outcome::None
            }
            (true, Some(Command::Cut), _) => {
                clipboard = session.selected(view);
                if clipboard.is_some() { session.insert(view, "") } else { Outcome::None }
            }
            (true, Some(Command::NextCell | Command::PreviousCell), _) => session.next(view, command == Some(Command::PreviousCell)),
            (true, Some(Command::CellBelow), _) => session.down(view),
            (true, Some(Command::Close), _) => Outcome::Leave,
            (true, Some(delete @ (Command::DeleteBackward | Command::DeleteWord)), _) => session.delete(view, false, delete == Command::DeleteWord),
            (true, Some(Command::LineDeleteBackward), _) => {
                if session.caret == session.anchor {
                    session.edge(view, false, true);
                }
                session.insert(view, "")
            }
            (true, Some(delete @ (Command::DeleteForward | Command::DeleteWordForward)), _) => session.delete(view, true, delete == Command::DeleteWordForward),
            (true, Some(step @ (Command::CharLeft | Command::CharRight | Command::WordLeft | Command::WordRight)), _) => {
                session.step(view, matches!(step, Command::CharRight | Command::WordRight), matches!(step, Command::WordLeft | Command::WordRight), extend);
                Outcome::Repaint
            }
            (true, Some(Command::LineStart | Command::LineUp | Command::DocumentStart | Command::ScrollLineUp), _) => {
                session.edge(view, false, extend);
                Outcome::Repaint
            }
            (true, Some(Command::LineEnd | Command::LineDown | Command::DocumentEnd | Command::ScrollLineDown), _) => {
                session.edge(view, true, extend);
                Outcome::Repaint
            }
            (true, None, Key::Character(text)) if keymap::typing(event.modifiers) => session.insert(view, text),
            (true, _, _) => Outcome::None,
        };
        if let Some(text) = clipboard {
            ctx.set_clipboard(text);
        }
        self.apply(ctx, outcome);
        ctx.set_handled();
        true
    }

    pub(in super::super) fn table_text(&mut self, ctx: &mut EventCtx<'_>, text: &str) -> bool {
        self.table_edit(ctx, |session, view| session.insert(view, text))
    }

    pub(in super::super) fn table_delete(&mut self, ctx: &mut EventCtx<'_>, before: usize, after: usize) -> bool {
        self.table_edit(ctx, |session, view| session.delete_around(view, before, after))
    }

    fn table_edit(&mut self, ctx: &mut EventCtx<'_>, edit: impl FnOnce(&mut Session, &TableView) -> Outcome) -> bool {
        if !self.table.as_ref().is_some_and(|s| s.active.is_some()) {
            return false;
        }
        let Some((session, view)) = self.table_parts() else { return false };
        let outcome = edit(session, view);
        self.apply(ctx, outcome);
        ctx.set_handled();
        true
    }
}
