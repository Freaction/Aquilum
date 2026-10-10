use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use masonry::peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat};

const MAX_SIDE: u32 = 2400;
const CACHE_LIMIT: usize = 16;

pub const DEFAULT_BOOK_COVER: &[u8] = include_bytes!("../assets/covers/default-book-cover.webp");

type Cache = Mutex<HashMap<PathBuf, (Option<SystemTime>, ImageBrush)>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(Cache::default)
}

pub fn decode(bytes: &[u8]) -> Option<ImageBrush> {
    let mut image = image::load_from_memory(bytes).ok()?;
    if image.width() > MAX_SIDE || image.height() > MAX_SIDE {
        image = image.resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Triangle);
    }
    let rgba = image.into_rgba8();
    let (width, height) = rgba.dimensions();
    let data = ImageData {
        data: Blob::new(Arc::new(rgba.into_raw())),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width,
        height,
    };
    Some(ImageBrush::new(data))
}

pub fn load(path: &Path) -> Option<ImageBrush> {
    let modified = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
    if let Some((stamp, image)) = cache().lock().ok()?.get(path)
        && *stamp == modified
    {
        return Some(image.clone());
    }
    let image = decode(&std::fs::read(path).ok()?)?;
    let mut cache = cache().lock().ok()?;
    if cache.len() >= CACHE_LIMIT {
        cache.clear();
    }
    cache.insert(path.to_path_buf(), (modified, image.clone()));
    Some(image)
}

pub fn default_book_cover() -> ImageBrush {
    static COVER: OnceLock<ImageBrush> = OnceLock::new();
    COVER.get_or_init(|| decode(DEFAULT_BOOK_COVER).expect("обложка книги по умолчанию")).clone()
}
