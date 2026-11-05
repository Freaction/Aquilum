mod reorder;

use super::*;
use crate::ui::scroll::Pan;
use crate::ui::widgets::{EASE_OUT, FAST, Tween};

use reorder::{Reorder, slide};

pub struct TabStrip {
    tabs: Vec<WidgetPod<dyn Widget>>,
    pan: Pan,
    boxes: Vec<(f64, f64)>,
    reorder: Option<Reorder>,
    shift: Vec<Tween>,
    settle: Option<(usize, f64)>,
    active: usize,
    follow: bool,
    frame: Option<Rect>,
}

impl TabStrip {
    pub fn new(tabs: Vec<NewWidget<dyn Widget>>, active: usize) -> NewWidget<Self> {
        NewWidget::new(TabStrip {
            tabs: tabs.into_iter().map(NewWidget::to_pod).collect(),
            pan: Pan::default(),
            boxes: Vec::new(),
            reorder: None,
            shift: Vec::new(),
            settle: None,
            active,
            follow: true,
            frame: None,
        })
    }

    pub fn set_tabs(this: &mut WidgetMut<'_, Self>, tabs: Vec<NewWidget<dyn Widget>>, active: usize) {
        for old in std::mem::take(&mut this.widget.tabs) {
            this.ctx.remove_child(old);
        }
        this.widget.follow |= this.widget.active != active;
        this.widget.active = active;
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
}

impl Widget for TabStrip {
    type Action = TitlebarAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                if self.pan.wheel(ctx, delta) {
                    ctx.request_layout();
                }
            }
            PointerEvent::Down(e) if pointer::press(e.button) => {
                let x = ctx.local_position(e.state.position).x + self.pan.offset;
                if let Some(from) = self.boxes.iter().position(|&(left, width)| x >= left && x < left + width) {
                    self.reorder = Some(Reorder::new(from, x, pointer::drag_slop(e.pointer.pointer_type)));
                }
            }
            PointerEvent::Move(update) if self.reorder.is_some() => {
                let x = ctx.local_position(update.current.position).x;
                self.drag_move(ctx, x);
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => self.drop_reorder(ctx, matches!(event, PointerEvent::Up(_))),
            _ => {}
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
        let height = Some(Length::px(size_.height));
        let widths: Vec<f64> = self.tabs.iter_mut().map(|tab| ctx.compute_length(tab, LenReq::MaxContent.into(), LayoutSize::maybe(Axis::Vertical, height), Axis::Horizontal, height).get()).collect();
        let (ear, gap) = (size::TAB_RADIUS, size::GAP_SM);
        let content = widths.iter().sum::<f64>() + gap * widths.len().saturating_sub(1) as f64 + ear * 2.0;
        self.follow |= self.pan.resize(content, size_.width);
        self.boxes = widths.iter().scan(ear, |x, &width| Some((std::mem::replace(x, *x + width + gap), width))).collect();
        if let Some(&(left, width)) = self.boxes.get(self.active).filter(|_| std::mem::take(&mut self.follow)) {
            self.pan.reveal(left - ear, width + ear * 2.0);
        }
        self.frame = None;
        for (index, (tab, &(x, width))) in self.tabs.iter_mut().zip(&self.boxes).enumerate() {
            ctx.run_layout(tab, Size::new(width, size_.height));
            let moved = self.shift.get(index).map_or(0.0, Tween::value);
            let at = Point::new(x - self.pan.offset + moved, 0.0);
            ctx.place_child(tab, at);
            if index == self.active {
                self.frame = Some(Rect::from_origin_size(at, Size::new(width, size_.height)));
            }
        }
        ctx.set_clip_path(Rect::new(0.0, 0.0, size_.width, size_.height + size::BORDER_1));
        ctx.set_paint_insets(Insets::new(0.0, 0.0, 0.0, size::BORDER_1 * 2.0));
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn post_paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let Some(tab) = self.frame else { return };
        let (bounds, scale) = (ctx.content_box(), ctx.scale_factor());
        let floor = ((tab.y1 + size::BORDER_1) * scale).ceil() / scale;
        painter.push_fill_clip(Rect::new(bounds.x0, bounds.y0, bounds.x1, floor));
        paint_frame(painter, tab, floor);
        painter.pop_clip();
    }

    fn accessibility_role(&self) -> Role {
        Role::TabList
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        self.tabs.iter().map(WidgetPod::id).collect()
    }
}
