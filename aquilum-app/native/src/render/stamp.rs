use crate::output::{Area, Canvas, Target};

#[derive(Default)]
pub struct Stamps {
    saved: Vec<(Area, Vec<u32>)>,
    spare: Vec<Vec<u32>>,
}

impl Stamps {
    pub fn trim(&mut self) {
        self.spare = Vec::new();
    }

    pub fn untouched(&self, touched: &[Area]) -> bool {
        self.saved.iter().all(|(area, _)| touched.iter().all(|t| area.intersect(*t).is_none()))
    }

    pub fn forget(&mut self) {
        self.spare.extend(self.saved.drain(..).map(|(_, pixels)| pixels));
    }

    pub fn restore(&mut self, canvas: &mut dyn Canvas) -> Option<Area> {
        if self.saved.is_empty() {
            return None;
        }
        let saved = std::mem::take(&mut self.saved);
        canvas.with_target(&mut |mut target| {
            for (area, pixels) in saved.iter().rev() {
                rows(&mut target, *area).zip(pixels.chunks_exact(area.w as usize)).for_each(|(row, kept)| row.copy_from_slice(kept));
            }
        });
        let bounds = saved.iter().map(|(area, _)| *area).reduce(Area::union);
        self.spare.extend(saved.into_iter().map(|(_, pixels)| pixels));
        bounds
    }

    pub fn save(&mut self, target: &mut Target<'_>, area: Area) -> Option<Area> {
        let area = area.intersect(Area { x: 0, y: 0, w: u32::from(target.width), h: u32::from(target.height) })?;
        let mut pixels = self.spare.pop().unwrap_or_default();
        pixels.clear();
        pixels.reserve(area.w as usize * area.h as usize);
        rows(target, area).for_each(|row| pixels.extend_from_slice(row));
        self.saved.push((area, pixels));
        Some(area)
    }

    pub fn apply(&mut self, target: &mut Target<'_>, area: Area, mut paint: impl FnMut(u32, u32, &mut u32)) -> Option<Area> {
        let area = self.save(target, area)?;
        for (y, row) in (area.y..).zip(rows(target, area)) {
            for (x, pixel) in (area.x..).zip(row.iter_mut()) {
                paint(x, y, pixel);
            }
        }
        Some(area)
    }
}

fn rows<'a>(target: &'a mut Target<'_>, area: Area) -> impl Iterator<Item = &'a mut [u32]> {
    let stride = usize::from(target.width);
    let (x, w) = (area.x as usize, area.w as usize);
    target.pixels.chunks_exact_mut(stride).skip(area.y as usize).take(area.h as usize).map(move |row| &mut row[x..x + w])
}

pub fn covering(rect: masonry::kurbo::Rect) -> Area {
    let r = rect.expand();
    let (x0, y0) = (r.x0.max(0.0), r.y0.max(0.0));
    Area { x: x0 as u32, y: y0 as u32, w: (r.x1 - x0).max(0.0) as u32, h: (r.y1 - y0).max(0.0) as u32 }
}

pub fn blend(under: u32, color: [f32; 4], coverage: f32) -> u32 {
    let under = super::in_canvas_order(under);
    let a = color[3] * coverage;
    let channel = |shift: u32, value: f32| {
        let dest = ((under >> shift) & 0xff) as f32;
        ((dest * (1.0 - a) + value * 255.0 * coverage).round().clamp(0.0, 255.0) as u32) << shift
    };
    super::in_canvas_order(0xff00_0000 | channel(0, color[0]) | channel(8, color[1]) | channel(16, color[2]))
}
