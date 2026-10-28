use std::ops::Range;

pub struct Pixels<'a> {
    pub data: &'a mut [u32],
    pub stride: usize,
    pub canvas: bool,
}

pub struct Target<'a> {
    pub pixels: Pixels<'a>,
    pub width: usize,
    pub height: usize,
    pub origin: (f64, f64),
    pub scale: f64,
    pub size: (f64, f64),
    pub radius: f64,
}

pub struct Row<'a> {
    pixels: &'a mut [u32],
    canvas: bool,
    inner: Range<usize>,
    x: f64,
    y: f64,
    size: (f64, f64),
    radius: f64,
}

impl Target<'_> {
    pub fn row(&mut self, j: usize) -> Row<'_> {
        let (w, h) = (self.size.0 * self.scale, self.size.1 * self.scale);
        let r = self.radius * self.scale;
        let column = |x: f64| ((x - self.origin.0).max(0.0) as usize).min(self.width);
        let y = self.origin.1 + j as f64 + 0.5;
        let reach = if y < r + 1.0 || y > h - r - 1.0 { r + 1.0 } else { 1.0 };
        let inner = if y < 1.0 || y > h - 1.0 { 0..0 } else { column(reach)..column(w - reach).max(column(reach)) };
        let (x, stride, width) = (self.origin.0, self.pixels.stride, self.width);
        Row { pixels: &mut self.pixels.data[j * stride..][..width], canvas: self.pixels.canvas, inner, x, y, size: (w, h), radius: r }
    }
}

impl Row<'_> {
    pub fn put(&mut self, i: usize, source: u32) {
        let source = if self.inner.contains(&i) { source } else { self.clip(i, source) };
        let pixel = &mut self.pixels[i];
        *pixel = over(order(self.canvas, source), *pixel);
    }

    pub fn put_slice(&mut self, start: usize, sources: &[u32]) {
        let end = start + sources.len();
        let inner = self.inner.start.clamp(start, end)..self.inner.end.clamp(start, end);
        let canvas = self.canvas;
        for (pixel, &source) in self.pixels[inner.clone()].iter_mut().zip(&sources[inner.start - start..]) {
            *pixel = over(order(canvas, source), *pixel);
        }
        for i in (start..inner.start).chain(inner.end..end) {
            self.put(i, sources[i - start]);
        }
    }

    fn clip(&self, i: usize, source: u32) -> u32 {
        let ((w, h), r, y) = (self.size, self.radius, self.y);
        let x = self.x + i as f64 + 0.5;
        let (cx, cy) = (x.clamp(r, w - r), y.clamp(r, h - r));
        let inside = r - ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        let coverage = (inside.min(x.min(w - x).min(y).min(h - y)) + 0.5).clamp(0.0, 1.0);
        if coverage < 1.0 { scale_pixel(source, coverage) } else { source }
    }
}

fn order(canvas: bool, source: u32) -> u32 {
    if canvas { crate::render::in_canvas_order(source) } else { source }
}

pub fn pack(rgb: [u8; 3], alpha: f64) -> u32 {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u32;
    let c = |v: u8| (u32::from(v) * a + 127) / 255;
    c(rgb[0]) | c(rgb[1]) << 8 | c(rgb[2]) << 16 | a << 24
}

fn scale_pixel(p: u32, k: f64) -> u32 {
    let m = |shift: u32| ((f64::from((p >> shift) & 0xff) * k).round() as u32) << shift;
    m(0) | m(8) | m(16) | m(24)
}

pub fn over(source: u32, dest: u32) -> u32 {
    let inverse = 255 - (source >> 24);
    let rb = (dest & 0x00ff_00ff) * inverse;
    let ag = ((dest >> 8) & 0x00ff_00ff) * inverse;
    let rb = ((rb + ((rb >> 8) & 0x00ff_00ff) + 0x0080_0080) >> 8) & 0x00ff_00ff;
    let ag = (ag + ((ag >> 8) & 0x00ff_00ff) + 0x0080_0080) & 0xff00_ff00;
    source + (rb | ag)
}
