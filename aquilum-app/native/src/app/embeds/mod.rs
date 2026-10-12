mod book;
mod cache;
mod dataview;
mod links;
mod media;
mod resolver;
#[cfg(test)]
mod testing;

pub use media::prune_remote;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use aquilum_core::Core;
use aquilum_core::search::error::SearchError;
use aquilum_core::search::paths::is_markdown;

use crate::ui::editor::resolve::{Book, Fetch, Grid, Media, Request, Resolver};
use cache::{Cache, Entry, claim, commit, settle};

const INDEX_WAIT: Duration = Duration::from_millis(100);
const INDEX_TRIES: usize = 600;

fn indexed<T>(mut run: impl FnMut() -> Result<T, SearchError>) -> Result<T, SearchError> {
    let mut result = run();
    for _ in 0..INDEX_TRIES {
        if !matches!(result, Err(SearchError::Unavailable { .. })) {
            break;
        }
        std::thread::sleep(INDEX_WAIT);
        result = run();
    }
    result
}


struct Inner {
    core: Arc<Core>,
    root: PathBuf,
    document: PathBuf,
    cache: PathBuf,
    wake: Arc<dyn Fn() + Send + Sync>,
    stamps: AtomicU64,
    changed: AtomicBool,
    grids: Cache<Grid>,
    media: Cache<Media>,
    books: Cache<Book>,
    suggestions: Cache<Vec<String>>,
    links: Mutex<links::Links>,
    book_queue: Mutex<BookQueue>,
    requests: Mutex<Vec<Request>>,
}

#[derive(Default)]
struct BookQueue {
    pending: Vec<(String, u64)>,
    running: bool,
}

impl Inner {
    fn touch(&self) {
        self.changed.store(true, Ordering::Release);
        (self.wake)();
    }

    fn workspace(&self) -> String {
        self.root.to_string_lossy().into_owned()
    }

    fn source(&self) -> String {
        self.document.to_string_lossy().into_owned()
    }
}

#[derive(Clone)]
pub struct Embeds(Arc<Inner>);

impl Embeds {
    pub fn new(core: Arc<Core>, root: PathBuf, document: PathBuf, cache: PathBuf, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        Embeds(Arc::new(Inner {
            core,
            root,
            document,
            cache,
            wake,
            stamps: AtomicU64::new(0),
            changed: AtomicBool::new(false),
            grids: Cache::default(),
            media: Cache::default(),
            books: Cache::default(),
            suggestions: Cache::default(),
            links: Mutex::default(),
            book_queue: Mutex::default(),
            requests: Mutex::new(Vec::new()),
        }))
    }

    pub fn root(&self) -> &Path {
        &self.0.root
    }

    pub fn document(&self) -> &Path {
        &self.0.document
    }

    pub fn take_requests(&self) -> Vec<Request> {
        std::mem::take(&mut *self.0.requests.lock().unwrap_or_else(|e| e.into_inner()))
    }

    pub fn forget_queries(&self) {
        forget(&self.0);
    }

    pub fn busy(&self) -> bool {
        fn running<T>(cache: &Cache<T>) -> bool {
            cache.entries.lock().unwrap_or_else(|e| e.into_inner()).values().any(|e| e.running)
        }
        running(&self.0.grids) || running(&self.0.media) || running(&self.0.books)
    }

    fn shared(&self, key: &str) -> String {
        format!("{}\0{key}", self.0.document.parent().unwrap_or(&self.0.root).display())
    }

    pub fn forget_paths(&self, paths: &[PathBuf]) {
        media::forget(paths);
        crate::video::forget(paths);
        let mut entries = self.0.media.entries.lock().unwrap_or_else(|e| e.into_inner());
        let before = entries.len();
        let gone = |fetch: &Fetch<Media>| matches!(fetch, Fetch::Ready(m) if m.path.as_ref().is_some_and(|p| paths.contains(p)));
        entries.retain(|_, entry| !gone(&entry.value) && !entry.next.as_ref().is_some_and(gone));
        let mut removed = entries.len() != before;
        drop(entries);
        if paths.iter().any(|p| is_markdown(p)) {
            self.0.books.epoch.fetch_add(1, Ordering::AcqRel);
            removed = true;
        }
        if removed {
            self.0.touch();
        }
    }

    pub fn take_changed(&self) -> bool {
        if !self.0.changed.swap(false, Ordering::AcqRel) {
            return false;
        }
        commit(&self.0.grids, &self.0.stamps);
        commit(&self.0.media, &self.0.stamps);
        commit(&self.0.books, &self.0.stamps);
        commit(&self.0.suggestions, &self.0.stamps);
        true
    }

    fn fetch<T: Clone + PartialEq + Send + 'static>(&self, cache: fn(&Inner) -> &Cache<T>, key: &str, work: impl FnOnce(&Inner) -> Fetch<T> + Send + 'static) -> (Fetch<T>, u64) {
        let (value, stamp, epoch) = claim(cache(&self.0), key);
        let Some(epoch) = epoch else { return (value, stamp) };
        let inner = Arc::clone(&self.0);
        let key = key.to_owned();
        std::thread::spawn(move || {
            let result = work(&inner);
            if settle(cache(&inner), &key, epoch, result) {
                inner.touch();
            }
        });
        (value, stamp)
    }
}


fn forget(inner: &Arc<Inner>) {
    inner.grids.epoch.fetch_add(1, Ordering::AcqRel);
    inner.suggestions.epoch.fetch_add(1, Ordering::AcqRel);
    inner.books.epoch.fetch_add(1, Ordering::AcqRel);
    links::refresh(inner);
    inner.touch();
}

fn refresh_after(inner: &Arc<Inner>, query: String, seconds: u32) {
    let weak = Arc::downgrade(inner);
    let epoch = inner.grids.epoch.load(Ordering::Acquire);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(u64::from(seconds.max(1))));
        let Some(inner) = weak.upgrade().filter(|inner| inner.grids.epoch.load(Ordering::Acquire) == epoch) else { return };
        if let Some(entry) = inner.grids.entries.lock().unwrap_or_else(|e| e.into_inner()).get_mut(&query) {
            entry.epoch = u64::MAX;
        }
        Embeds(inner).dataview(&query);
    });
}

