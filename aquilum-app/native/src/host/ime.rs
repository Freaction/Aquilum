use super::*;
use crate::ui::editor::TextEditor;
use winit::window::{ImeHint, ImeRequest, ImeRequestData};
#[cfg(not(target_os = "android"))]
use winit::window::{ImeCapabilities, ImeEnableRequest, ImePurpose, ImeSurroundingText};

fn focused_editor(root: &RenderRoot) -> Option<&TextEditor> {
    Some(root.get_widget(root.focused_widget()?)?.downcast::<TextEditor>()?.inner())
}

#[cfg(not(target_os = "android"))]
fn surrounding(editor: Option<&TextEditor>) -> ImeSurroundingText {
    let (_, text, cursor, anchor) = editor.map(|editor| editor.surrounding(None)).unwrap_or_default();
    ImeSurroundingText::new(text, cursor, anchor).expect("выдержка редактора короче 4000 байт и на границах символов")
}

fn hint(editor: Option<&TextEditor>) -> ImeHint {
    if editor.is_some_and(TextEditor::multiline) {
        ImeHint::MULTILINE | ImeHint::AUTO_CAPITALIZATION | ImeHint::SPELLCHECK | ImeHint::COMPLETION
    } else {
        ImeHint::COMPLETION
    }
}

impl Host {
    #[cfg(not(target_os = "android"))]
    pub(super) fn ime_start(&mut self, window: &dyn Window) {
        if self.ime_enabled {
            return;
        }
        let editor = self.root.as_ref().and_then(focused_editor);
        let text = surrounding(editor);
        let data = ImeRequestData::default()
            .with_cursor_area(Position::Logical((0.0, 0.0).into()), Size::Logical((1.0, 1.0).into()))
            .with_hint_and_purpose(hint(editor), ImePurpose::Normal)
            .with_surrounding_text(text.clone());
        let capabilities = ImeCapabilities::new().with_cursor_area().with_hint_and_purpose().with_surrounding_text();
        let request = ImeEnableRequest::new(capabilities, data).expect("запрос IME: возможности и данные согласованы");
        self.ime_enabled = window.request_ime_update(ImeRequest::Enable(request)).is_ok();
        self.ime_text = self.ime_enabled.then_some(text);
    }

    #[cfg(target_os = "android")]
    pub(super) fn ime_start(&mut self, _window: &dyn Window) {
        if !self.ime_enabled {
            let editor = self.root.as_ref().and_then(focused_editor);
            crate::android::ime::start(hint(editor).contains(ImeHint::MULTILINE));
            self.ime_enabled = true;
        }
    }

    pub(super) fn ime_end(&mut self, window: &dyn Window) {
        #[cfg(target_os = "android")]
        crate::android::ime::stop();
        #[cfg(not(target_os = "android"))]
        let _ = window.request_ime_update(ImeRequest::Disable);
        #[cfg(target_os = "android")]
        let _ = window;
        self.ime_enabled = false;
        self.ime_area = None;
        self.ime_text = None;
    }

    pub(super) fn ime_moved(&mut self, window: &dyn Window, position: LogicalPosition<f64>, size: LogicalSize<f64>) {
        if !self.ime_enabled || self.ime_area == Some((position, size)) {
            return;
        }
        self.ime_area = Some((position, size));
        let k = self.app.scale();
        let position = LogicalPosition::new(position.x * k, position.y * k);
        let size = LogicalSize::new(size.width * k, size.height * k);
        let _ = window.request_ime_update(ImeRequest::Update(ImeRequestData::default().with_cursor_area(Position::Logical(position), Size::Logical(size))));
    }

    #[cfg(not(target_os = "android"))]
    pub(super) fn sync_ime_text(&mut self, window: &dyn Window) {
        if !self.ime_enabled {
            return;
        }
        let text = surrounding(self.root.as_ref().and_then(focused_editor));
        if self.ime_text.as_ref() == Some(&text) {
            return;
        }
        let _ = window.request_ime_update(ImeRequest::Update(ImeRequestData::default().with_surrounding_text(text.clone())));
        self.ime_text = Some(text);
    }

    #[cfg(target_os = "android")]
    pub(super) fn sync_ime_text(&mut self, _window: &dyn Window) {
        let Some(editor) = self.root.as_ref().and_then(focused_editor).filter(|_| self.ime_enabled) else { return };
        let (base, text, cursor, anchor) = editor.surrounding(crate::android::ime::base());
        crate::android::ime::report(base, crate::ime_sync::AppState { text, anchor, cursor, preedit: editor.composing() });
    }

    #[cfg(target_os = "android")]
    pub(super) fn ime_tapped(&self) {
        if self.ime_enabled && self.root.as_ref().and_then(focused_editor).is_some() {
            crate::android::ime::show();
        }
    }

    #[cfg(target_os = "android")]
    pub(super) fn ime_inbox(&mut self) {
        use crate::ime_sync::Edit;
        let Some(root) = &mut self.root else { return };
        for edit in crate::android::ime::take() {
            let ime = match edit {
                Edit::DeleteSurrounding { before, after } => masonry::core::Ime::DeleteSurrounding { before_bytes: before, after_bytes: after },
                Edit::Preedit(text, cursor) => masonry::core::Ime::Preedit(text, cursor),
                Edit::Commit(text) => masonry::core::Ime::Commit(text),
                Edit::Enter => {
                    for state in [KeyState::Down, KeyState::Up] {
                        let key = Key::Named(NamedKey::Enter);
                        let event = KeyboardEvent { state, key, code: Code::Enter, location: Location::Standard, modifiers: Modifiers::empty(), repeat: false, is_composing: false };
                        root.handle_text_event(TextEvent::Keyboard(event));
                    }
                    continue;
                }
                Edit::Select { anchor, cursor } => {
                    let base = crate::android::ime::base().unwrap_or(0);
                    if let Some(id) = root.focused_widget() {
                        root.edit_widget(id, |mut widget| {
                            let mut editor = widget.downcast::<TextEditor>();
                            TextEditor::ime_select(&mut editor, base + anchor, base + cursor);
                        });
                    }
                    continue;
                }
            };
            root.handle_text_event(TextEvent::Ime(ime));
        }
    }
}
