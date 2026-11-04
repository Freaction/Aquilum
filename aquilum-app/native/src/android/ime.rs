use std::sync::{Arc, Mutex, PoisonError};

use super::java;
use crate::ime_sync::{AppState, Edit, Message, Session};
use crate::ui::keymap::Command;

const SELECT_ALL: i32 = 16_908_319;
const CUT: i32 = 16_908_320;
const COPY: i32 = 16_908_321;
const PASTE: i32 = 16_908_322;
const PASTE_PLAIN: i32 = 16_908_337;

#[derive(Default)]
struct Bridge {
    session: Session,
    commands: Vec<Command>,
    wake: Option<Arc<dyn Fn() + Send + Sync>>,
}

static BRIDGE: Mutex<Option<Bridge>> = Mutex::new(None);

fn with<T>(work: impl FnOnce(&mut Bridge) -> T) -> T {
    let mut bridge = BRIDGE.lock().unwrap_or_else(PoisonError::into_inner);
    work(bridge.get_or_insert_with(Bridge::default))
}

pub fn set_wake(wake: Arc<dyn Fn() + Send + Sync>) {
    with(|bridge| bridge.wake = Some(wake));
}

fn notify(work: impl FnOnce(&mut Bridge)) {
    let wake = with(|bridge| {
        work(bridge);
        bridge.wake.clone()
    });
    if let Some(wake) = wake {
        wake();
    }
}

pub fn received(message: Message) {
    notify(|bridge| bridge.session.received(message));
}

pub fn action(id: i32) -> bool {
    let command = match id {
        SELECT_ALL => Command::SelectAll,
        CUT => Command::Cut,
        COPY => Command::Copy,
        PASTE => Command::Paste,
        PASTE_PLAIN => Command::PastePlain,
        _ => return false,
    };
    notify(|bridge| bridge.commands.push(command));
    true
}

pub fn commands() -> Vec<Command> {
    with(|bridge| std::mem::take(&mut bridge.commands))
}

pub fn start(multiline: bool) {
    let session = with(|bridge| bridge.session.start());
    java::send_start(jni::jni_str!("imeStart"), multiline, session);
}

pub fn stop() {
    with(|bridge| bridge.session.stop());
    java::run(jni::jni_str!("imeStop"));
}

pub fn show() {
    java::run(jni::jni_str!("imeShow"));
}

pub fn base() -> Option<usize> {
    with(|bridge| bridge.session.base())
}

pub fn take() -> Vec<Edit> {
    with(|bridge| bridge.session.take())
}

pub fn report(base: usize, app: AppState) {
    let Some((applied, ime)) = with(|bridge| bridge.session.report(base, &app)) else { return };
    let ((anchor, cursor), compose) = ime.utf16();
    let (start, end) = compose.map_or((-1, -1), |(start, end)| (start as i32, end as i32));
    java::send_ime(jni::jni_str!("imeApply"), [applied, base as i32], &ime.text, [anchor as i32, cursor as i32, start, end]);
}
