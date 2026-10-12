mod reorder;

use super::*;
use crate::ui::widgets::{EASE_OUT, FAST, Tween};

use reorder::{Reorder, slide};

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
}

impl Widget for TabStrip {
    type Action = TitlebarAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => self.scroll(ctx, delta),
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                let x = ctx.local_position(e.state.position).x + self.offset;
                match self.boxes.iter().position(|&(left, width)| x >= left && x < left + width) {
                    Some(from) => self.reorder = Some(Reorder::new(from, x)),
                    None => drag_or_maximize(ctx, event),
                }
            }
            PointerEvent::Move(update) if self.reorder.is_some() => {
                let x = ctx.local_position(update.current.position).x;
                self.drag_move(ctx, x);
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => self.drop_reorder(ctx, matches!(event, PointerEvent::Up(_))),
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
