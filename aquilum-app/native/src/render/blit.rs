use masonry::kurbo::{Affine, Rect};
use vello_cpu::Pixmap;

#[derive(Clone, Copy)]
pub(super) struct Blit {
    pub id: u64,
    pub transform: Affine,
    pub rect: Rect,
}

pub(super) fn fits(transform: Affine) -> bool {
    let [sx, ky, kx, sy, _, _] = transform.as_coeffs();
    ky == 0.0 && kx == 0.0 && (sx - 1.0).abs() < 1e-9 && (sy - 1.0).abs() < 1e-9
}

impl Blit {
    fn offset(&self) -> (isize, isize) {
        let [.., tx, ty] = self.transform.as_coeffs();
        ((0.5 - tx).floor() as isize, (0.5 - ty).floor() as isize)
    }

    pub fn inner(&self, pixmap: &Pixmap) -> Option<Rect> {
        let (dx, dy) = self.offset();
        let image = Rect::new(-dx as f64, -dy as f64, (usize::from(pixmap.width()) as isize - dx) as f64, (usize::from(pixmap.height()) as isize - dy) as f64);
        let r = self.rect.intersect(image);
        let inner = Rect::new((r.x0 / 4.0).ceil() * 4.0, (r.y0 / 4.0).ceil() * 4.0, (r.x1 / 4.0).floor() * 4.0, (r.y1 / 4.0).floor() * 4.0);
        (inner.width() > 0.0 && inner.height() > 0.0).then_some(inner)
    }

    pub fn copy(&self, pixmap: &Pixmap, pixels: &mut [u32], stride: usize, (x, y, w, h): (u16, u16, u16, u16)) {
        let (dx, dy) = self.offset();
        let width = usize::from(pixmap.width());
        let source = pixmap.data();
        let (x, w) = (usize::from(x), usize::from(w));
        for row in usize::from(y)..usize::from(y) + usize::from(h) {
            let from = &source[(row as isize + dy) as usize * width + (x as isize + dx) as usize..][..w];
            let line = &mut pixels[row * stride + x..][..w];
            for (pixel, p) in line.iter_mut().zip(from) {
                *pixel = super::in_canvas_order(u32::from_ne_bytes([p.r, p.g, p.b, p.a]));
            }
        }
    }
}
