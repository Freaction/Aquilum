mod book;
mod dataview;
mod media;

pub use media::prune_remote;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use aquilum_core::Core;
use aquilum_core::search::error::SearchError;

use crate::ui::editor::resolve::{Book, Fetch, Grid, Media, Request, Resolver};

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

struct Entry<T> {
    value: Fetch<T>,
    epoch: u64,
    running: bool,
    stamp: u64,
    used: bool,
    next: Option<Fetch<T>>,
}

struct Cache<T> {
    entries: Mutex<HashMap<String, Entry<T>>>,
    epoch: AtomicU64,
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Cache { entries: Mutex::new(HashMap::new()), epoch: AtomicU64::new(0) }
    }
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
    requests: Mutex<Vec<Request>>,
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

    #[cfg(test)]
    pub fn cached_media(&self) -> usize {
        self.0.media.entries.lock().unwrap_or_else(|e| e.into_inner()).len() + media::recent_count(&self.shared(""))
    }

    #[cfg(test)]
    pub fn cached_book(&self, target: &str) -> Option<Book> {
        match &self.0.books.entries.lock().unwrap_or_else(|e| e.into_inner()).get(target)?.value {
            Fetch::Ready(book) => Some(book.clone()),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn cached_rows(&self, query: &str) -> Option<usize> {
        match &self.0.grids.entries.lock().unwrap_or_else(|e| e.into_inner()).get(query)?.value {
            Fetch::Ready(grid) => Some(grid.rows.len()),
            _ => None,
        }
    }

    pub fn forget_paths(&self, paths: &[PathBuf]) {
        media::forget(paths);
        crate::video::forget(paths);
        let mut entries = self.0.media.entries.lock().unwrap_or_else(|e| e.into_inner());
        let before = entries.len();
        entries.retain(|_, entry| !matches!(&entry.value, Fetch::Ready(m) if m.path.as_ref().is_some_and(|p| paths.contains(p))));
        let mut removed = entries.len() != before;
        drop(entries);
        if paths.iter().any(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"))) {
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
        let store = cache(&self.0);
        let epoch = store.epoch.load(Ordering::Acquire);
        let mut entries = store.entries.lock().unwrap_or_else(|e| e.into_inner());
        let value = match entries.get_mut(key) {
            Some(entry) if entry.epoch == epoch || entry.running => {
                entry.used = true;
                return (entry.value.clone(), entry.stamp);
            }
            Some(entry) => {
                entry.used = true;
                entry.running = true;
                (entry.value.clone(), entry.stamp)
            }
            None => {
                entries.insert(key.to_owned(), Entry { value: Fetch::Pending, epoch, running: true, stamp: 0, used: true, next: None });
                (Fetch::Pending, 0)
            }
        };
        drop(entries);
        let inner = Arc::clone(&self.0);
        let key = key.to_owned();
        std::thread::spawn(move || {
            let result = work(&inner);
            let store = cache(&inner);
            let mut entries = store.entries.lock().unwrap_or_else(|e| e.into_inner());
            let Some(entry) = entries.get_mut(&key) else { return };
            entry.epoch = epoch;
            if entry.value == result {
                entry.running = false;
                return;
            }
            entry.next = Some(result);
            drop(entries);
            inner.touch();
        });
        value
    }
}

fn commit<T>(cache: &Cache<T>, stamps: &AtomicU64) {
    for entry in cache.entries.lock().unwrap_or_else(|e| e.into_inner()).values_mut() {
        if let Some(next) = entry.next.take() {
            entry.value = next;
            entry.stamp = stamps.fetch_add(1, Ordering::AcqRel) + 1;
            entry.running = false;
        }
    }
}

fn forget(inner: &Inner) {
    inner.grids.epoch.fetch_add(1, Ordering::AcqRel);
    inner.suggestions.epoch.fetch_add(1, Ordering::AcqRel);
    inner.books.epoch.fetch_add(1, Ordering::AcqRel);
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

const SUGGESTIONS: usize = 12;

impl Resolver for Embeds {
    fn dataview(&self, query: &str) -> (Fetch<Grid>, u64) {
        let text = query.to_owned();
        let weak = Arc::downgrade(&self.0);
        self.fetch(|i| &i.grids, query, move |inner| {
            let (result, refresh) = dataview::run(inner, &text);
            if let (Some(seconds), Some(strong)) = (refresh, weak.upgrade()) {
                refresh_after(&strong, text, seconds);
            }
            result
        })
    }

    fn media(&self, source: &str, wiki: bool) -> (Fetch<Media>, u64) {
        let source_owned = source.to_owned();
        let key = format!("{}{source}", if wiki { "[[" } else { "" });
        let shared = self.shared(&key);
        {
            let mut entries = self.0.media.entries.lock().unwrap_or_else(|e| e.into_inner());
            if !entries.contains_key(&key)
                && let Some(media) = media::recent(&shared)
            {
                let epoch = self.0.media.epoch.load(Ordering::Acquire);
                let stamp = self.0.stamps.fetch_add(1, Ordering::AcqRel) + 1;
                entries.insert(key.clone(), Entry { value: Fetch::Ready(media), epoch, running: false, stamp, used: true, next: None });
            }
        }
        self.fetch(|i| &i.media, &key, move |inner| {
            let fetched = media::load(inner, &source_owned, wiki);
            if let Fetch::Ready(media) = &fetched {
                media::remember(shared, media.clone());
            }
            fetched
        })
    }

    fn book(&self, target: &str) -> (Fetch<Book>, u64) {
        let target_owned = target.to_owned();
        self.fetch(|i| &i.books, target, move |inner| book::load(inner, &target_owned))
    }

    fn suggest(&self, query: &str) -> Option<Vec<String>> {
        let text = query.to_owned();
        let (fetched, _) = self.fetch(|i| &i.suggestions, &query.to_lowercase(), move |inner| {
            match inner.core.search.suggest_notes(&inner.workspace(), &text, SUGGESTIONS) {
                Ok(notes) => Fetch::Ready(notes.into_iter().map(|n| n.title).collect()),
                Err(_) => Fetch::Ready(Vec::new()),
            }
        });
        match fetched {
            Fetch::Ready(titles) => Some(titles),
            _ => None,
        }
    }

    fn suggest_min(&self) -> Option<usize> {
        let editor = self.0.core.settings.get_config().editor;
        editor.link_suggest.then_some(editor.link_suggest_min_chars.max(1) as usize)
    }

    fn sweep(&self) {
        let mut entries = self.0.media.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.retain(|key, entry| {
            let keep = entry.used || entry.running;
            if !keep {
                media::forget_key(&self.shared(key));
            }
            entry.used = false;
            keep
        });
    }

    fn request(&self, request: Request) {
        self.0.requests.lock().unwrap_or_else(|e| e.into_inner()).push(request);
        (self.0.wake)();
    }
}
