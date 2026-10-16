mod mode;
mod notes;
mod panel;
pub mod reader;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use aquilum_core::Core;

pub use mode::{Method, Mode};
pub use notes::{find_note, linked_file_path};
pub use panel::run;

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Note(PathBuf, Option<usize>),
    Wiki(String),
    Url(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub title: String,
    pub counter: Option<String>,
    pub target: Target,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Listing {
    pub generation: u64,
    pub items: Vec<Item>,
    pub notice: Option<String>,
}

pub struct Request {
    pub generation: u64,
    pub workspace: PathBuf,
    pub document: PathBuf,
    pub mode: Mode,
    pub method: Method,
    pub text: String,
}

pub fn spawn(core: Arc<Core>, request: Request, results: Sender<Listing>, wake: Arc<dyn Fn() + Send + Sync>) {
    std::thread::spawn(move || {
        let listing = {
            let _zone = aq_trace::zone("фон: панель ссылок");
            run(&core, &request)
        };
        if results.send(listing).is_ok() {
            wake();
        }
    });
}
