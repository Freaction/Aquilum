use super::*;

impl TextEditor {
    pub(super) fn keyboard(&mut self, ctx: &mut EventCtx<'_>, event: &masonry::core::keyboard::KeyboardEvent) {
        if event.state != KeyState::Down || self.compose.is_some() {
            return;
        }
        let shift = event.modifiers.shift();
        let action = if cfg!(target_os = "macos") { event.modifiers.meta() } else { event.modifiers.ctrl() };
        let vertical = matches!(event.key, Key::Named(NamedKey::ArrowUp | NamedKey::ArrowDown));
        if !vertical {
            self.column = None;
        }
        if !action
            && let Some(edited) = self.suggest_key(ctx.widget_id(), &event.key)
        {
            ctx.set_handled();
            if edited {
                self.changed(ctx);
            } else {
                self.refresh(ctx);
            }
            return;
        }
        let typing = matches!(&event.key, Key::Character(_) | Key::Named(NamedKey::Backspace | NamedKey::Delete)) && !action;
        let mut edited = false;
        match &event.key {
            Key::Character(c) if action && c.eq_ignore_ascii_case("x") => {
                if let Some(text) = self.selected_text() {
                    self.insert("", Kind::Other);
                    ctx.set_clipboard(text);
                    edited = true;
                }
            }
            Key::Character(c) if action && c.eq_ignore_ascii_case("c") => {
                if let Some(text) = self.selected_text() {
                    ctx.set_clipboard(text);
                }
            }
            Key::Character(c) if action && (c.eq_ignore_ascii_case("z") || c.eq_ignore_ascii_case("y")) => {
                let redo = c.eq_ignore_ascii_case("y") || shift;
                edited = self.undo(redo);
            }
            Key::Character(c) if action && c.eq_ignore_ascii_case("a") => {
                self.anchor = 0;
                self.focus = self.doc.len();
            }
            Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowRight) => {
                let forward = matches!(event.key, Key::Named(NamedKey::ArrowRight));
                let range = self.selection();
                if !shift && !action && !range.is_empty() {
                    let edge = if forward { range.end } else { range.start };
                    self.move_to((edge, Affinity::Downstream), false);
                } else {
                    let target = self.step(forward, action);
                    self.move_to(target, shift);
                    self.skip_tables(forward);
                }
            }
            Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::ArrowDown) => {
                let target = self.vertical(matches!(event.key, Key::Named(NamedKey::ArrowDown)));
                self.move_to(target, shift);
                self.skip_tables(matches!(event.key, Key::Named(NamedKey::ArrowDown)));
            }
            Key::Named(NamedKey::Home) | Key::Named(NamedKey::End) => {
                let end = matches!(event.key, Key::Named(NamedKey::End));
                let target = match (action, end) {
                    (true, false) => (0, Affinity::Downstream),
                    (true, true) => (self.doc.len(), Affinity::Downstream),
                    (false, end) => self.line_edge(end),
                };
                self.move_to(target, shift);
            }
            Key::Named(NamedKey::Backspace) => {
                self.delete(false, action);
                edited = true;
            }
            Key::Named(NamedKey::Delete) => {
                self.delete(true, action);
                edited = true;
            }
            Key::Named(NamedKey::Enter) if self.single_line => {
                ctx.submit_action::<TextAction>(TextAction::Entered(self.doc.text().to_owned()));
            }
            Key::Named(NamedKey::Enter) => {
                if self.doc.markdown.is_none() || !self.newline(shift) {
                    self.insert("\n", Kind::Other);
                }
                edited = true;
            }
            Key::Named(NamedKey::Tab) if self.single_line || self.doc.markdown.is_none() => return,
            Key::Named(NamedKey::Tab) => edited = self.indent(!shift),
            Key::Named(NamedKey::Escape) => ctx.submit_action::<TextAction>(TextAction::Cancelled),
            Key::Character(text) if !action => {
                self.insert(text, Kind::Typing);
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
            Ime::Preedit(text, cursor) if text.is_empty() => {
                self.compose = None;
                let _ = cursor;
            }
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

