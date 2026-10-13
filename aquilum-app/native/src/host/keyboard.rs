use super::*;

fn latin(code: Code) -> Option<char> {
    const LETTERS: [Code; 26] = [
        Code::KeyA, Code::KeyB, Code::KeyC, Code::KeyD, Code::KeyE, Code::KeyF, Code::KeyG, Code::KeyH, Code::KeyI,
        Code::KeyJ, Code::KeyK, Code::KeyL, Code::KeyM, Code::KeyN, Code::KeyO, Code::KeyP, Code::KeyQ, Code::KeyR,
        Code::KeyS, Code::KeyT, Code::KeyU, Code::KeyV, Code::KeyW, Code::KeyX, Code::KeyY, Code::KeyZ,
    ];
    LETTERS.iter().position(|&c| c == code).map(|i| char::from(b'a' + i as u8))
}

fn png(image: arboard::ImageData<'_>) -> Option<Vec<u8>> {
    let rgba = image::RgbaImage::from_raw(image.width as u32, image.height as u32, image.bytes.into_owned())?;
    let mut out = std::io::Cursor::new(Vec::new());
    rgba.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

impl Host {
    pub(super) fn text_event(&mut self, event: TextEvent) {
        if let Some(root) = &mut self.root {
            root.handle_text_event(event);
        }
    }

    fn shortcut(&mut self, key: &Key, code: Code, action_modifier: bool) -> bool {
        let Some(root) = &mut self.root else { return false };
        let modifiers = self.modifiers;
        if action_modifier && code == Code::KeyO {
            self.app.open_search(root);
            return true;
        }
        #[cfg(not(feature = "installed"))]
        if action_modifier && modifiers.shift() && code == Code::KeyP {
            self.metrics.toggle_live();
            self.show_title();
            return true;
        }
        if action_modifier && modifiers.shift() && code == Code::KeyF {
            self.app.toggle_focus_mode(root);
            return true;
        }
        if action_modifier && modifiers.shift() && code == Code::KeyL {
            self.app.toggle_wide_mode(root);
            return true;
        }
        if action_modifier && !modifiers.shift() && code == Code::KeyN {
            if self.app.new_note(root) {
                self.show_title();
            }
            return true;
        }
        if action_modifier && !modifiers.shift() && code == Code::KeyF {
            self.app.open_find(root);
            return true;
        }
        if action_modifier && !modifiers.shift() && code == Code::KeyU {
            self.app.open_templates(root);
            return true;
        }
        if self.app.find_key(root, key, modifiers) {
            return true;
        }
        if let Some(outcome) = self.app.search_key(root, key, modifiers) {
            if outcome.title {
                self.show_title();
            }
            return true;
        }
        if let Some(outcome) = self.app.templates_key(root, key, modifiers) {
            if outcome.title {
                self.show_title();
            }
            return true;
        }
        if let Some(outcome) = self.app.settings_key(root, key) {
            if outcome.rescale {
                self.rescale();
            }
            return true;
        }
        false
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
        let action_modifier = if cfg!(target_os = "macos") { self.modifiers.meta() } else { self.modifiers.ctrl() };
        let key = match latin(code) {
            Some(letter) if action_modifier && !self.modifiers.alt() => Key::Character(letter.to_string()),
            _ => key,
        };
        #[cfg(not(feature = "installed"))]
        if state == KeyState::Down && code == Code::F12 {
            self.renderer.toggle_overlay();
            self.request_redraw();
            return;
        }
        if state == KeyState::Down && self.shortcut(&key, code, action_modifier) {
            return;
        }
        if state == KeyState::Down
            && action_modifier
            && let Key::Character(c) = &key
            && self.app.zoom_key(c)
        {
            self.rescale();
            if let Some(root) = &mut self.root {
                self.app.scale_changed(root);
            }
            return;
        }
        let is_paste = state == KeyState::Down
            && action_modifier
            && matches!(&key, Key::Character(c) if c.eq_ignore_ascii_case("v"));
        if is_paste {
            let Some(clipboard) = self.clipboard.as_mut() else { return };
            if let Ok(text) = clipboard.get_text() {
                self.text_event(TextEvent::ClipboardPaste(text));
            } else if let Ok(image) = clipboard.get_image()
                && let Some(png) = png(image)
                && let Some(root) = &mut self.root
            {
                self.app.paste_image(root, &png);
            }
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
