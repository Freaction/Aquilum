mod css;
mod jobs;
mod motion;
mod raster;

pub use jobs::{Key, image, set_waker, take_finished, trim};
pub use raster::Cell;
#[cfg(test)]
pub use raster::expand;

pub use motion::{Pixels as MotionPixels, Target as MotionTarget, animated, forget_tile, fps as motion_fps, over, render as render_motion};

use std::collections::HashMap;
use std::sync::OnceLock;

use masonry::peniko::Color;

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
    include_str!("../../assets/covers/patterns/base.pattern"),
    include_str!("../../assets/covers/patterns/stripes.pattern"),
    include_str!("../../assets/covers/patterns/hearts.pattern"),
    include_str!("../../assets/covers/patterns/cubes.pattern"),
    include_str!("../../assets/covers/patterns/origami.pattern"),
    include_str!("../../assets/covers/patterns/foliage.pattern"),
    include_str!("../../assets/covers/patterns/arrows.pattern"),
    include_str!("../../assets/covers/patterns/moire.pattern"),
    include_str!("../../assets/covers/patterns/parquet.pattern"),
    include_str!("../../assets/covers/patterns/ripples.pattern"),
    include_str!("../../assets/covers/patterns/trellis.pattern"),
    include_str!("../../assets/covers/patterns/grids.pattern"),
    include_str!("../../assets/covers/patterns/radial.pattern"),
    include_str!("../../assets/covers/patterns/dreams.pattern"),
    include_str!("../../assets/covers/patterns/polka.pattern"),
    include_str!("../../assets/covers/patterns/weave.pattern"),
    include_str!("../../assets/covers/patterns/quadrants.pattern"),
    include_str!("../../assets/covers/patterns/woven-tiles.pattern"),
    include_str!("../../assets/covers/patterns/city.pattern"),
    include_str!("../../assets/covers/patterns/tunnel.pattern"),
);

fn patterns() -> &'static HashMap<String, css::Pattern> {
    static PATTERNS: OnceLock<HashMap<String, css::Pattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| css::parse_patterns(CSS))
}

pub fn render_pattern(id: &str, width: u32, height: u32, scale: f64) -> Option<Cell> {
    let pattern = patterns().get(id)?;
    Some(raster::render(&[&pattern.base, &pattern.overlay], width, height, scale))
}

pub const DEFAULT_PATTERN: &str = "origami";
const PREFIX: &str = "pattern:";
const LEGACY_PREFIX: &str = "builtin:";

pub fn pattern_id(value: Option<&str>) -> Option<&'static str> {
    let value = value.map(str::trim).unwrap_or_default();
    if value.is_empty() {
        return Some(DEFAULT_PATTERN);
    }
    if let Some(id) = value.strip_prefix(PREFIX) {
        return Some(named_pattern(id).unwrap_or(DEFAULT_PATTERN));
    }
    if let Some(id) = named_pattern(value) {
        return Some(id);
    }
    if let Some(legacy) = value.strip_prefix(LEGACY_PREFIX) {
        let hash = legacy.encode_utf16().fold(0u64, |h, c| (h * 31 + u64::from(c)) % 1_000_003);
        return Some(PATTERN_IDS[(hash % PATTERN_IDS.len() as u64) as usize]);
    }
    None
}

fn named_pattern(name: &str) -> Option<&'static str> {
    PATTERN_IDS.iter().copied().find(|p| p.eq_ignore_ascii_case(name.trim()))
}

pub fn suggest_patterns(query: &str) -> Vec<String> {
    let query = query.trim().to_lowercase();
    let query = query.strip_prefix(PREFIX).unwrap_or(&query);
    crate::rank::rank(PATTERN_IDS, query).into_iter().map(str::to_owned).collect()
}

pub fn random_pattern(current: Option<&str>) -> String {
    let current = pattern_id(current);
    let pool: Vec<&str> = PATTERN_IDS.iter().copied().filter(|id| Some(*id) != current).collect();
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() as usize).unwrap_or_default();
    format!("{PREFIX}{}", pool[seed % pool.len()])
}

pub fn centered(id: &str) -> bool {
    matches!(id, "city" | "dreams")
}

pub fn base_color(id: &str) -> Color {
    let c = patterns().get(id).map(|p| p.base.color).unwrap_or([0.0; 4]);
    Color::new([c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32])
}

#[cfg(test)]
mod tests {
    #[test]
    fn tiled_bake_matches_the_whole_bake_for_every_pattern() {
        let mut tiled = Vec::new();
        for (width, height, scale) in [(700u32, 300u32, 1.75), (480, 200, 1.0)] {
            for id in super::PATTERN_IDS {
                let pattern = super::patterns().get(*id).unwrap();
                let backgrounds = [&pattern.base, &pattern.overlay];
                let whole = super::raster::rasterize(&backgrounds, width, height, scale, (width, height));
                let fast = super::raster::expand(&super::render_pattern(id, width, height, scale).unwrap(), width, height);
                assert_eq!(whole.len(), fast.len());
                let differ = whole.iter().zip(&fast).filter(|(a, b)| a.abs_diff(**b) > 70).count();
                assert!(differ * 1000 < whole.len(), "{id} {width}x{height}@{scale}: {differ} из {} значений каналов отличаются", whole.len());
                if let Some(cell) = super::raster::tile(&backgrounds, width, height, scale) {
                    tiled.push(format!("{id}@{scale}: {}x{}", cell.0, cell.1));
                }
            }
        }
        assert!(tiled.len() >= 30, "почти все узоры должны запекаться плиткой: {tiled:?}");
    }

    #[test]
    #[ignore]
    fn snapshot_motion() {
        let dir = std::path::PathBuf::from(std::env::var_os("AQ_OUT").unwrap());
        let (w, h) = (900usize, 300usize);
        for id in ["city", "dreams", "tunnel"] {
            let base = super::raster::expand(&super::render_pattern(id, w as u32, h as u32, 1.0).unwrap(), w as u32, h as u32);
            for t in [0.0, 0.5, 20.0] {
                let mut motion = vec![0u32; w * h];
                let pixels = super::MotionPixels { data: &mut motion, stride: w, canvas: false };
                let mut target = super::MotionTarget { pixels, width: w, height: h, origin: (0.0, 0.0), scale: 1.0, size: (w as f64, h as f64), radius: 12.0 };
                super::render_motion(id, &mut target, t, 900.0);
                let pixels: Vec<u8> = base
                    .chunks_exact(4)
                    .zip(&motion)
                    .flat_map(|(b, &m)| super::over(m, u32::from_le_bytes([b[0], b[1], b[2], b[3]])).to_le_bytes())
                    .collect();
                image::RgbaImage::from_raw(w as u32, h as u32, pixels).unwrap().save(dir.join(format!("motion-{id}-{t}.png"))).unwrap();
            }
        }
    }

    #[test]
    #[ignore]
    fn motion_cost() {
        let (w, h, scale) = (1793usize, 359usize, 1.75);
        let mut pixels = vec![0u32; w * h];
        for id in ["tunnel", "city", "dreams"] {
            let started = std::time::Instant::now();
            for frame in 0..200 {
                pixels.fill(0);
                let data = super::MotionPixels { data: &mut pixels, stride: w, canvas: false };
                let mut target = super::MotionTarget { pixels: data, width: w, height: h, origin: (0.0, 0.0), scale, size: (w as f64 / scale, h as f64 / scale), radius: 12.0 };
                super::render_motion(id, &mut target, f64::from(frame) / 45.0, 900.0);
            }
            eprintln!("{id}: {:.3} мс на кадр", started.elapsed().as_secs_f64() * 1000.0 / 200.0);
        }
    }

    #[test]
    #[ignore]
    fn pattern_cost() {
        let started = std::time::Instant::now();
        let _ = super::patterns();
        eprintln!("разбор всего CSS: {:?}", started.elapsed());
        let (w, h, scale) = (1793u32, 359u32, 1.75);
        let mut all = Vec::new();
        for id in super::PATTERN_IDS {
            let started = std::time::Instant::now();
            let cell = super::render_pattern(id, w, h, scale).unwrap();
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            all.push((ms, *id, cell.pixels.len()));
        }
        all.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (ms, id, bytes) in &all {
            eprintln!("{id:<12} {ms:>7.1} мс  {:.1} МБ", crate::stats::mb(*bytes));
        }
        let total: f64 = all.iter().map(|a| a.0).sum();
        eprintln!("итого {total:.0} мс на {} узоров, в среднем {:.0} мс", all.len(), total / all.len() as f64);
    }

    #[test]
    #[ignore]
    fn untiled_patterns() {
        for id in super::PATTERN_IDS {
            let pattern = super::patterns().get(*id).unwrap();
            let backgrounds = [&pattern.base, &pattern.overlay];
            if super::raster::tile(&backgrounds, 1793, 359, 1.75).is_none() {
                let sizes: Vec<String> = backgrounds.iter().flat_map(|b| &b.layers).map(|l| match l.size { None => "авто".to_owned(), Some((w, h)) => format!("{}/{}%x{}/{}%", w.px, w.pct, h.px, h.pct) }).collect();
                eprintln!("{id}: {}", sizes.join(", "));
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
            let pixels = super::raster::expand(&super::render_pattern(id, w, h, 1.0).unwrap(), w, h);
            eprintln!("{id}: {:?}", started.elapsed());
            let mut pixmap = vello_cpu::Pixmap::new(w as u16, h as u16);
            pixmap.data_as_u8_slice_mut().copy_from_slice(&pixels);
            std::fs::write(dir.join(format!("cover-{id}.png")), pixmap.into_png().unwrap()).unwrap();
        }
    }
}
