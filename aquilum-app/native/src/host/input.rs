use super::*;

const DOUBLE_CLICK: Duration = Duration::from_millis(500);
const DOUBLE_CLICK_SLOP: f64 = 4.0;

fn latin(code: Code) -> Option<char> {
    const LETTERS: [Code; 26] = [
        Code::KeyA, Code::KeyB, Code::KeyC, Code::KeyD, Code::KeyE, Code::KeyF, Code::KeyG, Code::KeyH, Code::KeyI,
        Code::KeyJ, Code::KeyK, Code::KeyL, Code::KeyM, Code::KeyN, Code::KeyO, Code::KeyP, Code::KeyQ, Code::KeyR,
        Code::KeyS, Code::KeyT, Code::KeyU, Code::KeyV, Code::KeyW, Code::KeyX, Code::KeyY, Code::KeyZ,
    ];
    LETTERS.iter().position(|&c| c == code).map(|i| char::from(b'a' + i as u8))
}

impl Host {
    pub(super) fn update_resize_edge(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let window = self.window();
        let edge = if NATIVE_FRAME || window.is_maximized() {
            None
        } else {
            let size = window.surface_size();
            let border = RESIZE_BORDER * window.scale_factor();
            let corner = border * 2.0;
            let (w, h) = (f64::from(size.width), f64::from(size.height));
            let (x, y) = (position.x, position.y);
            let left = x < border;
            let right = x > w - border;
            let top = y < border;
            let bottom = y > h - border;
            let near_left = x < corner;
            let near_right = x > w - corner;
            let near_top = y < corner;
            let near_bottom = y > h - corner;
            if (top && near_left) || (left && near_top) {
                Some(ResizeDirection::NorthWest)
            } else if (top && near_right) || (right && near_top) {
                Some(ResizeDirection::NorthEast)
            } else if (bottom && near_left) || (left && near_bottom) {
                Some(ResizeDirection::SouthWest)
            } else if (bottom && near_right) || (right && near_bottom) {
                Some(ResizeDirection::SouthEast)
            } else if top {
                Some(ResizeDirection::North)
            } else if bottom {
                Some(ResizeDirection::South)
            } else if left {
                Some(ResizeDirection::West)
            } else if right {
                Some(ResizeDirection::East)
            } else {
                None
            }
        };
        if edge != self.resize_edge {
            self.resize_edge = edge;
            let cursor = edge.map_or(self.cursor, CursorIcon::from);
            self.window().set_cursor(cursor.into());
        }
    }

    pub(super) fn time_ns(&self) -> u64 {
        self.started.elapsed().as_nanos() as u64
    }

    pub(super) fn click_count(&mut self, button: Option<PointerButton>, position: winit::dpi::PhysicalPosition<f64>) -> u8 {
        let now = Instant::now();
        let slop = DOUBLE_CLICK_SLOP * self.window().scale_factor();
        let count = match self.click {
            Some((last, at, from, count)) if button == Some(last) && now - at <= DOUBLE_CLICK && (from.x - position.x).abs() <= slop && (from.y - position.y).abs() <= slop => count.saturating_add(1),
            _ => 1,
        };
        self.click = button.map(|b| (b, now, position, count));
        count
    }

    pub(super) fn pointer_state(&mut self) -> PointerState {
        self.pointer.time = self.time_ns();
        self.pointer.modifiers = self.modifiers;
        self.pointer.scale_factor = self.scale();
        self.pointer.clone()
    }

    pub(super) fn pinch(&mut self, delta: f64) {
        if !delta.is_finite() || delta == 0.0 {
            return;
        }
        let state = self.pointer_state();
        let gesture = masonry::core::PointerGestureEvent { pointer: PRIMARY_MOUSE, gesture: masonry::core::PointerGesture::Pinch(delta as f32), state };
        self.pointer_event(PointerEvent::Gesture(gesture));
    }

    pub(super) fn pointer_event(&mut self, event: PointerEvent) {
        if let Some(root) = &mut self.root {
            root.handle_pointer_event(event);
        }
    }

    pub(super) fn text_event(&mut self, event: TextEvent) {
        if let Some(root) = &mut self.root {
            root.handle_text_event(event);
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
        if state == KeyState::Down
            && let Some(root) = &mut self.root
        {
            let modifiers = self.modifiers;
            if action_modifier && code == Code::KeyO {
                self.app.open_search(root);
                return;
            }
            #[cfg(not(feature = "installed"))]
            if action_modifier && modifiers.shift() && code == Code::KeyP {
                self.metrics.toggle_live();
                self.show_title();
                return;
            }
            if action_modifier && modifiers.shift() && code == Code::KeyF {
                self.app.toggle_focus_mode(root);
                return;
            }
            if action_modifier && !modifiers.shift() && code == Code::KeyN {
                if self.app.new_note(root) {
                    self.show_title();
                }
                return;
            }
            if action_modifier && !modifiers.shift() && code == Code::KeyF {
                self.app.open_find(root);
                return;
            }
            if action_modifier && !modifiers.shift() && code == Code::KeyU {
                self.app.open_templates(root);
                return;
            }
            if self.app.find_key(root, &key, modifiers) {
                return;
            }
            if let Some(outcome) = self.app.search_key(root, &key, modifiers) {
                if outcome.title {
                    self.show_title();
                }
                return;
            }
            if let Some(outcome) = self.app.templates_key(root, &key, modifiers) {
                if outcome.title {
                    self.show_title();
                }
                return;
            }
            if let Some(outcome) = self.app.settings_key(root, &key) {
                if outcome.rescale {
                    self.rescale();
                }
                return;
            }
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

fn png(image: arboard::ImageData<'_>) -> Option<Vec<u8>> {
    let rgba = image::RgbaImage::from_raw(image.width as u32, image.height as u32, image.bytes.into_owned())?;
    let mut out = std::io::Cursor::new(Vec::new());
    rgba.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}
