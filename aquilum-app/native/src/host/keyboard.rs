use super::*;
use crate::ui::keymap::{self, Command, Scope};

impl Host {
    pub(super) fn text_event(&mut self, event: TextEvent) {
        if let Some(root) = &mut self.root {
            root.handle_text_event(event);
        }
    }

    #[cfg(not(feature = "installed"))]
    fn debug_command(&mut self, command: Command) -> bool {
        match command {
            Command::PaintFlashing => {
                self.renderer.toggle_overlay();
                self.request_redraw();
            }
            Command::LiveMetrics => {
                self.metrics.toggle_live();
                self.show_title();
            }
            _ => return false,
        }
        true
    }

    fn shortcut(&mut self, key: &Key, repeat: bool) -> bool {
        let modifiers = self.modifiers;
        let command = keymap::command(&[Scope::Global], key, modifiers);
        #[cfg(not(feature = "installed"))]
        if command.is_some_and(|command| self.debug_command(command)) {
            return true;
        }
        let Some(root) = &mut self.root else { return false };
        if let Some(command) = command
            && (!repeat || command.repeats())
            && let Some(outcome) = self.app.command(root, command)
        {
            self.apply(outcome);
            return true;
        }
        if self.app.find_key(root, key, modifiers) {
            return true;
        }
        let outcome = self.app.search_key(root, key, modifiers).or_else(|| self.app.templates_key(root, key, modifiers)).or_else(|| self.app.settings_key(root, key, modifiers));
        let Some(outcome) = outcome else { return false };
        self.apply(outcome);
        true
    }

    pub(super) fn apply(&mut self, outcome: crate::app::Outcome) {
        if outcome.title {
            self.show_title();
        }
        if outcome.rescale {
            self.rescale();
            if let Some(root) = &mut self.root {
                self.app.scale_changed(root);
            }
        }
    }

    pub(super) fn keyboard(&mut self, event: winit::event::KeyEvent) {
        let key = match event.logical_key {
            wk::Key::Named(named) => Key::Named(named),
            wk::Key::Character(s) => Key::Character(s.to_string()),
            wk::Key::Dead(_) => Key::Named(NamedKey::Dead),
            wk::Key::Unidentified(_) => Key::Named(NamedKey::Unidentified),
        };
        let code = match event.physical_key {
            PhysicalKey::Code(code) => code,
            PhysicalKey::Unidentified(_) => Code::Unidentified,
        };
        let state = match event.state {
            ElementState::Pressed => KeyState::Down,
            ElementState::Released => KeyState::Up,
        };
        let key = keymap::shortcut_key(key, code, self.modifiers);
        if state == KeyState::Down && self.shortcut(&key, event.repeat) {
            return;
        }
        if state == KeyState::Down
            && let Some(paste @ (Command::Paste | Command::PastePlain)) = keymap::command(&[Scope::Text], &key, self.modifiers)
            && let Some(root) = &mut self.root
            && self.app.paste(root, paste == Command::PastePlain)
        {
            return;
        }
        let location: Location = event.location;
        self.text_event(TextEvent::Keyboard(KeyboardEvent {
            state,
            key,
            code,
            location,
            modifiers: self.modifiers,
            repeat: event.repeat,
            is_composing: false,
        }));
    }
}
