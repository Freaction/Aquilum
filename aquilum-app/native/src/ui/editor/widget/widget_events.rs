use masonry::core::{EventCtx, PointerButtonEvent, PointerEvent, PointerUpdate, PropertiesMut, TextEvent, UpdateCtx};
use crate::ui::keymap::pointer;
use super::Kind;
use super::paste;
use super::TextEditor;

const BLINK_HALF: std::time::Duration = std::time::Duration::from_millis(500);

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
        if self.doc.next_frame() {
            ctx.request_compose();
        }
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
        if self.copied.is_some_and(|(_, at)| at.elapsed() >= super::code::COPIED) {
            self.copied = None;
            ctx.request_render();
        }
        let copied = self.copied.map(|(_, at)| at + super::code::COPIED);
        if !(ctx.is_window_focused() && ctx.is_focus_target()) {
            if let Some(at) = copied {
                crate::ui::timer::wake(ctx.widget_id(), at);
            }
            return;
        }
        let halves = self.blink_start.elapsed().as_millis() / BLINK_HALF.as_millis();
        let visible = halves % 2 == 0;
        if visible != self.caret_visible {
            self.caret_visible = visible;
            ctx.request_post_paint();
        }
        let blink = self.blink_start + BLINK_HALF * (halves as u32 + 1);
        crate::ui::timer::wake(ctx.widget_id(), copied.map_or(blink, |at| at.min(blink)));
    }

    pub(super) fn do_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if !self.laid_out() || self.compose.is_some() {
            return;
        }
        match event {
            PointerEvent::Down(PointerButtonEvent { button, state, pointer: device }) if pointer::press(*button) => {
                self.close_suggest();
                let point = ctx.local_position(state.position);
                let new_tab = pointer::new_tab(state.modifiers);
                if self.table_press(ctx, point, crate::ui::keymap::extend(state.modifiers), new_tab, state.count)
                    || self.media_press(ctx, point, state.count, pointer::reach(device.pointer_type))
                    || self.code_press(ctx, point)
                    || self.card_click(ctx, point, new_tab)
                    || self.toggle_task(ctx, point)
                    || self.open_link(ctx, point, new_tab)
                {
                    return;
                }
                if pointer::add_cursor(state.modifiers) {
                    let (offset, _) = self.hit(point);
                    self.add_cursor_at(offset);
                } else if !crate::ui::keymap::extend(state.modifiers) {
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
            PointerEvent::Move(PointerUpdate { current, pointer: device, .. }) => {
                let point = ctx.local_position(current.position);
                self.hover_text(ctx, point);
                if self.media_move(ctx, point, pointer::reach(device.pointer_type)) || self.table_move(ctx, point, !current.buttons.is_empty()) || self.code_drag(ctx, point) {
                    ctx.set_handled();
                    return;
                }
                if !ctx.is_active() || self.table_active() {
                    return;
                }
                self.drag_at = Some(ctx.window_transform() * point);
                if self.drag_select(point) {
                    self.refresh(ctx);
                }
            }
            PointerEvent::Down(PointerButtonEvent { button, state, .. }) if pointer::context(*button) => {
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
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => {
                self.drag_at = None;
                self.media_release(ctx);
                self.table_release(ctx);
                self.code_release();
            }
            PointerEvent::Scroll(scroll) => {
                let point = ctx.local_position(scroll.state.position);
                let d = crate::ui::scroll::pixels(&scroll.delta, ctx.scale_factor(), ctx.content_box().width());
                let dx = crate::ui::scroll::sideways(d, pointer::sideways(scroll.state.modifiers));
                if dx != 0.0 && (self.table_scroll(ctx, point, -dx) || self.code_scroll(ctx, point, -dx) || self.card_scroll(ctx, point, -dx)) {
                    ctx.set_handled();
                }
            }
            _ => {}
        }
    }

    pub(super) fn do_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        self.restart_blink(ctx);
        if !matches!(event, TextEvent::WindowFocusChange(_)) {
            self.include_caret();
        }
        match event {
            TextEvent::Keyboard(key) if self.laid_out() => {
                if !self.media_key(ctx, key) && !self.table_key(ctx, key) {
                    self.keyboard(ctx, key);
                }
            }
            TextEvent::Ime(ime) if self.table_ime(ctx, ime) => {}
            TextEvent::Ime(ime) => self.ime(ctx, ime),
            TextEvent::ClipboardPaste(text) if self.table_text(ctx, text) => {}
            TextEvent::ClipboardPaste(text) => {
                let text = aquilum_core::files::document::normalize_line_endings(text.clone());
                let separated = paste::separate_pasted_frontmatter(&text);
                let pasted = self.pasted_items(&separated).map_or(separated, std::borrow::Cow::Owned);
                if !self.paste_link(ctx.widget_id(), &pasted) {
                    self.insert(&pasted, Kind::Other);
                }
                ctx.set_handled();
                self.follow_caret = true;
                self.changed(ctx);
            }
            TextEvent::WindowFocusChange(_) => ctx.request_render(),
            _ => {}
        }
    }
}
