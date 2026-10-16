use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use masonry::peniko::{Blob, ImageAlphaType, ImageBrush, WeakBlob};

mod decode;
mod kind;

pub use kind::{MediaKind, extension, kind};

pub const MAX_WIDTH: u32 = 1600;
const MIN_WIDTH: u32 = 320;
const MAX_PIXELS: f64 = 1600.0 * 1600.0;
const RECENT_BYTES: usize = 8 << 20;

static SHOWN: AtomicU32 = AtomicU32::new(MAX_WIDTH);

const DEFAULT_BOOK_COVER: &[u8] = include_bytes!("../../assets/covers/default-book-cover.webp");

#[derive(Clone)]
pub struct Picture {
    pub image: ImageBrush,
    pub natural: (u32, u32),
}

struct Cached {
    modified: Option<SystemTime>,
    hollow: ImageBrush,
    natural: (u32, u32),
    data: WeakBlob<u8>,
}

#[derive(Default)]
struct Cache {
    entries: HashMap<PathBuf, Cached>,
    recent: VecDeque<ImageBrush>,
}

impl Cache {
    fn keep(&mut self, image: &ImageBrush) {
        let id = image.image.data.id();
        self.recent.retain(|kept| kept.image.data.id() != id);
        self.recent.push_front(image.clone());
        let mut total = 0;
        let mut count = 0;
        self.recent.retain(|kept| {
            total += kept.image.data.len();
            count += 1;
            count == 1 || total <= RECENT_BYTES
        });
    }
}

fn cache() -> std::sync::MutexGuard<'static, Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default).lock().unwrap_or_else(|e| e.into_inner())
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

pub const EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];

pub fn show(width: f64) {
    SHOWN.store((width.ceil() as u32).clamp(MIN_WIDTH, MAX_WIDTH), Ordering::Relaxed);
}

fn shown() -> u32 {
    SHOWN.load(Ordering::Relaxed)
}

fn decode_within(bytes: &[u8], limit: u32) -> Option<Picture> {
    let decoded = decode::decode(bytes, limit, MAX_PIXELS)?;
    let image = crate::render::from_rgba(decoded.pixels, decoded.width, decoded.height, ImageAlphaType::Alpha)?;
    Some(Picture { image, natural: decoded.natural })
}

pub fn decode(bytes: &[u8]) -> Option<ImageBrush> {
    decode_within(bytes, MAX_WIDTH).map(|p| p.image)
}

pub fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().ok()?.into_dimensions().ok()
}

pub fn decoded_size(bytes: &[u8]) -> Option<(u32, u32)> {
    dimensions(bytes).map(|natural| decode::fit(natural, MAX_WIDTH, MAX_PIXELS))
}

pub fn decode_shown(bytes: &[u8]) -> Option<Picture> {
    decode_within(bytes, shown())
}

pub fn cached(path: &Path) -> Option<Picture> {
    let modified = modified(path);
    let mut cache = cache();
    let entry = cache.entries.get(path).filter(|e| e.modified == modified)?;
    let sharp = decode::fit(entry.natural, shown(), MAX_PIXELS).0 <= entry.hollow.image.width;
    let data = entry.data.upgrade().filter(|_| sharp);
    let Some(data) = data else {
        cache.entries.remove(path);
        return None;
    };
    let mut image = entry.hollow.clone();
    image.image.data = data;
    let natural = entry.natural;
    cache.keep(&image);
    Some(Picture { image, natural })
}

pub fn remember(path: &Path, picture: &Picture) {
    let modified = modified(path);
    let mut hollow = picture.image.clone();
    hollow.image.data = Blob::new(Arc::new(Vec::<u8>::new()));
    let mut cache = cache();
    cache.entries.retain(|_, e| e.data.upgrade().is_some());
    cache.entries.insert(path.to_path_buf(), Cached { modified, hollow, natural: picture.natural, data: picture.image.image.data.downgrade() });
    cache.keep(&picture.image);
}

pub fn trim() {
    let mut cache = cache();
    cache.recent.clear();
    cache.entries.retain(|_, e| e.data.upgrade().is_some());
}

pub fn picture(path: &Path) -> Option<Picture> {
    if let Some(picture) = cached(path) {
        return Some(picture);
    }
    let picture = decode_shown(&std::fs::read(path).ok()?)?;
    remember(path, &picture);
    Some(picture)
}

pub fn load(path: &Path) -> Option<ImageBrush> {
    picture(path).map(|p| p.image)
}

pub fn default_book_cover() -> ImageBrush {
    static COVER: OnceLock<ImageBrush> = OnceLock::new();
    COVER.get_or_init(|| decode(DEFAULT_BOOK_COVER).expect("обложка книги по умолчанию")).clone()
}
