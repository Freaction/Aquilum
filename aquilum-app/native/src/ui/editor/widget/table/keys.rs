use masonry::core::keyboard::KeyboardEvent;

use super::*;
use crate::ui::keymap::{self, Command, Scope};

impl TextEditor {
    pub(in super::super) fn table_key(&mut self, ctx: &mut EventCtx<'_>, event: &KeyboardEvent) -> bool {
        if !self.table_active() || event.state != KeyState::Down {
            return self.table_active();
        }
        let shift = event.modifiers.shift();
        let command = keymap::command(&[Scope::Text, Scope::Navigation, Scope::Table], &event.key, event.modifiers);
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
            (false, _, Key::Named(NamedKey::Delete | NamedKey::Backspace)) => session.clear_range(view),
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
            (true, _, Key::Named(NamedKey::Backspace)) => session.delete(view, false, command == Some(Command::DeleteWord)),
            (true, _, Key::Named(NamedKey::Delete)) => session.delete(view, true, command == Some(Command::DeleteWordForward)),
            (true, _, Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowRight)) => {
                session.step(view, matches!(event.key, Key::Named(NamedKey::ArrowRight)), matches!(command, Some(Command::WordLeft | Command::WordRight)), shift);
                Outcome::Repaint
            }
            (true, _, Key::Named(NamedKey::Home | NamedKey::ArrowUp)) => {
                session.edge(view, false, shift);
                Outcome::Repaint
            }
            (true, _, Key::Named(NamedKey::End | NamedKey::ArrowDown)) => {
                session.edge(view, true, shift);
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
        if !self.table.as_ref().is_some_and(|s| s.active.is_some()) {
            return false;
        }
        let Some((session, view)) = self.table_parts() else { return false };
        let outcome = session.insert(view, text);
        self.apply(ctx, outcome);
        ctx.set_handled();
        true
    }
}
