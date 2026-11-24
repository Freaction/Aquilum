use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use aquilum_core::threads::{self, Work};

type Wake = Arc<dyn Fn() + Send + Sync>;
type Job = Box<dyn FnOnce() + Send>;

pub(crate) fn detach<T: Send + 'static>(name: &'static str, job: impl FnOnce() -> T + Send + 'static) -> Option<JoinHandle<T>> {
    threads::background(name, job).map_err(|error| report!("поток «{name}» не запущен: {error}")).ok()
}

pub(crate) fn delivery<T: Send + 'static>(sender: Sender<T>, wake: &Wake, job: impl FnOnce() -> Option<T> + Send + 'static) -> impl FnOnce() + Send + 'static {
    let wake = Arc::clone(wake);
    move || {
        if let Some(value) = job()
            && sender.send(value).is_ok()
        {
            wake();
        }
    }
}

pub(crate) fn deliver<T: Send + 'static>(name: &'static str, sender: Sender<T>, wake: &Wake, job: impl FnOnce() -> Option<T> + Send + 'static) -> Option<JoinHandle<()>> {
    let delivery = delivery(sender, wake, job);
    detach(name, move || {
        let _zone = aq_trace::zone(name);
        delivery();
    })
}

pub(crate) struct Queue {
    name: &'static str,
    work: Work,
    state: Mutex<(Vec<Job>, usize)>,
}

impl Queue {
    pub(crate) const fn new(name: &'static str, work: Work) -> Self {
        Queue { name, work, state: Mutex::new((Vec::new(), 0)) }
    }

    pub(crate) fn push(&'static self, job: impl FnOnce() + Send + 'static) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.0.push(Box::new(job));
        if state.1 < threads::count(self.work) && detach(self.name, move || self.drain()).is_some() {
            state.1 += 1;
        }
    }

    fn drain(&self) {
        while let Some(job) = self.next() {
            let _zone = aq_trace::zone(self.name);
            drop(std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)));
        }
    }

    fn next(&self) -> Option<Job> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let job = state.0.pop();
        if job.is_none() {
            state.1 -= 1;
        }
        job
    }
}
