use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Mutex, PoisonError};

static WAITING: Mutex<Option<Sender<String>>> = Mutex::new(None);

pub fn login_links() -> Option<Receiver<String>> {
    let (sender, receiver) = channel();
    *WAITING.lock().unwrap_or_else(PoisonError::into_inner) = Some(sender);
    Some(receiver)
}

pub fn deliver(link: String) {
    if let Some(sender) = WAITING.lock().unwrap_or_else(PoisonError::into_inner).as_ref() {
        let _ = sender.send(link);
    }
}
