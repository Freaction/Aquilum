use super::*;
use crate::ui::editor::grip::Grip;
use crate::ui::keymap::gesture::GRIP_LINE_SLOP;
use masonry::core::{LayerType, PointerType};

#[derive(Clone, Copy, Default, PartialEq)]
pub(super) enum Mode {
    #[default]
    Off,
    On,
    Held(Vec2),
}

#[derive(Default)]
pub(super) struct Grips {
    mode: Mode,
    pub(super) at: [Option<Point>; 2],
}

impl Grips {
    pub(super) fn on(&self) -> bool {
        self.mode != Mode::Off
    }

    pub(super) fn off(&mut self) -> bool {
        std::mem::take(&mut self.mode) != Mode::Off
    }
}

fn frames(ends: [Rect; 2]) -> [Rect; 2] {
    let [start, end] = ends.map(|caret| Point::new(caret.center().x, caret.y1));
    [Grip::<false>::frame(start), Grip::<true>::frame(end)]
}

fn sync<const END: bool>(ctx: &mut ComposeCtx<'_>, at: Option<Point>) {
    match (ctx.get_attached_layer::<Grip<END>>(), at) {
        (Some(layer), Some(at)) => ctx.reposition_layer(layer, at),
        (Some(layer), None) => ctx.remove_layer(layer),
        (None, Some(at)) => {
            let grip = Grip::<END>::new(ctx.widget_id());
            ctx.create_attached_layer(LayerType::Other, grip, at);
        }
        (None, None) => {}
    }
}

impl TextEditor {
    pub(super) fn ends(&self) -> [Rect; 2] {
        let range = self.selection();
        [self.caret_at(range.start, 0, Affinity::Downstream), self.caret_at(range.end, 0, Affinity::Upstream)]
    }

    pub(super) fn grips_wanted(&self) -> bool {
        self.grips.on() && self.focused && self.laid_out() && !self.selection().is_empty() && !self.table_active() && self.media.selected.is_none()
    }

    pub(super) fn place_grips(&mut self, ctx: &mut ComposeCtx<'_>) {
        let wanted = self.grips_wanted();
        if !wanted && self.grips.at == [None; 2] {
            return;
        }
        let (transform, clip) = (ctx.window_transform(), ctx.window_clip());
        let ends = self.ends().map(|caret| transform.transform_rect_bbox(caret));
        let frames = frames(ends);
        let seen = |caret: Rect| (clip.x0..=clip.x1).contains(&caret.center().x) && (clip.y0..=clip.y1).contains(&caret.y1);
        let at: [Option<Point>; 2] = std::array::from_fn(|i| (wanted && seen(ends[i])).then_some(frames[i].origin()));
        if at[0] != self.grips.at[0] {
            sync::<false>(ctx, at[0]);
        }
        if at[1] != self.grips.at[1] {
            sync::<true>(ctx, at[1]);
        }
        self.grips.at = at;
    }

    pub(super) fn grab_grip(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
        let ends = self.ends();
        let frames = frames(ends);
        let Some(end) = (0..2).find(|&i| self.grips.at[i].is_some() && frames[i].contains(point)) else { return false };
        let range = self.selection();
        (self.anchor, self.focus, self.affinity) = if end == 1 { (range.start, range.end, Affinity::Upstream) } else { (range.end, range.start, Affinity::Downstream) };
        self.grips.mode = Mode::Held(ends[end].center() - point);
        self.menu = super::menu::Menu::Pressed(1);
        ctx.capture_pointer();
        true
    }

    pub(super) fn drag_grip(&mut self, point: Point) -> bool {
        let Mode::Held(offset) = self.grips.mode else { return false };
        let target = point + offset;
        let line = self.caret_at(self.focus, 0, self.affinity);
        let slop = line.height() * GRIP_LINE_SLOP;
        let y = if target.y > line.y0 - slop && target.y < line.y1 + slop { line.center().y } else { target.y };
        let hit = self.hit(Point::new(target.x, y));
        if hit.0 != self.anchor {
            self.move_to(hit, true);
        }
        true
    }

    pub(super) fn release_grips(&mut self, ctx: &mut EventCtx<'_>, pointer: Option<PointerType>) {
        let mode = match (pointer, self.grips.mode) {
            (Some(pointer), _) if crate::ui::keymap::pointer::grips(pointer) => Mode::On,
            (None, Mode::Held(_)) => Mode::On,
            (None, mode) => mode,
            (Some(_), _) => Mode::Off,
        };
        if std::mem::replace(&mut self.grips.mode, mode) != mode {
            ctx.request_compose();
        }
    }
}
