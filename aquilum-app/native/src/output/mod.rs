#[cfg(target_os = "windows")]
mod device;
#[cfg(target_os = "windows")]
mod dxgi;
#[cfg(target_os = "android")]
mod android;
#[cfg(not(any(target_os = "windows", target_os = "android")))]
mod soft;

#[cfg(target_os = "windows")]
pub use dxgi::Output;
#[cfg(target_os = "android")]
pub use android::Output;
#[cfg(not(any(target_os = "windows", target_os = "android")))]
pub use soft::Output;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[cfg(not(target_os = "android"))]
const MAX_RECTS: usize = 8;

#[cfg(not(target_os = "android"))]
pub fn merge(areas: &mut Vec<Area>) {
    areas.retain(|a| a.w > 0 && a.h > 0);
    while areas.len() > 1 {
        let mut best = (0, 1, i64::MAX);
        for a in 0..areas.len() {
            for b in a + 1..areas.len() {
                let cost = areas[a].union(areas[b]).size() - areas[a].size() - areas[b].size();
                if cost < best.2 {
                    best = (a, b, cost);
                }
            }
        }
        let (a, b, cost) = best;
        if areas.len() <= MAX_RECTS && cost > 0 {
            return;
        }
        areas[a] = areas[a].union(areas[b]);
        areas.swap_remove(b);
    }
}

impl Area {
    #[cfg(not(target_os = "android"))]
    fn size(self) -> i64 {
        i64::from(self.w) * i64::from(self.h)
    }

    pub fn union(self, other: Area) -> Area {
        let (x, y) = (self.x.min(other.x), self.y.min(other.y));
        Area { x, y, w: (self.x + self.w).max(other.x + other.w) - x, h: (self.y + self.h).max(other.y + other.h) - y }
    }

    pub fn intersect(self, other: Area) -> Option<Area> {
        let (x0, y0) = (self.x.max(other.x), self.y.max(other.y));
        let (x1, y1) = ((self.x + self.w).min(other.x + other.w), (self.y + self.h).min(other.y + other.h));
        (x1 > x0 && y1 > y0).then(|| Area { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
    }
}

pub trait Canvas {
    fn with_target(&mut self, paint: &mut dyn FnMut(Target<'_>));
}

pub struct Target<'a> {
    pub pixels: &'a mut [u32],
    pub width: u16,
    pub height: u16,
}
