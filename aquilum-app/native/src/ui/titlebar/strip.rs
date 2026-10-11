use super::*;
use crate::ui::widgets::{EASE_OUT, FAST, Tween};

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

fn slide() -> Tween {
    Tween::new(0.0, FAST, EASE_OUT)
}

pub struct TabStrip {
    tabs: Vec<WidgetPod<dyn Widget>>,
    offset: f64,
    content: f64,
    width: f64,
    boxes: Vec<(f64, f64)>,
    reorder: Option<Reorder>,
    shift: Vec<Tween>,
    settle: Option<(usize, f64)>,
}

impl TabStrip {
    pub fn new(tabs: Vec<NewWidget<dyn Widget>>) -> NewWidget<Self> {
        NewWidget::new(TabStrip {
            tabs: tabs.into_iter().map(NewWidget::to_pod).collect(),
            offset: 0.0,
            content: 0.0,
            width: 0.0,
            boxes: Vec::new(),
            reorder: None,
            shift: Vec::new(),
            settle: None,
        })
    }

    pub fn set_tabs(this: &mut WidgetMut<'_, Self>, tabs: Vec<NewWidget<dyn Widget>>) {
        for old in std::mem::take(&mut this.widget.tabs) {
            this.ctx.remove_child(old);
        }
        this.widget.tabs = tabs.into_iter().map(NewWidget::to_pod).collect();
        this.widget.reorder = None;
        this.widget.shift = this.widget.tabs.iter().map(|_| slide()).collect();
        if let Some((index, from)) = this.widget.settle.take()
            && let Some(tween) = this.widget.shift.get_mut(index)
        {
            *tween = Tween::new(from, FAST, EASE_OUT);
            tween.set(0.0);
            this.ctx.request_anim_frame();
        }
        this.ctx.children_changed();
    }

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

    fn clamp(&mut self) {
        self.offset = self.offset.clamp(0.0, (self.content - self.width).max(0.0));
    }

    fn drag_move(&mut self, ctx: &mut EventCtx<'_>, x: f64) {
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
}

impl Widget for TabStrip {
    type Action = TitlebarAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
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
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                let x = ctx.local_position(e.state.position).x + self.offset;
                match self.boxes.iter().position(|&(left, width)| x >= left && x < left + width) {
                    Some(from) => {
                        self.reorder = Some(Reorder { from, start: x, dragging: false, target: from });
                    }
                    None => drag_or_maximize(ctx, event),
                }
            }
            PointerEvent::Move(update) if self.reorder.is_some() => {
                let x = ctx.local_position(update.current.position).x;
                self.drag_move(ctx, x);
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => {
                if let Some(reorder) = self.reorder.take()
                    && reorder.dragging
                {
                    if reorder.target != reorder.from && matches!(event, PointerEvent::Up(_)) {
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
            _ => drag_or_maximize(ctx, event),
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let mut moving = false;
        for tween in &mut self.shift {
            moving |= tween.advance(interval);
        }
        if moving {
            ctx.request_anim_frame();
        }
        ctx.request_layout();
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for tab in &mut self.tabs {
            ctx.register_child(tab);
        }
    }

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(size::TABS_HEIGHT),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::ZERO,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let mut widths = Vec::with_capacity(self.tabs.len());
        for tab in &mut self.tabs {
            let context = LayoutSize::maybe(Axis::Vertical, Some(Length::px(size_.height)));
            let width = ctx.compute_length(tab, LenReq::MaxContent.into(), context, Axis::Horizontal, Some(Length::px(size_.height))).get();
            widths.push(width);
        }
        self.content = widths.iter().sum();
        self.width = size_.width;
        self.clamp();
        self.boxes.clear();
        let mut x = 0.0;
        for (index, (tab, width)) in self.tabs.iter_mut().zip(widths).enumerate() {
            self.boxes.push((x, width));
            ctx.run_layout(tab, Size::new(width, size_.height));
            let moved = self.shift.get(index).map_or(0.0, Tween::value);
            ctx.place_child(tab, Point::new(x - self.offset + moved, 0.0));
            x += width;
        }
        ctx.set_clip_path(Rect::new(0.0, 0.0, size_.width, size_.height + size::BORDER_1));
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::TabList
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        self.tabs.iter().map(WidgetPod::id).collect()
    }
}
