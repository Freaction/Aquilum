use super::*;

impl ReaderView {
    pub(super) fn key_event(&mut self, ctx: &mut EventCtx<'_>, event: &TextEvent) {
        let TextEvent::Keyboard(k) = event else { return };
        if k.state != KeyState::Down {
            return;
        }
        if k.code == Code::KeyC && (k.modifiers.ctrl() || k.modifiers.meta()) && self.popover.is_none() {
            if let Some((from, to)) = self.ordered() {
                let text = self.session.text(from, to);
                if let Err(error) = arboard::Clipboard::new().and_then(|mut c| c.set_text(text)) {
                    report!("не скопировано: {error}");
                }
                ctx.set_handled();
            }
            return;
        }
        match &k.key {
            Key::Named(NamedKey::Escape) => {
                ctx.submit_action::<ReaderAction>(if self.popover.is_some() { ReaderAction::Settings } else { ReaderAction::Close });
            }
            _ if self.popover.is_some() => return,
            Key::Named(NamedKey::ArrowRight | NamedKey::PageDown) => self.turn(ctx, Side::Next),
            Key::Character(c) if c == " " => self.turn(ctx, Side::Next),
            Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) => self.turn(ctx, Side::Prev),
            _ => return,
        }
        ctx.set_handled();
    }

    pub(super) fn pointer_event(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Move(update) => {
                let p = ctx.local_position(update.current.position);
                if self.selecting {
                    if let Some((_, focus)) = self.session.hit(p - self.body.origin().to_vec2(), false)
                        && let Some((anchor, old)) = self.selection
                    {
                        if focus != old {
                            self.selection = Some((anchor, focus));
                            ctx.request_paint_only();
                        }
                    }
                    return;
                }
                let hover = self.body.contains(p).then(|| self.side(p));
                self.set_hover(ctx, hover);
                let local = p - self.body.origin().to_vec2();
                let quote = if self.popover.is_some() { None } else { self.quote_rects.iter().position(|(_, rects)| rects.iter().any(|r| r.contains(local))) };
                self.set_quote_hover(ctx, quote);
            }
            PointerEvent::Leave(_) => {
                self.set_hover(ctx, None);
                self.set_quote_hover(ctx, None);
            }
            PointerEvent::Down(down) => {
                let p = ctx.local_position(down.state.position);
                if self.popover.is_some() {
                    if self.body.contains(p) && !self.popover_rect.contains(p) {
                        ctx.submit_action::<ReaderAction>(ReaderAction::Settings);
                    }
                    return;
                }
                if self.quote_button.is_some() && self.selection.is_some() && self.toolbar.contains(p) {
                    return;
                }
                if let Some((_, cfi)) = self.labels.iter().find(|(r, _)| r.contains(p)) {
                    ctx.submit_action::<ReaderAction>(ReaderAction::QuoteRef(cfi.clone()));
                    return;
                }
                if let Some(side) = self.side(p) {
                    self.turn(ctx, side);
                    return;
                }
                if !self.body.contains(p) {
                    return;
                }
                let had = self.selection.is_some();
                self.selection = self.session.hit(p - self.body.origin().to_vec2(), down.state.count == 2);
                self.selecting = true;
                ctx.capture_pointer();
                if had != self.selection.is_some() {
                    ctx.request_layout();
                }
                ctx.request_paint_only();
            }
            PointerEvent::Up(_) => {
                if !std::mem::take(&mut self.selecting) {
                    return;
                }
                if self.ordered().is_none() {
                    self.selection = None;
                }
                ctx.request_layout();
            }
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                if self.popover.is_some() {
                    return;
                }
                let (dx, dy) = match delta {
                    ScrollDelta::PixelDelta(d) => (d.x / ctx.scale_factor(), d.y / ctx.scale_factor()),
                    ScrollDelta::LineDelta(x, y) => (f64::from(*x) * crate::ui::scroll::LINE, f64::from(*y) * crate::ui::scroll::LINE),
                    _ => (0.0, 0.0),
                };
                if self.settings.scrolled {
                    let changed = self.session.scroll_by(-dy);
                    self.moved(ctx, changed);
                    return;
                }
                self.wheel += if dx.abs() > dy.abs() { -dx } else { -dy };
                if self.wheel.abs() >= WHEEL_STEP {
                    let side = if self.wheel > 0.0 { Side::Next } else { Side::Prev };
                    self.wheel = 0.0;
                    self.turn(ctx, side);
                }
            }
            _ => {}
        }
    }
}
