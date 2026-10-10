mod css;
mod motion;
mod raster;

pub use motion::{animated, paint as paint_motion};

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

use masonry::peniko::{Blob, Color, ImageAlphaType, ImageBrush, ImageData, ImageFormat};

pub const PATTERN_IDS: &[&str] = &[
    "origami",
    "scales",
    "chevron",
    "arrows",
    "moire",
    "ruby",
    "rings",
    "notebook",
    "dotgrid",
    "parquet",
    "tartan",
    "plaid",
    "pineapple",
    "stripes",
    "zigzag",
    "hearts",
    "cubes",
    "fan",
    "blueprint",
    "neon",
    "lattice",
    "quilt",
    "foliage",
    "ripples",
    "trellis",
    "dreams",
    "polka",
    "weave",
    "quadrants",
    "woven-tiles",
    "city",
    "tunnel",
];

const CSS: &str = concat!(
    include_str!("../../assets/covers/patterns/base.css"),
    include_str!("../../assets/covers/patterns/stripes.css"),
    include_str!("../../assets/covers/patterns/hearts.css"),
    include_str!("../../assets/covers/patterns/cubes.css"),
    include_str!("../../assets/covers/patterns/origami.css"),
    include_str!("../../assets/covers/patterns/foliage.css"),
    include_str!("../../assets/covers/patterns/arrows.css"),
    include_str!("../../assets/covers/patterns/moire.css"),
    include_str!("../../assets/covers/patterns/parquet.css"),
    include_str!("../../assets/covers/patterns/ripples.css"),
    include_str!("../../assets/covers/patterns/trellis.css"),
    include_str!("../../assets/covers/patterns/grids.css"),
    include_str!("../../assets/covers/patterns/radial.css"),
    include_str!("../../assets/covers/patterns/dreams.css"),
    include_str!("../../assets/covers/patterns/polka.css"),
    include_str!("../../assets/covers/patterns/weave.css"),
    include_str!("../../assets/covers/patterns/quadrants.css"),
    include_str!("../../assets/covers/patterns/woven-tiles.css"),
    include_str!("../../assets/covers/patterns/city.css"),
    include_str!("../../assets/covers/patterns/tunnel.css"),
);

fn patterns() -> &'static HashMap<String, css::Pattern> {
    static PATTERNS: OnceLock<HashMap<String, css::Pattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| css::parse_patterns(CSS))
}

pub fn render_pattern(id: &str, width: u32, height: u32, scale: f64) -> Option<Vec<u8>> {
    let pattern = patterns().get(id)?;
    Some(raster::render(&[&pattern.base, &pattern.overlay], width, height, scale))
}

pub const DEFAULT_PATTERN: &str = "origami";
const PREFIX: &str = "pattern:";
const LEGACY_PREFIX: &str = "builtin:";
const CACHE_LIMIT: usize = 6;

pub fn pattern_id(value: Option<&str>) -> Option<&'static str> {
    let value = value.map(str::trim).unwrap_or_default();
    if value.is_empty() {
        return Some(DEFAULT_PATTERN);
    }
    if let Some(id) = value.strip_prefix(PREFIX) {
        return Some(PATTERN_IDS.iter().copied().find(|p| *p == id).unwrap_or(DEFAULT_PATTERN));
    }
    if let Some(legacy) = value.strip_prefix(LEGACY_PREFIX) {
        let hash = legacy.encode_utf16().fold(0u64, |h, c| (h * 31 + u64::from(c)) % 1_000_003);
        return Some(PATTERN_IDS[(hash % PATTERN_IDS.len() as u64) as usize]);
    }
    None
}

pub fn random_pattern(current: Option<&str>) -> String {
    let current = pattern_id(current);
    let pool: Vec<&str> = PATTERN_IDS.iter().copied().filter(|id| Some(*id) != current).collect();
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() as usize).unwrap_or_default();
    format!("{PREFIX}{}", pool[seed % pool.len()])
}

pub fn base_color(id: &str) -> Color {
    let c = patterns().get(id).map(|p| p.base.color).unwrap_or([0.0; 4]);
    Color::new([c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32])
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Key {
    pub id: &'static str,
    pub width: u32,
    pub height: u32,
    pub scale: u32,
}

#[derive(Default)]
struct Jobs {
    done: Vec<(Key, ImageBrush)>,
    pending: HashSet<Key>,
    finished: bool,
}

static JOBS: Mutex<Option<Jobs>> = Mutex::new(None);
static WAKER: OnceLock<Arc<dyn Fn() + Send + Sync>> = OnceLock::new();

pub fn set_waker(wake: Arc<dyn Fn() + Send + Sync>) {
    let _ = WAKER.set(wake);
}

pub fn image(key: Key) -> Option<ImageBrush> {
    let mut guard = JOBS.lock().ok()?;
    let jobs = guard.get_or_insert_with(Jobs::default);
    if let Some((_, image)) = jobs.done.iter().find(|(k, _)| *k == key) {
        return Some(image.clone());
    }
    if jobs.pending.insert(key) {
        std::thread::spawn(move || {
            let pixels = render_pattern(key.id, key.width, key.height, f64::from(key.scale) / 1000.0);
            let mut guard = JOBS.lock().unwrap_or_else(|e| e.into_inner());
            let jobs = guard.get_or_insert_with(Jobs::default);
            jobs.pending.remove(&key);
            if let Some(pixels) = pixels {
                let data = ImageData {
                    data: Blob::new(Arc::new(pixels)),
                    format: ImageFormat::Rgba8,
                    alpha_type: ImageAlphaType::AlphaPremultiplied,
                    width: key.width,
                    height: key.height,
                };
                if jobs.done.len() >= CACHE_LIMIT {
                    jobs.done.remove(0);
                }
                jobs.done.push((key, ImageBrush::new(data)));
                jobs.finished = true;
            }
            drop(guard);
            if let Some(wake) = WAKER.get() {
                wake();
            }
        });
    }
    None
}

pub fn take_finished() -> bool {
    JOBS.lock().ok().and_then(|mut g| g.as_mut().map(|j| std::mem::take(&mut j.finished))).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn snapshot_motion() {
        use masonry::kurbo::{Affine, Rect};
        let dir = std::path::PathBuf::from(std::env::var_os("AQ_OUT").unwrap());
        let (w, h) = (900u32, 300u32);
        for id in ["city", "dreams", "tunnel"] {
            let pixels = super::render_pattern(id, w, h, 1.0).unwrap();
            let data = masonry::peniko::ImageData {
                data: masonry::peniko::Blob::new(std::sync::Arc::new(pixels)),
                format: masonry::peniko::ImageFormat::Rgba8,
                alpha_type: masonry::peniko::ImageAlphaType::AlphaPremultiplied,
                width: w,
                height: h,
            };
            let pattern = masonry::peniko::ImageBrush::new(data);
            for t in [0.0, 0.5, 20.0] {
                let bounds = Rect::new(0.0, 0.0, f64::from(w), f64::from(h));
                let image = crate::render::offscreen(w as u16, h as u16, |painter| {
                    painter.fill(bounds, super::base_color(id)).draw();
                    painter.draw_image(&pattern, Affine::IDENTITY);
                    super::paint_motion(painter, id, bounds, t, 900.0);
                })
                .unwrap();
                let rgba = image::RgbaImage::from_raw(w, h, image.image.data.data().to_vec()).unwrap();
                rgba.save(dir.join(format!("motion-{id}-{t}.png"))).unwrap();
            }
        }
    }

    #[test]
    #[ignore]
    fn snapshot_patterns() {
        let dir = std::env::var_os("AQUILUM_SNAPSHOTS").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let (w, h) = (640u32, 200u32);
        for id in super::PATTERN_IDS {
            let started = std::time::Instant::now();
            let pixels = super::render_pattern(id, w, h, 1.0).unwrap();
            eprintln!("{id}: {:?}", started.elapsed());
            let mut pixmap = vello_cpu::Pixmap::new(w as u16, h as u16);
            pixmap.data_as_u8_slice_mut().copy_from_slice(&pixels);
            std::fs::write(dir.join(format!("cover-{id}.png")), pixmap.into_png().unwrap()).unwrap();
        }
    }
}
