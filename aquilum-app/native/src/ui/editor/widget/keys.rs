use super::*;
use crate::ui::editor::markdown::Mark;
use crate::ui::keymap::{self, Command, Scope};

impl TextEditor {
    pub(super) fn keyboard(&mut self, ctx: &mut EventCtx<'_>, event: &masonry::core::keyboard::KeyboardEvent) {
        if event.state != KeyState::Down || self.compose.is_some() {
            return;
        }
        let shift = event.modifiers.shift();
        let typed = keymap::typing(event.modifiers);
        let vertical = matches!(event.key, Key::Named(NamedKey::ArrowUp | NamedKey::ArrowDown | NamedKey::PageUp | NamedKey::PageDown));
        if !vertical {
            self.column = None;
        }
        if typed
            && let Some(edited) = self.suggest_key(ctx.widget_id(), &event.key, event.modifiers)
        {
            ctx.set_handled();
            if edited {
                self.changed(ctx);
            } else {
                self.refresh(ctx);
            }
            return;
        }
        let command = keymap::command(&[Scope::Text, Scope::Editor, Scope::Navigation], &event.key, event.modifiers);
        let rich = !self.single_line && self.doc.markdown.is_some();
        let typing = matches!(&event.key, Key::Character(_) | Key::Named(NamedKey::Backspace | NamedKey::Delete)) && typed && command.is_none();
        let mut edited = false;
        match (command, &event.key) {
            (Some(Command::Cut), _) => {
                if let Some(text) = self.selected_text() {
                    self.insert("", Kind::Other);
                    ctx.set_clipboard(text);
                    edited = true;
                }
            }
            (Some(Command::Copy), _) => {
                if let Some(text) = self.selected_text() {
                    ctx.set_clipboard(text);
                }
            }
            (Some(Command::Undo | Command::Redo), _) => edited = self.undo(command == Some(Command::Redo)),
            (Some(Command::SelectAll), _) => {
                self.anchor = 0;
                self.focus = self.doc.len();
            }
            (Some(Command::Bold | Command::Italic | Command::Strike | Command::Code), _) if rich => {
                let mark = match command {
                    Some(Command::Bold) => Mark::Bold,
                    Some(Command::Italic) => Mark::Italic,
                    Some(Command::Code) => Mark::Code,
                    _ => Mark::Strike,
                };
                edited = self.toggle_mark(mark);
            }
            (Some(Command::Indent | Command::Outdent), _) if rich => edited = self.indent(command == Some(Command::Indent)),
            (Some(Command::PageUp | Command::PageDown), _) if !self.single_line => {
                ctx.submit_action::<TextAction>(TextAction::Page { down: command == Some(Command::PageDown), extend: shift });
                ctx.set_handled();
                self.close_suggest();
                return;
            }
            (Some(Command::ScrollLineUp | Command::ScrollLineDown), _) if !self.single_line => {
                ctx.submit_action::<TextAction>(TextAction::Line { down: command == Some(Command::ScrollLineDown) });
                ctx.set_handled();
                return;
            }
            (_, Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowRight)) => {
                let forward = matches!(event.key, Key::Named(NamedKey::ArrowRight));
                let word = matches!(command, Some(Command::WordLeft | Command::WordRight));
                let range = self.selection();
                if !shift && !word && !range.is_empty() {
                    let edge = if forward { range.end } else { range.start };
                    self.move_to((edge, Affinity::Downstream), false);
                } else {
                    let target = self.step(forward, word);
                    self.move_to(target, shift);
                    self.skip_tables(forward);
                }
            }
            (_, Key::Named(NamedKey::ArrowUp | NamedKey::ArrowDown)) => {
                let target = self.vertical(matches!(event.key, Key::Named(NamedKey::ArrowDown)));
                self.move_to(target, shift);
                self.skip_tables(matches!(event.key, Key::Named(NamedKey::ArrowDown)));
            }
            (_, Key::Named(NamedKey::Home | NamedKey::End)) => {
                let target = match command {
                    Some(Command::DocumentStart) => (0, Affinity::Downstream),
                    Some(Command::DocumentEnd) => (self.doc.len(), Affinity::Downstream),
                    _ => self.line_edge(matches!(event.key, Key::Named(NamedKey::End))),
                };
                self.move_to(target, shift);
            }
            (_, Key::Named(NamedKey::Backspace)) => {
                if !self.apply_multicursor_delete(false) && !self.handle_pair_backspace() {
                    self.delete(false, command == Some(Command::DeleteWord));
                }
                edited = true;
            }
            (_, Key::Named(NamedKey::Delete)) => {
                if !self.apply_multicursor_delete(true) {
                    self.delete(true, command == Some(Command::DeleteWordForward));
                }
                edited = true;
            }
            (_, Key::Named(NamedKey::Enter)) if self.single_line => {
                ctx.submit_action::<TextAction>(TextAction::Entered(self.doc.whole()));
            }
            (_, Key::Named(NamedKey::Enter)) => {
                if self.doc.markdown.is_none() || !self.newline(command == Some(Command::LineBreak)) {
                    self.insert("\n", Kind::Other);
                }
                edited = true;
            }
            (_, Key::Named(NamedKey::Escape)) => {
                if self.clear_extra_cursors() {
                    ctx.set_handled();
                    self.refresh(ctx);
                    return;
                }
                ctx.submit_action::<TextAction>(TextAction::Cancelled);
                if self.single_line {
                    self.close_suggest();
                    return;
                }
            }
            (_, Key::Character(text)) if typed => {
                if !self.apply_multicursor_typing(text) && !self.handle_pair_typing(text) && !self.smart_dash(text) {
                    self.insert(text, Kind::Typing);
                }
                edited = true;
            }
            _ => return,
        }
        ctx.set_handled();
        if edited {
            self.changed(ctx);
        } else {
            self.refresh(ctx);
        }
        if typing && edited {
            self.update_suggest(ctx.widget_id());
        } else {
            self.close_suggest();
        }
    }

    pub(super) fn ime(&mut self, ctx: &mut EventCtx<'_>, ime: &Ime) {
        let mut edited = false;
        match ime {
            Ime::Enabled => {}
            Ime::Disabled => self.compose = None,
            Ime::Preedit(text, _) if text.is_empty() => self.compose = None,
            Ime::Preedit(text, cursor) => {
                if !self.selection().is_empty() {
                    self.insert("", Kind::Other);
                    edited = true;
                }
                self.compose = Some((text.clone(), *cursor));
            }
            Ime::Commit(text) => {
                self.compose = None;
                self.insert(text, Kind::Other);
                edited = true;
            }
        }
        if edited {
            self.update_suggest(ctx.widget_id());
        }
        self.doc.invalidate(self.focus);
        ctx.set_handled();
        if edited {
            self.changed(ctx);
        } else {
            ctx.request_layout();
            ctx.request_render();
        }
    }
}

