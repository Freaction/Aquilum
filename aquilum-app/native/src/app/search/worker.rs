use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::Duration;

use aquilum_core::Core;
use aquilum_core::search::error::SearchError;
use aquilum_core::search::models::SearchResponse;
use aquilum_core::search::query::Request;
use aquilum_core::text::cancel::{Cancel, Generation};

const TYPING_DELAY: Duration = Duration::from_millis(70);

pub(in crate::app) struct Found {
    pub(super) generation: u64,
    pub(super) response: Result<SearchResponse, String>,
}

struct Ask {
    workspace: PathBuf,
    query: String,
    cancel: Cancel,
}

pub(super) struct Worker {
    asks: Sender<Ask>,
    generation: Generation,
}

impl Worker {
    pub(super) fn spawn(core: Arc<Core>, found: Sender<Found>, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let (asks, inbox) = channel();
        let started = std::thread::Builder::new().name("поиск Ctrl+O".into()).spawn(move || serve(&core, &inbox, &found, &*wake));
        if let Err(error) = started {
            report!("поток поиска не запущен: {error}");
        }
        Self { asks, generation: Generation::default() }
    }

    pub(super) fn ask(&self, workspace: PathBuf, query: String) {
        let cancel = self.generation.next();
        let _ = self.asks.send(Ask { workspace, query, cancel });
    }

    pub(super) fn stop(&self) {
        self.generation.stop();
    }

    pub(super) fn current(&self) -> u64 {
        self.generation.current()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.generation.stop();
    }
}

fn latest(inbox: &Receiver<Ask>, mut ask: Ask) -> Option<Ask> {
    loop {
        match inbox.recv_timeout(TYPING_DELAY) {
            Ok(newer) => ask = newer,
            Err(RecvTimeoutError::Timeout) => return Some(ask),
            Err(RecvTimeoutError::Disconnected) => return None,
        }
    }
}

fn serve(core: &Core, inbox: &Receiver<Ask>, found: &Sender<Found>, wake: &(dyn Fn() + Send + Sync)) {
    while let Ok(first) = inbox.recv() {
        let Some(ask) = latest(inbox, first) else { return };
        let request = Request { cancel: ask.cancel.clone(), ..Request::new(&ask.query) };
        let response = match core.search.search(&ask.workspace.to_string_lossy(), &request) {
            Err(SearchError::Cancelled) => continue,
            response => response.map_err(|e| format!("{e:?}")),
        };
        if ask.cancel.is_cancelled() {
            continue;
        }
        if found.send(Found { generation: ask.cancel.generation(), response }).is_err() {
            return;
        }
        wake();
    }
}
