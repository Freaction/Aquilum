use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use aquilum_core::files::error::FileCommandError;
use aquilum_core::search::paths::same_path;
use masonry::app::RenderRoot;

use super::super::App;
use crate::note::Note;

type Opened = Result<(Note, crop::Rope), FileCommandError>;

pub(in crate::app) struct Opening {
    path: PathBuf,
    rx: Receiver<Opened>,
    done: Option<Opened>,
}

impl App {
    pub(in crate::app) fn on_opened(&mut self, root: &mut RenderRoot) -> bool {
        let Some(opening) = self.opening.as_mut().filter(|opening| opening.done.is_none()) else { return false };
        let opened = match opening.rx.try_recv() {
            Err(TryRecvError::Empty) => return false,
            received => received.unwrap_or_else(|_| Err(FileCommandError::Task { message: "чтение прервано".to_owned() })),
        };
        opening.done = Some(opened);
        self.show_active(root);
        true
    }

    pub(super) fn opened(&mut self, path: Option<&Path>) -> Option<Opened> {
        self.opening.take_if(|opening| !path.is_some_and(|path| same_path(path, &opening.path)));
        let path = path?;
        if let Some(opened) = self.opening.as_mut().and_then(|opening| opening.done.take()) {
            self.opening = None;
            return Some(opened);
        }
        if self.opening.is_none() {
            self.read_note(path.to_path_buf());
        }
        None
    }

    fn read_note(&mut self, path: PathBuf) {
        let (tx, rx) = channel();
        let (wake, read) = (Arc::clone(&self.workspace.wake), path.clone());
        crate::disk::run(move || {
            let opened = {
                let _zone = aq_trace::zone("открытие: чтение файла");
                Note::open(read)
            };
            if tx.send(opened).is_ok() {
                wake();
            }
        });
        self.opening = Some(Opening { path, rx, done: None });
        self.hold = Some(std::time::Instant::now() + super::super::HOLD);
    }
}
