use masonry::imaging::ClipRef;

use super::Painter;
use super::convert;

impl Painter {
    pub(super) fn offscreen(&self, bounds: masonry::kurbo::Rect) -> bool {
        self.culled > 0 || !self.filtered() && !self.frame.overlaps(bounds)
    }

    pub(super) fn cull(&mut self, clip: &ClipRef<'_>) -> bool {
        if self.culled > 0 {
            return self.cull_inside();
        }
        let bounds = match clip {
            ClipRef::Fill { transform, shape, .. } => transform.transform_rect_bbox(convert::shape_bounds(shape)),
            ClipRef::Stroke { transform, shape, stroke } => {
                let w = stroke.width.max(1.0) * stroke.miter_limit.max(1.0);
                transform.transform_rect_bbox(convert::shape_bounds(shape).inflate(w, w))
            }
        };
        if !self.filtered() && !self.frame.overlaps(bounds) {
            self.culled = 1;
            return true;
        }
        false
    }

    pub(super) fn filtered(&self) -> bool {
        self.groups.contains(&true)
    }

    pub(super) fn cull_inside(&mut self) -> bool {
        self.culled += 1;
        true
    }

    pub(super) fn uncull(&mut self) -> bool {
        if self.culled == 0 {
            return false;
        }
        self.culled -= 1;
        true
    }
}
