use super::*;

pub(super) trait Submit {
    fn submit(&mut self, action: ReaderAction);
    fn animate(&mut self) -> bool;
}

impl Submit for EventCtx<'_> {
    fn submit(&mut self, action: ReaderAction) {
        self.submit_action::<ReaderAction>(action);
    }

    fn animate(&mut self) -> bool {
        self.request_anim_frame();
        true
    }
}

impl Submit for LayoutCtx<'_> {
    fn submit(&mut self, action: ReaderAction) {
        self.submit_action::<ReaderAction>(action);
    }

    fn animate(&mut self) -> bool {
        false
    }
}

impl ReaderView {
    pub(super) fn moved(&mut self, ctx: &mut EventCtx<'_>, changed: bool) {
        if !changed {
            return;
        }
        self.report = true;
        self.hovered = None;
        if self.selection.take().is_some() {
            ctx.request_layout();
        }
        if self.session.needs_layout() {
            ctx.request_layout();
        } else {
            self.emit(ctx);
            ctx.request_paint_only();
        }
    }

    pub(super) fn emit(&mut self, ctx: &mut impl Submit) {
        if !std::mem::take(&mut self.report) {
            return;
        }
        if let Some((cfi, fraction)) = self.session.location() {
            self.fraction = fraction;
            if self.bar.set(f64::from(self.current()) / f64::from(self.total.max(1))) && !ctx.animate() {
                self.bar.finish();
            }
            ctx.submit(ReaderAction::Moved { cfi, fraction });
        }
    }

    pub(super) fn side(&self, p: Point) -> Option<Side> {
        if !self.body.contains(p) {
            return None;
        }
        let nav = nav_width(self.body.width());
        if p.x < self.body.x0 + nav {
            Some(Side::Prev)
        } else if p.x > self.body.x1 - nav {
            Some(Side::Next)
        } else {
            None
        }
    }

    pub(super) fn turn(&mut self, ctx: &mut EventCtx<'_>, side: Side) {
        let changed = match side {
            Side::Prev => self.session.prev(),
            Side::Next => self.session.next(),
        };
        self.moved(ctx, changed);
    }

    pub(super) fn set_hover(&mut self, ctx: &mut EventCtx<'_>, hover: Option<Option<Side>>) {
        if hover == self.hover {
            return;
        }
        self.hover = hover;
        let mut moving = false;
        for (zone, side) in self.zones.iter_mut().zip([Side::Prev, Side::Next]) {
            let target = match hover {
                None => 0.0,
                Some(over) if over == Some(side) => 1.0,
                Some(_) => 0.7,
            };
            moving |= zone.set(target);
        }
        if moving {
            ctx.request_anim_frame();
        }
        ctx.request_paint_only();
    }
}
