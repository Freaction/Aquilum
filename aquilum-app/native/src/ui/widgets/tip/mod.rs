mod layer;
#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::time::Instant;

use masonry::core::{EventCtx, LayerType, PointerEvent, Update, UpdateCtx};
use masonry::kurbo::{Axis, Point, Rect};

use layer::TipLayer;

use crate::ui::keymap::gesture::{TIP_DELAY, TIP_RESHOW};
use crate::ui::keymap::{self, Command, Scope, pointer};

thread_local! {
    static HIDDEN: Cell<Option<Instant>> = const { Cell::new(None) };
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Idle,
    Waiting(Instant),
    Shown,
    Spent,
}

pub struct Tip {
    pub(in crate::ui::widgets) label: bool,
    keys: Option<String>,
    axis: Axis,
    state: State,
}

impl Tip {
    pub fn new(label: bool) -> Self {
        Tip { label, keys: None, axis: Axis::Vertical, state: State::Idle }
    }

    pub fn keys(&mut self, command: Command) {
        self.keys = Some(keymap::label(Scope::Global, command)).filter(|keys| !keys.is_empty());
    }

    pub fn beside(&mut self) {
        self.axis = Axis::Horizontal;
    }

    pub fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Move(update) if (self.label || self.keys.is_some()) && self.state == State::Idle && pointer::hovers(update.pointer.pointer_type) => {
                let warm = HIDDEN.get().is_some_and(|at| at.elapsed() < TIP_DELAY);
                let due = Instant::now() + if warm { TIP_RESHOW } else { TIP_DELAY };
                self.state = State::Waiting(due);
                crate::ui::timer::wake(ctx.widget_id(), due);
            }
            PointerEvent::Down(_) | PointerEvent::Scroll(_) if self.state != State::Spent => {
                self.state = State::Spent;
                ctx.request_anim_frame();
            }
            _ => {}
        }
    }

    pub fn update(&mut self, ctx: &mut UpdateCtx<'_>, event: &Update) {
        if matches!(event, Update::HoveredChanged(false) | Update::DisabledChanged(true) | Update::StashedChanged(true)) {
            self.state = State::Idle;
            self.sync(ctx);
        }
    }

    pub fn anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, text: &str) {
        if let State::Waiting(due) = self.state {
            if Instant::now() < due {
                crate::ui::timer::wake(ctx.widget_id(), due);
                return;
            }
            self.state = if ctx.is_hovered() { State::Shown } else { State::Idle };
            if self.state == State::Shown {
                let trigger = ctx.window_transform().transform_rect_bbox(Rect::from_origin_size(Point::ORIGIN, ctx.border_box().size()));
                let layer = TipLayer::new(text, self.keys.as_deref(), trigger, self.axis);
                ctx.create_attached_layer(LayerType::Tooltip(text.to_owned()), layer, Point::ORIGIN);
            }
        }
        self.sync(ctx);
    }

    fn sync(&mut self, ctx: &mut UpdateCtx<'_>) {
        if self.state != State::Shown
            && let Some(layer) = ctx.get_attached_layer::<TipLayer>()
        {
            ctx.remove_layer(layer);
            HIDDEN.set(Some(Instant::now()));
        }
    }
}
