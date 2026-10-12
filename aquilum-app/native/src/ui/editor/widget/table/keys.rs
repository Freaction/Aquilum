use masonry::core::keyboard::KeyboardEvent;

use super::*;

impl TextEditor {
    pub(in super::super) fn table_key(&mut self, ctx: &mut EventCtx<'_>, event: &KeyboardEvent) -> bool {
        if !self.table_active() || event.state != KeyState::Down {
            return self.table_active();
        }
        let shift = event.modifiers.shift();
        let action = if cfg!(target_os = "macos") { event.modifiers.meta() } else { event.modifiers.ctrl() };
        let editing = self.table.as_ref().is_some_and(|s| s.active.is_some());
        if editing && action && matches!(&event.key, Key::Character(c) if c.eq_ignore_ascii_case("z") || c.eq_ignore_ascii_case("y")) {
            self.leave_table();
            self.sync_editing();
            return false;
        }
        let Some((session, view)) = self.table_parts() else {
            self.table = None;
            return false;
        };
        let mut clipboard = None;
        let outcome = if !editing {
            match &event.key {
                Key::Named(NamedKey::Delete | NamedKey::Backspace) => session.clear_range(view),
                Key::Named(NamedKey::Escape) => {
                    session.range = None;
                    Outcome::Repaint
                }
                _ => Outcome::None,
            }
        } else {
            match &event.key {
                Key::Character(c) if action && c.eq_ignore_ascii_case("a") => {
                    session.select_all(view);
                    Outcome::Repaint
                }
                Key::Character(c) if action && c.eq_ignore_ascii_case("c") => {
                    clipboard = session.selected(view);
                    Outcome::None
                }
                Key::Character(c) if action && c.eq_ignore_ascii_case("x") => {
                    clipboard = session.selected(view);
                    if clipboard.is_some() { session.insert(view, "") } else { Outcome::None }
                }
                Key::Character(text) if !action => session.insert(view, text),
                Key::Named(NamedKey::Backspace) => session.delete(view, false, action),
                Key::Named(NamedKey::Delete) => session.delete(view, true, action),
                Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowRight) => {
                    session.step(view, matches!(event.key, Key::Named(NamedKey::ArrowRight)), action, shift);
                    Outcome::Repaint
                }
                Key::Named(NamedKey::Home) | Key::Named(NamedKey::ArrowUp) => {
                    session.edge(view, false, shift);
                    Outcome::Repaint
                }
                Key::Named(NamedKey::End) | Key::Named(NamedKey::ArrowDown) => {
                    session.edge(view, true, shift);
                    Outcome::Repaint
                }
                Key::Named(NamedKey::Enter) => session.down(view),
                Key::Named(NamedKey::Tab) => session.next(view, shift),
                Key::Named(NamedKey::Escape) => Outcome::Leave,
                _ => Outcome::None,
            }
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
