use masonry::core::{EventCtx, Ime, PointerButton, PointerButtonEvent, PointerEvent, PointerUpdate, PropertiesMut, TextEvent, UpdateCtx};
use super::Kind;
use super::paste;
use super::TextEditor;

const BLINK_HALF: std::time::Duration = std::time::Duration::from_millis(500);
const LINE_SCROLL: f64 = 40.0;

impl TextEditor {
    pub(super) fn restart_blink(&mut self, ctx: &mut EventCtx<'_>) {
        self.reset_blink();
        ctx.request_anim_frame();
    }

    pub(super) fn reset_blink(&mut self) {
        self.blink_start = std::time::Instant::now();
        self.caret_visible = true;
    }

    pub(super) fn do_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        self.media_tick(ctx, interval);
        let (changed, playing) = self.videos.tick();
        if changed {
            ctx.request_post_paint();
        }
        if playing {
            ctx.request_anim_frame();
        }
        if let Some((_, start)) = &self.external_reveal {
            if start.elapsed().as_secs_f32() >= 0.9 {
                self.external_reveal = None;
            } else {
                ctx.request_anim_frame();
                ctx.request_render();
            }
        }
        if !(ctx.is_window_focused() && ctx.is_focus_target()) {
            return;
        }
        let halves = self.blink_start.elapsed().as_millis() / BLINK_HALF.as_millis();
        let visible = halves % 2 == 0;
        if visible != self.caret_visible {
            self.caret_visible = visible;
            ctx.request_post_paint();
        }
        crate::ui::timer::wake(ctx.widget_id(), self.blink_start + BLINK_HALF * (halves as u32 + 1));
    }

    pub(super) fn do_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.compose.is_some() || !self.laid_out() {
            return;
        }
        match event {
            PointerEvent::Down(PointerButtonEvent { button: None | Some(PointerButton::Primary), state, .. }) => {
                self.close_suggest();
                let point = ctx.local_position(state.position);
                let new_tab = state.modifiers.ctrl() || state.modifiers.meta();
                if self.table_press(ctx, point, state.modifiers.shift(), new_tab, state.count)
                    || self.media_press(ctx, point, state.count)
                    || self.card_click(ctx, point, new_tab)
                    || self.toggle_task(ctx, point)
                    || self.open_link(ctx, point, new_tab)
                {
                    return;
                }
                if state.modifiers.alt() {
                    let (offset, _) = self.hit(point);
                    self.add_cursor_at(offset);
                } else if !state.modifiers.shift() {
                    self.clear_extra_cursors();
                    self.select_at(point, state.count, false);
                } else {
                    self.select_at(point, state.count, true);
                }
                ctx.request_focus();
                ctx.capture_pointer();
                self.restart_blink(ctx);
                self.refresh(ctx);
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                let point = ctx.local_position(current.position);
                self.hover_text(ctx, point);
                if self.media_move(ctx, point) || self.table_move(ctx, point) || !ctx.is_active() || self.table_active() {
                    return;
                }
                self.drag_select(point);
                self.refresh(ctx);
            }
            PointerEvent::Down(PointerButtonEvent { button: Some(PointerButton::Secondary), state, .. }) => {
                let point = ctx.local_position(state.position);
                if !self.table_context(ctx, point) {
                    if let Some(resolver) = &self.doc.resolver {
                        let at = ctx.window_transform() * point;
                        resolver.request(crate::ui::editor::resolve::Request::ContextMenu {
                            at,
                            has_selection: !self.selection().is_empty(),
                        });
                    }
                }
            }
            PointerEvent::Up(_) => {
                self.media_release(ctx);
                self.table_release(ctx);
            }
            PointerEvent::Leave(_) => {
                self.media_leave(ctx);
                if self.hovered_link.take().is_some() {
                    ctx.request_render();
                }
                if self.hovered_table.take().is_some() {
                    ctx.request_render();
                }
            }
            PointerEvent::Scroll(scroll) => {
                let point = ctx.local_position(scroll.state.position);
                let (dx, dy) = match scroll.delta {
                    masonry::core::ScrollDelta::PixelDelta(p) => (p.x / ctx.scale_factor(), p.y / ctx.scale_factor()),
                    masonry::core::ScrollDelta::LineDelta(x, y) => (f64::from(x) * LINE_SCROLL, f64::from(y) * LINE_SCROLL),
                    _ => (0.0, 0.0),
                };
                let (dx, dy) = if scroll.state.modifiers.shift() && dx == 0.0 { (dy, 0.0) } else { (dx, dy) };
                if dx.abs() > dy.abs() && self.table_scroll(ctx, point, -dx) {
                    ctx.set_handled();
                }
            }
            _ => {}
        }
    }

    pub(super) fn do_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        self.restart_blink(ctx);
        match event {
            TextEvent::Keyboard(key) if self.laid_out() => {
                if !self.media_key(ctx, key) && !self.table_key(ctx, key) {
                    self.keyboard(ctx, key);
                }
            }
            TextEvent::Ime(Ime::Commit(text)) if self.table_text(ctx, text) => {}
            TextEvent::Ime(ime) => self.ime(ctx, ime),
            TextEvent::ClipboardPaste(text) if self.table_text(ctx, text) => {}
            TextEvent::ClipboardPaste(text) => {
                let text = text.replace("\r\n", "\n").replace('\r', "\n");
                let separated = paste::separate_pasted_frontmatter(&text);
                if !self.paste_link(ctx.widget_id(), &separated) {
                    self.insert(&separated, Kind::Other);
                }
                ctx.set_handled();
                self.changed(ctx);
            }
            TextEvent::WindowFocusChange(_) => ctx.request_render(),
            _ => {}
        }
    }
}
