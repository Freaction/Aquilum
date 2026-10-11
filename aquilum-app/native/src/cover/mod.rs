mod css;
mod jobs;
mod motion;
mod raster;

pub use jobs::{Key, image, set_waker, take_finished, trim};

pub use motion::{Target as MotionTarget, animated, fps as motion_fps, over, render as render_motion};

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
    let (mut first, rest): (Vec<&str>, Vec<&str>) = PATTERN_IDS.iter().filter(|id| id.contains(query)).partition(|id| id.starts_with(query));
    first.extend(rest);
    first.into_iter().map(str::to_owned).collect()
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
    #[ignore]
    fn snapshot_motion() {
        let dir = std::path::PathBuf::from(std::env::var_os("AQ_OUT").unwrap());
        let (w, h) = (900usize, 300usize);
        for id in ["city", "dreams", "tunnel"] {
            let base = super::render_pattern(id, w as u32, h as u32, 1.0).unwrap();
            for t in [0.0, 0.5, 20.0] {
                let mut motion = vec![0u32; w * h];
                let mut target = super::MotionTarget { pixels: &mut motion, width: w, height: h, origin: (0.0, 0.0), scale: 1.0, size: (w as f64, h as f64), radius: 12.0 };
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
                let mut target = super::MotionTarget { pixels: &mut pixels, width: w, height: h, origin: (0.0, 0.0), scale, size: (w as f64 / scale, h as f64 / scale), radius: 12.0 };
                super::render_motion(id, &mut target, f64::from(frame) / 45.0, 900.0);
            }
            eprintln!("{id}: {:.3} мс на кадр", started.elapsed().as_secs_f64() * 1000.0 / 200.0);
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
