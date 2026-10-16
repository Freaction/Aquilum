use std::sync::OnceLock;
use std::sync::mpsc::{Sender, channel};

type Job = Box<dyn FnOnce() + Send>;

static QUEUE: OnceLock<Sender<Job>> = OnceLock::new();

pub fn run(job: impl FnOnce() + Send + 'static) {
    let queue = QUEUE.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        std::thread::spawn(move || rx.into_iter().for_each(|job| job()));
        tx
    });
    let _ = queue.send(Box::new(job));
}

pub fn wait() {
    let (tx, rx) = channel();
    run(move || {
        let _ = tx.send(());
    });
    let _ = rx.recv();
}
