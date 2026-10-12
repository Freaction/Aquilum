use super::*;

pub fn reorder_layout(boxes: &[(f64, f64)], from: usize, offset: f64) -> (usize, Vec<f64>) {
    let mut shift = vec![0.0; boxes.len()];
    let Some(&(left, width)) = boxes.get(from) else { return (from, shift) };
    let left = left + offset;
    let right = left + width;
    let mut target = from;
    for (index, &(box_left, box_width)) in boxes.iter().enumerate() {
        let middle = box_left + box_width / 2.0;
        if index < from && left < middle {
            shift[index] = width;
            target = target.min(index);
        } else if index > from && right > middle {
            shift[index] = -width;
            target = target.max(index);
        }
    }
    shift[from] = offset;
    (target, shift)
}

pub(super) struct Reorder {
    from: usize,
    start: f64,
    dragging: bool,
    target: usize,
}

impl Reorder {
    pub(super) fn new(from: usize, start: f64) -> Self {
        Reorder { from, start, dragging: false, target: from }
    }
}

pub(super) fn slide() -> Tween {
    Tween::new(0.0, FAST, EASE_OUT)
}

impl TabStrip {
    fn slide_to(&mut self, ctx: &mut EventCtx<'_>, targets: &[f64], dragged: Option<usize>) {
        self.shift.resize_with(self.tabs.len(), slide);
        let mut moving = false;
        for (index, tween) in self.shift.iter_mut().enumerate() {
            let target = targets.get(index).copied().unwrap_or(0.0);
            if tween.set(target) {
                if Some(index) == dragged {
                    tween.finish();
                } else {
                    moving = true;
                }
            }
        }
        if moving {
            ctx.request_anim_frame();
        }
        ctx.request_layout();
    }

    pub(super) fn clamp(&mut self) {
        self.offset = self.offset.clamp(0.0, (self.content - self.width).max(0.0));
    }

    pub(super) fn scroll(&mut self, ctx: &mut EventCtx<'_>, delta: &ScrollDelta) {
        let d = match *delta {
            ScrollDelta::LineDelta(x, y) => -f64::from(if x != 0.0 { x } else { y }) * crate::ui::scroll::LINE,
            ScrollDelta::PixelDelta(p) => -(if p.x != 0.0 { p.x } else { p.y }) / ctx.scale_factor(),
            ScrollDelta::PageDelta(x, y) => -f64::from(if x != 0.0 { x } else { y }) * self.width,
        };
        let before = self.offset;
        self.offset += d;
        self.clamp();
        if self.offset != before {
            ctx.request_layout();
            ctx.set_handled();
        }
    }

    pub(super) fn drag_move(&mut self, ctx: &mut EventCtx<'_>, x: f64) {
        let Some(reorder) = &self.reorder else { return };
        if !reorder.dragging && (x + self.offset - reorder.start).abs() < DRAG_THRESHOLD {
            return;
        }
        if x < EDGE {
            self.offset -= EDGE_STEP;
        } else if x > self.width - EDGE {
            self.offset += EDGE_STEP;
        }
        self.clamp();
        let Some(reorder) = &mut self.reorder else { return };
        reorder.dragging = true;
        let from = reorder.from;
        let (target, shift) = reorder_layout(&self.boxes, from, x + self.offset - reorder.start);
        reorder.target = target;
        self.slide_to(ctx, &shift, Some(from));
    }

    pub(super) fn drop_reorder(&mut self, ctx: &mut EventCtx<'_>, released: bool) {
        let Some(reorder) = self.reorder.take() else { return };
        if !reorder.dragging {
            return;
        }
        if reorder.target != reorder.from && released {
            let dropped = self.boxes[reorder.from].0 + self.shift.get(reorder.from).map_or(0.0, Tween::value);
            let (lo, hi) = (reorder.from.min(reorder.target), reorder.from.max(reorder.target));
            let slot = if reorder.target < reorder.from {
                self.boxes[lo].0
            } else {
                self.boxes[hi].0 + self.boxes[hi].1 - self.boxes[reorder.from].1
            };
            self.settle = Some((reorder.target, dropped - slot));
            ctx.submit_action::<TitlebarAction>(TitlebarAction::Reorder(reorder.from, reorder.target));
        } else {
            self.slide_to(ctx, &[], None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::reorder_layout;

    #[test]
    fn dragging_past_middle_moves_target() {
        let boxes = [(0.0, 100.0), (100.0, 100.0), (200.0, 100.0)];
        assert_eq!(reorder_layout(&boxes, 0, 30.0).0, 0);
        let (target, shift) = reorder_layout(&boxes, 0, 60.0);
        assert_eq!(target, 1);
        assert_eq!(shift, [60.0, -100.0, 0.0]);
        assert_eq!(reorder_layout(&boxes, 0, 160.0).0, 2);
        assert_eq!(reorder_layout(&boxes, 2, -160.0).0, 0);
    }
}
