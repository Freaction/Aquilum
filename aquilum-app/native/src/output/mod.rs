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
const KEPT_FRAMES: usize = 2;

#[cfg(not(target_os = "android"))]
#[derive(Default)]
struct History(Vec<Vec<Area>>);

#[cfg(not(target_os = "android"))]
impl History {
    fn reset(&mut self) {
        self.0.clear();
    }

    fn frame(&mut self, dirty: &[Area], age: usize) -> (Vec<Area>, Option<Vec<Area>>) {
        let mut areas = dirty.to_vec();
        merge(&mut areas);
        let missing = (1..=self.0.len() + 1).contains(&age).then(|| areas.iter().chain(self.0[..age - 1].iter().flatten()).copied().collect());
        self.0.insert(0, areas.clone());
        self.0.truncate(KEPT_FRAMES);
        (areas, missing)
    }
}

#[cfg(not(target_os = "windows"))]
struct Pixels {
    data: Vec<u32>,
    width: u32,
    height: u32,
    background: u32,
    fresh: bool,
}

#[cfg(not(target_os = "windows"))]
impl Pixels {
    fn new() -> Self {
        Pixels { data: Vec::new(), width: 0, height: 0, background: 0, fresh: true }
    }

    fn sized(&self, width: u32, height: u32) -> bool {
        self.width == width && self.height == height && !self.data.is_empty()
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.data.resize((width * height) as usize, self.background);
        self.data.fill(self.background);
        (self.width, self.height, self.fresh) = (width, height, true);
    }

    fn trim(&mut self) {
        self.data = Vec::new();
        (self.width, self.height, self.fresh) = (0, 0, true);
    }

    fn set_background(&mut self, color: u32) {
        if self.background != color {
            self.background = color;
            self.data.fill(color);
            self.fresh = true;
        }
    }

    fn copy_to(&self, dst: &mut [u32], stride: usize, area: Area) {
        let (x, w) = (area.x as usize, area.w as usize);
        for y in area.y as usize..(area.y + area.h) as usize {
            let (from, to) = (y * self.width as usize + x, y * stride + x);
            if let (Some(dst), Some(src)) = (dst.get_mut(to..to + w), self.data.get(from..from + w)) {
                dst.copy_from_slice(src);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
impl Canvas for Pixels {
    fn with_target(&mut self, paint: &mut dyn FnMut(Target<'_>)) {
        paint(Target { pixels: &mut self.data, width: self.width as u16, height: self.height as u16 });
    }
}

pub fn closest<T: Copy>(items: &[T], cost: impl Fn(T, T) -> f64) -> Option<(usize, usize, f64)> {
    let mut best: Option<(usize, usize, f64)> = None;
    for a in 0..items.len() {
        for b in a + 1..items.len() {
            let extra = cost(items[a], items[b]);
            if best.is_none_or(|(_, _, c)| extra < c) {
                best = Some((a, b, extra));
            }
        }
    }
    best
}

#[cfg(not(target_os = "android"))]
pub fn merge(areas: &mut Vec<Area>) {
    areas.retain(|a| a.w > 0 && a.h > 0);
    while let Some((a, b, cost)) = closest(areas, |x, y| (x.union(y).size() - x.size() - y.size()) as f64) {
        if areas.len() <= MAX_RECTS && cost > 0.0 {
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

#[cfg(all(test, not(target_os = "android")))]
#[test]
fn a_buffer_gets_every_change_since_it_was_shown() {
    let at = |y| Area { x: 0, y, w: 10, h: 10 };
    let mut history = History::default();
    assert_eq!(history.frame(&[at(0)], 3).1, None, "буфер старше истории копируется целиком");
    assert_eq!(history.frame(&[at(100)], 3).1, None);
    assert_eq!(history.frame(&[at(200)], 3).1, Some(vec![at(200), at(100), at(0)]));
    assert_eq!(history.frame(&[at(300)], 1).1, Some(vec![at(300)]));
    assert_eq!(history.frame(&[at(400)], 0).1, None, "возраст 0 — содержимое неизвестно");
    history.reset();
    assert_eq!(history.frame(&[at(500)], 2), (vec![at(500)], None));
}
