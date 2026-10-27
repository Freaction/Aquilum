use std::sync::{Arc, Mutex, PoisonError};

use super::java;
use crate::ime_sync::{AppState, Edit, ImeState, Sync};

#[derive(Default)]
struct Session {
    sync: Sync,
    base: Option<usize>,
    applied: i32,
    inbox: Vec<(i32, ImeState)>,
    wake: Option<Arc<dyn Fn() + Send + std::marker::Sync>>,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

fn with<T>(work: impl FnOnce(&mut Session) -> T) -> T {
    let mut session = SESSION.lock().unwrap_or_else(PoisonError::into_inner);
    work(session.get_or_insert_with(Session::default))
}

pub fn set_wake(wake: Arc<dyn Fn() + Send + std::marker::Sync>) {
    with(|session| session.wake = Some(wake));
}

pub fn received(seq: i32, text: String, selection: (usize, usize), compose: Option<(usize, usize)>) {
    let wake = with(|session| {
        session.inbox.push((seq, ImeState::from_utf16(text, selection, compose)));
        session.wake.clone()
    });
    if let Some(wake) = wake {
        wake();
    }
}

pub fn start(multiline: bool) {
    with(|session| (session.sync, session.base) = (Sync::default(), None));
    java::send_bool(jni::jni_str!("imeStart"), multiline);
}

pub fn stop() {
    java::run(jni::jni_str!("imeStop"));
}

pub fn show() {
    java::run(jni::jni_str!("imeShow"));
}

pub fn base() -> Option<usize> {
    with(|session| session.base)
}

pub fn take() -> Vec<Edit> {
    with(|session| {
        let inbox = std::mem::take(&mut session.inbox);
        inbox.into_iter().flat_map(|(seq, ime)| {
            session.applied = seq;
            session.sync.ime_changed(&ime)
        }).collect()
    })
}

pub fn report(base: usize, app: AppState) {
    let state = with(|session| {
        let moved = session.base.replace(base) != Some(base);
        (session.sync.app_changed(app.clone()) || moved).then_some(session.applied)
    });
    let Some(applied) = state else { return };
    let ime = app.to_ime();
    let ((anchor, cursor), compose) = ime.utf16();
    let (start, end) = compose.map_or((-1, -1), |(start, end)| (start as i32, end as i32));
    java::send_ime(jni::jni_str!("imeApply"), [applied, base as i32], &ime.text, [anchor as i32, cursor as i32, start, end]);
}
