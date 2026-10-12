use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::ui::editor::resolve::Fetch;

pub(super) struct Entry<T> {
    pub(super) value: Fetch<T>,
    pub(super) epoch: u64,
    pub(super) running: bool,
    pub(super) stamp: u64,
    pub(super) used: bool,
    pub(super) next: Option<Fetch<T>>,
}

pub(super) struct Cache<T> {
    pub(super) entries: Mutex<HashMap<String, Entry<T>>>,
    pub(super) epoch: AtomicU64,
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Cache { entries: Mutex::new(HashMap::new()), epoch: AtomicU64::new(0) }
    }
}

pub(super) fn claim<T: Clone>(store: &Cache<T>, key: &str) -> (Fetch<T>, u64, Option<u64>) {
    let epoch = store.epoch.load(Ordering::Acquire);
    let mut entries = store.entries.lock().unwrap_or_else(|e| e.into_inner());
    match entries.get_mut(key) {
        Some(entry) if entry.epoch == epoch || entry.running => {
            entry.used = true;
            (entry.value.clone(), entry.stamp, None)
        }
        Some(entry) => {
            entry.used = true;
            entry.running = true;
            (entry.value.clone(), entry.stamp, Some(epoch))
        }
        None => {
            entries.insert(key.to_owned(), Entry { value: Fetch::Pending, epoch, running: true, stamp: 0, used: true, next: None });
            (Fetch::Pending, 0, Some(epoch))
        }
    }
}

pub(super) fn settle<T: PartialEq>(store: &Cache<T>, key: &str, epoch: u64, result: Fetch<T>) -> bool {
    let mut entries = store.entries.lock().unwrap_or_else(|e| e.into_inner());
    let Some(entry) = entries.get_mut(key) else { return false };
    entry.epoch = epoch;
    if entry.value == result {
        entry.running = false;
        return false;
    }
    entry.next = Some(result);
    true
}

pub(super) fn commit<T>(cache: &Cache<T>, stamps: &AtomicU64) {
    for entry in cache.entries.lock().unwrap_or_else(|e| e.into_inner()).values_mut() {
        if let Some(next) = entry.next.take() {
            entry.value = next;
            entry.stamp = stamps.fetch_add(1, Ordering::AcqRel) + 1;
            entry.running = false;
        }
    }
}
