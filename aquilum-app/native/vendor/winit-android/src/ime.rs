use std::sync::Mutex;

use android_activity::AndroidApp;
use android_activity::input::{Keycode, TextInputState, TextSpan};
use ndk::configuration::{Keyboard, KeysHidden};
use smol_str::SmolStr;
use winit_core::event::{ElementState, Ime, KeyEvent, WindowEvent};
use winit_core::window::{ImeCapabilities, ImeRequest, ImeRequestData, ImeRequestError, ImeSurroundingText};

use crate::bridge::{Bridge, Edit, ImeState};
use crate::keycodes;

mod editor_info;

struct Session {
    bridge: Bridge,
    dispatching: bool,
    pending: Option<ImeSurroundingText>,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

pub(crate) fn request(
    app: &AndroidApp,
    enabled: &Mutex<Option<ImeCapabilities>>,
    request: ImeRequest,
) -> Result<(), ImeRequestError> {
    let mut enabled = enabled.lock().unwrap();
    match request {
        ImeRequest::Enable(enable) => {
            if enabled.is_some() {
                return Err(ImeRequestError::AlreadyEnabled);
            }
            let (capabilities, data) = enable.into_raw();
            *enabled = Some(capabilities);
            if hardware_keyboard(app) {
                return Ok(());
            }
            let bridge = Bridge::new(app.text_input_state().text);
            *SESSION.lock().unwrap() = Some(Session { bridge, dispatching: false, pending: None });
            update(app, capabilities, data);
            app.show_soft_input(true);
        },
        ImeRequest::Update(data) => {
            let Some(capabilities) = *enabled else { return Err(ImeRequestError::NotEnabled) };
            update(app, capabilities, data);
        },
        ImeRequest::Disable => {
            *enabled = None;
            *SESSION.lock().unwrap() = None;
            app.hide_soft_input(true);
        },
        _ => return Err(ImeRequestError::NotSupported),
    }
    Ok(())
}

pub(crate) fn text_changed(
    app: &AndroidApp,
    state: &TextInputState,
    dispatch: impl FnMut(WindowEvent),
) {
    let selection = (state.selection.start, state.selection.end);
    let compose = state.compose_region.map(|span| (span.start, span.end));
    let ime = ImeState::from_utf16(state.text.clone(), selection, compose);
    deliver(app, |bridge| bridge.ime_changed(&ime), dispatch);
}

pub(crate) fn editor_action(app: &AndroidApp, dispatch: impl FnMut(WindowEvent)) {
    deliver(app, |_| vec![Edit::Enter], dispatch);
}

fn hardware_keyboard(app: &AndroidApp) -> bool {
    let config = app.config();
    config.keyboard() == Keyboard::Qwerty && config.keys_hidden() == KeysHidden::No
}

fn update(app: &AndroidApp, capabilities: ImeCapabilities, data: ImeRequestData) {
    let mut session = SESSION.lock().unwrap();
    let Some(session) = session.as_mut() else { return };
    if let Some((hint, purpose)) = data.hint_and_purpose.filter(|_| capabilities.hint_and_purpose())
    {
        let (input_type, action, options) = editor_info::editor_info(hint, purpose);
        app.set_ime_editor_info(input_type, action, options);
        session.bridge.restart();
    }
    if let Some(text) = data.surrounding_text.filter(|_| capabilities.surrounding_text()) {
        if session.dispatching {
            session.pending = Some(text);
            return;
        }
        app_changed(&mut session.bridge, text);
    }
    push(app, &mut session.bridge);
}

fn deliver(
    app: &AndroidApp,
    edits: impl FnOnce(&mut Bridge) -> Vec<Edit>,
    mut dispatch: impl FnMut(WindowEvent),
) {
    let edits = {
        let mut session = SESSION.lock().unwrap();
        let Some(session) = session.as_mut() else { return };
        session.dispatching = true;
        edits(&mut session.bridge)
    };
    for edit in edits {
        emit(edit, &mut dispatch);
    }
    let mut session = SESSION.lock().unwrap();
    let Some(session) = session.as_mut() else { return };
    session.dispatching = false;
    if let Some(text) = session.pending.take() {
        app_changed(&mut session.bridge, text);
    }
    push(app, &mut session.bridge);
}

fn app_changed(bridge: &mut Bridge, text: ImeSurroundingText) {
    let (cursor, anchor) = (text.cursor(), text.anchor());
    bridge.app_changed(text.into_text(), cursor, anchor);
}

fn push(app: &AndroidApp, bridge: &mut Bridge) {
    let Some(state) = bridge.take_push() else { return };
    let ((anchor, cursor), compose) = state.utf16();
    app.set_text_input_state(TextInputState {
        text: state.text,
        selection: TextSpan { start: anchor, end: cursor },
        compose_region: compose.map(|(start, end)| TextSpan { start, end }),
    });
}

fn emit(edit: Edit, dispatch: &mut impl FnMut(WindowEvent)) {
    match edit {
        Edit::DeleteSurrounding { before, after } => dispatch(WindowEvent::Ime(
            Ime::DeleteSurrounding { before_bytes: before, after_bytes: after },
        )),
        Edit::Preedit(text, cursor) => dispatch(WindowEvent::Ime(Ime::Preedit(text, cursor))),
        Edit::Commit(text) => dispatch(WindowEvent::Ime(Ime::Commit(text))),
        Edit::Enter => press(Keycode::Enter, 1, dispatch),
        Edit::Left(count) => press(Keycode::DpadLeft, count, dispatch),
        Edit::Right(count) => press(Keycode::DpadRight, count, dispatch),
    }
}

fn press(keycode: Keycode, count: usize, dispatch: &mut impl FnMut(WindowEvent)) {
    let logical_key = keycodes::to_logical(None, keycode);
    for _ in 0..count {
        for state in [ElementState::Pressed, ElementState::Released] {
            let text = logical_key.to_text().filter(|_| state == ElementState::Pressed).map(SmolStr::new);
            dispatch(WindowEvent::KeyboardInput {
                device_id: None,
                event: KeyEvent {
                    physical_key: keycodes::to_physical_key(keycode),
                    logical_key: logical_key.clone(),
                    text: text.clone(),
                    location: keycodes::to_location(keycode),
                    state,
                    repeat: false,
                    text_with_all_modifiers: text,
                    key_without_modifiers: logical_key.clone(),
                },
                is_synthetic: false,
            });
        }
    }
}
