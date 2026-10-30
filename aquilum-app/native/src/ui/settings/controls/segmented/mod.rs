mod paint;

use super::*;

pub struct Segmented {
    options: Vec<String>,
    selected: usize,
    hovered: Option<usize>,
    lines: Vec<text::Line>,
    rects: Vec<Rect>,
    shown: f64,
}

pub(super) const SEGMENT_PAD: f64 = 3.0;
pub(super) const SEGMENT_GAP: f64 = 2.0;
pub(super) const SEGMENT_HEIGHT: f64 = 24.0;

impl Segmented {
    pub fn new(options: Vec<String>, selected: usize) -> NewWidget<Self> {
        let lines = options.iter().map(|_| text::Line::default()).collect();
        NewWidget::new(Segmented { options, selected, hovered: None, lines, rects: Vec::new(), shown: selected as f64 })
    }

    pub fn set(this: &mut WidgetMut<'_, Self>, selected: usize) {
        if selected < this.widget.options.len() && this.widget.selected != selected {
            this.widget.selected = selected;
            this.ctx.request_anim_frame();
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }

    fn style() -> text::Style {
        text::Style::ui(size::SIZE_20)
    }

    fn naturals(&mut self, fcx: &mut masonry::parley::FontContext, lcx: &mut masonry::parley::LayoutContext<masonry::core::BrushIndex>) -> Vec<f64> {
        self.lines.iter_mut().zip(&self.options).map(|(line, o)| line.width(fcx, lcx, o, Self::style()).ceil() + size::PADDING_XS * 2.0).collect()
    }

    fn total(widths: &[f64]) -> f64 {
        widths.iter().sum::<f64>() + SEGMENT_GAP * widths.len().saturating_sub(1) as f64 + SEGMENT_PAD * 2.0
    }

    #[cfg(test)]
    pub fn segment(&self, index: usize) -> Option<Rect> {
        self.rects.get(index).copied()
    }

    fn segment_at(&self, p: Point) -> Option<usize> {
        self.rects.iter().position(|r| r.inflate(SEGMENT_GAP / 2.0, SEGMENT_PAD).contains(p))
    }

    fn choose(&mut self, ctx: &mut EventCtx<'_>, index: usize) {
        if index != self.selected && index < self.options.len() {
            self.selected = index;
            ctx.request_anim_frame();
            ctx.request_paint_only();
            ctx.request_accessibility_update();
            ctx.submit_action::<Changed>(Changed::Choice(index));
        }
    }

    fn indicator(&self) -> Option<Rect> {
        let last = self.rects.len().checked_sub(1)?;
        let at = self.shown.clamp(0.0, last as f64);
        let (i, t) = (at.floor() as usize, at.fract());
        let (a, b) = (self.rects[i], self.rects[(i + 1).min(last)]);
        let mix = |x: f64, y: f64| x + (y - x) * t;
        Some(Rect::new(mix(a.x0, b.x0), a.y0, mix(a.x1, b.x1), a.y1))
    }
}

impl Widget for Segmented {
    type Action = Changed;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Move(update) => {
                let hovered = self.segment_at(ctx.local_position(update.current.position));
                if hovered != self.hovered {
                    self.hovered = hovered;
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Leave(_) => {
                self.hovered = None;
                ctx.request_paint_only();
            }
            PointerEvent::Down(e) if crate::ui::keymap::pointer::press(e.button) => {
                if let Some(index) = self.segment_at(ctx.local_position(e.state.position)) {
                    self.choose(ctx, index);
                }
                ctx.set_handled();
            }
            _ => {}
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &masonry::core::TextEvent) {
        use crate::ui::keymap::{self, Command, Scope};
        use masonry::core::keyboard::KeyState;
        let masonry::core::TextEvent::Keyboard(k) = event else { return };
        if k.state != KeyState::Down {
            return;
        }
        let last = self.options.len().saturating_sub(1);
        let index = match keymap::command(&[Scope::Choice], &k.key, k.modifiers) {
            Some(Command::Decrease) => self.selected.saturating_sub(1),
            Some(Command::Increase) => (self.selected + 1).min(last),
            Some(Command::First) => 0,
            Some(Command::Last) => last,
            _ => return,
        };
        self.choose(ctx, index);
        ctx.set_handled();
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let goal = self.selected as f64;
        self.shown = motion::approach(self.shown, goal, motion::step(interval as f64 / 1e9, motion::MEDIUM));
        ctx.request_paint_only();
        if self.shown != goal {
            ctx.request_anim_frame();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FontsChanged => {
                self.lines.iter_mut().for_each(text::Line::clear);
                ctx.request_layout();
            }
            Update::FocusChanged(_) => ctx.request_paint_only(),
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match axis {
            Axis::Vertical => Length::px(SEGMENT_HEIGHT + SEGMENT_PAD * 2.0),
            Axis::Horizontal => {
                let (fcx, lcx) = ctx.text_contexts();
                let natural = Self::total(&self.naturals(fcx, lcx));
                match len_req {
                    LenReq::FitContent(space) => Length::px(space.get().max(natural)),
                    _ => Length::px(natural),
                }
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let (fcx, lcx) = ctx.text_contexts();
        let natural = self.naturals(fcx, lcx);
        let extra = ((size.width - Self::total(&natural)) / natural.len().max(1) as f64).max(0.0);
        let mut left = SEGMENT_PAD;
        self.rects = natural
            .iter()
            .map(|w| {
                let rect = Rect::new(left, SEGMENT_PAD, left + w + extra, size.height - SEGMENT_PAD);
                left = rect.x1 + SEGMENT_GAP;
                rect
            })
            .collect();
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        self.draw(ctx, painter);
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        CursorIcon::Pointer
    }

    fn accessibility_role(&self) -> Role {
        Role::RadioGroup
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        if let Some(option) = self.options.get(self.selected) {
            node.set_value(option.clone());
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
