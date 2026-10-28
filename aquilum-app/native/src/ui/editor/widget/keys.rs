use super::*;
use crate::ui::editor::markdown::Mark;
use crate::ui::keymap::{self, Command, Scope};

impl TextEditor {
    pub(super) fn keyboard(&mut self, ctx: &mut EventCtx<'_>, event: &masonry::core::keyboard::KeyboardEvent) {
        if event.state != KeyState::Down || self.compose.is_some() {
            return;
        }
        let extend = keymap::extend(event.modifiers);
        let typed = keymap::typing(event.modifiers);
        let command = keymap::command(&[Scope::Text, Scope::Editor, Scope::Navigation, Scope::Caret], &event.key, event.modifiers);
        let vertical = matches!(command, Some(Command::LineUp | Command::LineDown | Command::PageUp | Command::PageDown | Command::ScrollLineUp | Command::ScrollLineDown));
        if !vertical {
            self.column = None;
        }
        if typed
            && let Some(edited) = self.suggest_key(ctx.widget_id(), &event.key, event.modifiers)
        {
            ctx.set_handled();
            if edited {
                self.follow_caret = true;
                self.changed(ctx);
            } else {
                self.refresh(ctx);
            }
            return;
        }
        let rich = !self.single_line && self.doc.markdown.is_some();
        let typing = typed && (matches!(&event.key, Key::Character(_)) && command.is_none() || matches!(command, Some(Command::DeleteBackward | Command::DeleteForward)));
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
            (Some(Command::Undo | Command::Redo), _) => edited = self.rewind(command == Some(Command::Redo)),
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
                ctx.submit_action::<TextAction>(TextAction::Page { down: command == Some(Command::PageDown), extend });
                ctx.set_handled();
                self.close_suggest();
                return;
            }
            (Some(Command::ScrollLineUp | Command::ScrollLineDown), _) if !self.single_line => {
                ctx.submit_action::<TextAction>(TextAction::Line { down: command == Some(Command::ScrollLineDown) });
                ctx.set_handled();
                return;
            }
            (Some(step @ (Command::CharLeft | Command::CharRight | Command::WordLeft | Command::WordRight)), _) => {
                let forward = matches!(step, Command::CharRight | Command::WordRight);
                let word = matches!(step, Command::WordLeft | Command::WordRight);
                let range = self.selection();
                if !extend && !word && !range.is_empty() {
                    let edge = if forward { range.end } else { range.start };
                    self.move_to((edge, Affinity::Downstream), false);
                } else {
                    let target = self.step(forward, word);
                    self.move_to(target, extend);
                    self.skip_tables(forward);
                }
            }
            (Some(line @ (Command::LineUp | Command::LineDown)), _) => {
                let down = line == Command::LineDown;
                let target = self.vertical(down);
                self.move_to(target, extend);
                self.skip_tables(down);
            }
            (Some(edge @ (Command::LineStart | Command::LineEnd | Command::DocumentStart | Command::DocumentEnd)), _) => {
                let target = match edge {
                    Command::DocumentStart => (0, Affinity::Downstream),
                    Command::DocumentEnd => (self.doc.len(), Affinity::Downstream),
                    _ => self.line_edge(edge == Command::LineEnd),
                };
                self.move_to(target, extend);
            }
            (Some(delete @ (Command::DeleteBackward | Command::DeleteWord)), _) => {
                if !self.apply_multicursor_delete(false) && !self.handle_pair_backspace() {
                    self.delete(false, delete == Command::DeleteWord);
                }
                edited = true;
            }
            (Some(Command::LineDeleteBackward), _) => {
                if !self.apply_multicursor_delete(false) {
                    self.delete_to_line_start();
                }
                edited = true;
            }
            (Some(delete @ (Command::DeleteForward | Command::DeleteWordForward)), _) => {
                if !self.apply_multicursor_delete(true) {
                    self.delete(true, delete == Command::DeleteWordForward);
                }
                edited = true;
            }
            (Some(Command::NewLine | Command::LineBreak), _) if self.single_line => {
                ctx.submit_action::<TextAction>(TextAction::Entered(self.doc.whole()));
            }
            (Some(newline @ (Command::NewLine | Command::LineBreak)), _) => {
                if self.doc.markdown.is_none() || !self.newline(newline == Command::LineBreak) {
                    self.insert("\n", Kind::Other);
                }
                edited = true;
            }
            (Some(Command::Close), _) => {
                if self.clear_extra_cursors() {
                    ctx.set_handled();
                    self.refresh(ctx);
                    return;
                }
                ctx.submit_action::<TextAction>(TextAction::Cancelled);
                if self.renaming {
                    ctx.set_handled();
                }
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
            self.follow_caret = true;
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
}

