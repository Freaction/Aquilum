use super::*;

pub(super) fn visible(transform: Affine, window: Size) -> (f64, f64) {
    let r = transform.inverse().transform_rect_bbox(window.to_rect());
    (r.y0, r.y1)
}

pub(super) fn rect(b: BoundingBox, top: f64) -> Rect {
    Rect::new(b.x0, b.y0 + top, b.x1, b.y1 + top)
}
