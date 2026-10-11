use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

use masonry::peniko::{ImageAlphaType, ImageBrush};

use super::render_pattern;

const CACHE_BYTES: usize = 4 << 20;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Key {
    pub id: &'static str,
    pub width: u32,
    pub height: u32,
    pub scale: u32,
}

pub struct Pattern {
    pub image: ImageBrush,
    pub exact: bool,
}

#[derive(Default)]
struct Jobs {
    done: Vec<(Key, ImageBrush)>,
    wanted: HashMap<&'static str, Key>,
    running: HashSet<&'static str>,
    finished: bool,
}

static JOBS: Mutex<Option<Jobs>> = Mutex::new(None);
static WAKER: OnceLock<Arc<dyn Fn() + Send + Sync>> = OnceLock::new();

pub fn set_waker(wake: Arc<dyn Fn() + Send + Sync>) {
    let _ = WAKER.set(wake);
}

pub fn image(key: Key) -> Option<Pattern> {
    let mut guard = JOBS.lock().ok()?;
    let jobs = guard.get_or_insert_with(Jobs::default);
    if let Some(at) = jobs.done.iter().position(|(k, _)| *k == key) {
        let entry = jobs.done.remove(at);
        let image = entry.1.clone();
        jobs.done.push(entry);
        return Some(Pattern { image, exact: true });
    }
    jobs.wanted.insert(key.id, key);
    if jobs.running.insert(key.id) {
        std::thread::spawn(move || work(key.id));
    }
    let nearest = jobs
        .done
        .iter()
        .filter(|(k, _)| k.id == key.id && k.scale == key.scale)
        .min_by_key(|(k, _)| (k.height.abs_diff(key.height), k.width.abs_diff(key.width)))
        .map(|(_, image)| image.clone());
    nearest.map(|image| Pattern { image, exact: false })
}

fn work(id: &'static str) {
    loop {
        let key = {
            let mut guard = JOBS.lock().unwrap_or_else(|e| e.into_inner());
            let jobs = guard.get_or_insert_with(Jobs::default);
            match jobs.wanted.remove(id) {
                Some(key) if !jobs.done.iter().any(|(k, _)| *k == key) => key,
                Some(_) => continue,
                None => {
                    jobs.running.remove(id);
                    return;
                }
            }
        };
        let pixels = render_pattern(key.id, key.width, key.height, f64::from(key.scale) / 1000.0);
        let image = pixels.and_then(|pixels| crate::render::from_rgba(pixels, key.width, key.height, ImageAlphaType::AlphaPremultiplied));
        let mut guard = JOBS.lock().unwrap_or_else(|e| e.into_inner());
        let jobs = guard.get_or_insert_with(Jobs::default);
        if let Some(image) = image {
            jobs.done.push((key, image));
            while jobs.done.len() > 1 && jobs.done.iter().map(|(_, i)| i.image.data.len()).sum::<usize>() > CACHE_BYTES {
                jobs.done.remove(0);
            }
            jobs.finished = true;
        }
        drop(guard);
        if let Some(wake) = WAKER.get() {
            wake();
        }
    }
}

pub fn trim() {
    if let Ok(mut guard) = JOBS.lock()
        && let Some(jobs) = guard.as_mut()
    {
        jobs.done.clear();
    }
}

pub fn take_finished() -> bool {
    JOBS.lock().ok().and_then(|mut g| g.as_mut().map(|j| std::mem::take(&mut j.finished))).unwrap_or(false)
}
