use std::sync::{Arc, Mutex};

use aquilum_core::recent::Recent;

use super::{Target, pack};

const TILE_PNG: &[u8] = include_bytes!("../../../assets/city-cover.png");
const WIDTH: f64 = 640.0;
const HEIGHT: f64 = 442.0;
const STRIP: f64 = WIDTH * 9.0;
const SECONDS: f64 = 60.0;

type Tile = (u32, usize, usize, Arc<Vec<u32>>);

static TILE: Mutex<Recent<u32, Tile>> = Mutex::new(Recent::new(1, usize::MAX));

pub fn forget() {
    TILE.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

fn scaled_tile(scale: f64) -> Option<Tile> {
    let key = (scale * 1000.0).round() as u32;
    let mut cached = TILE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(tile) = cached.get(&key) {
        return Some(tile.clone());
    }
    let (w, h) = ((WIDTH * scale).round() as u32, (HEIGHT * scale).round() as u32);
    let image = image::load_from_memory(TILE_PNG).ok()?.resize_exact(w.max(1), h.max(1), image::imageops::FilterType::Triangle).into_rgba8();
    let pixels = image.pixels().map(|p| pack([p[0], p[1], p[2]], f64::from(p[3]) / 255.0)).collect();
    let tile = (key, w as usize, h as usize, Arc::new(pixels));
    cached.put(key, tile.clone(), 0);
    Some(tile)
}

pub(super) fn draw(t: &mut Target<'_>, seconds: f64) {
    let Some((_, tw, th, tile)) = scaled_tile(t.scale) else { return };
    let (w, h) = t.size;
    let shift = -WIDTH + WIDTH * (seconds % SECONDS) / SECONDS;
    let start = w / 2.0 - STRIP / 2.0 + shift;
    let top = ((h - HEIGHT) * t.scale - t.origin.1).round() as isize;
    let first = ((-start) / WIDTH).floor().max(0.0);
    let mut x = start + first * WIDTH;
    while x < w && x < start + STRIP {
        let left = (x * t.scale - t.origin.0).round() as isize;
        x += WIDTH;
        for j in 0..th {
            let y = top + j as isize;
            if y < 0 || y >= t.height as isize {
                continue;
            }
            let (from, to) = (left.max(0), (left + tw as isize).min(t.width as isize));
            if to <= from {
                continue;
            }
            let row = &tile[j * tw..(j + 1) * tw];
            let dest = &mut t.pixels[y as usize * t.width..];
            dest[from as usize..to as usize].copy_from_slice(&row[(from - left) as usize..(to - left) as usize]);
        }
    }
}
