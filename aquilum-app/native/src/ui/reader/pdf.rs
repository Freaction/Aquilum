use super::*;

impl ReaderView {
    #[cfg(test)]
    pub fn pdf_rendered(&self) -> bool {
        self.session.pdf().is_some_and(|p| p.current_frame())
    }

    pub fn is_pdf(&self) -> bool {
        self.session.pdf().is_some()
    }

    pub fn trim_pdf(this: &mut WidgetMut<'_, Self>) {
        if let Some(pdf) = this.widget.session.pdf_mut() {
            pdf.trim();
            this.ctx.request_layout();
        }
    }

    pub fn poll_pdf(this: &mut WidgetMut<'_, Self>) -> Option<u32> {
        let pdf = this.widget.session.pdf_mut()?;
        if !pdf.poll() {
            return None;
        }
        let total = pdf.total();
        this.widget.total = total.max(1);
        this.widget.report = true;
        this.ctx.request_layout();
        Some(total)
    }

    pub fn pdf_control(this: &mut WidgetMut<'_, Self>, index: usize) {
        if this.widget.apply_pdf_control(index) {
            this.ctx.request_layout();
        }
    }

    fn apply_pdf_control(&mut self, index: usize) -> bool {
        let center = self.body.size().to_rect().center();
        let Some(pdf) = self.session.pdf_mut() else { return false };
        match index {
            0 => {
                pdf.turn(false);
            }
            1 => {
                pdf.turn(true);
            }
            2 => {
                pdf.zoom(1.0 / 1.2, center);
            }
            3 => {
                pdf.zoom(1.2, center);
            }
            4 => pdf.fit(false),
            5 => pdf.fit(true),
            _ => return false,
        }
        self.report = true;
        true
    }

    pub fn pdf_zoom(this: &mut WidgetMut<'_, Self>, command: Command) -> bool {
        let handled = this.widget.zoom_pdf(command);
        if handled {
            this.ctx.request_layout();
        }
        handled
    }

    pub(super) fn zoom_pdf(&mut self, command: Command) -> bool {
        self.apply_pdf_control(match command {
            Command::ZoomIn => 3,
            Command::ZoomOut => 2,
            Command::ZoomReset => 4,
            _ => return false,
        })
    }

    pub(super) fn arrange_pdf(&mut self, ctx: &mut LayoutCtx<'_>) {
        if !self.is_pdf() {
            return;
        }
        let count = self.pdf_controls.len() as f64;
        let width = (self.room.width() - size::SPACE_24).min(count * (size::SIZE_32 + size::SPACE_8) + size::SPACE_16).max(1.0);
        let cell = (width - size::SPACE_16).max(1.0) / count;
        let height = size::SIZE_32 + size::SPACE_16;
        let left = self.room.x0 + (self.room.width() - width) / 2.0;
        let bottom = self.body.y1 - size::SPACE_16;
        self.pdf_bar = Rect::new(left, bottom - height, left + width, bottom);
        for (i, control) in self.pdf_controls.iter_mut().enumerate() {
            ctx.run_layout(control, Size::new(cell.max(1.0), size::SIZE_32));
            ctx.place_child(control, Point::new(left + size::SPACE_8 + i as f64 * cell, self.pdf_bar.y0 + size::SPACE_8));
        }
    }

    pub(super) fn draw_pdf(&mut self, ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_>) {
        let Some(pdf) = self.session.pdf() else { return };
        let body = self.body;
        let offset = body.origin().to_vec2();
        let page = pdf.page_rect() + offset;
        painter.with_fill_clip(body, |painter| {
            painter.fill(page, masonry::peniko::Color::WHITE).draw();
            if let Some((image, transform)) = pdf.image() {
                painter.with_fill_clip(page, |painter| painter.draw_image(image, Affine::translate(offset) * transform));
            }
        });
        if let Some(error) = &pdf.error {
            let message = format!("{}: {error}", t("reader.pdf.error"));
            let line = Self::line_height();
            let layout = self.preview_line.layout(ctx, &message, (body.width() - size::SPACE_32).max(1.0), Style::ui(line));
            render_text(painter, Affine::translate((body.x0 + size::SPACE_16, body.y0 + size::SPACE_16)), layout, &[theme::current().text_secondary.into()], true);
        }
        let tm = theme::current();
        let shape = RoundedRect::from_rect(self.pdf_bar, size::RADIUS_XL);
        painter.fill(shape, tm.bg_surface.multiply_alpha(number::OPACITY_60 as f32)).draw();
    }

    pub(super) fn pdf_pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Gesture(gesture) => {
                if let masonry::core::PointerGesture::Pinch(delta) = gesture.gesture {
                    let p = ctx.local_position(gesture.state.position);
                    if !self.body.contains(p) || self.pdf_bar.contains(p) {
                        return;
                    }
                    self.pdf_drag = None;
                    let changed = self.session.pdf_mut().unwrap().zoom((1.0 + f64::from(delta)).max(0.01), p - self.body.origin().to_vec2());
                    self.moved(ctx, changed);
                    ctx.set_handled();
                }
            }
            PointerEvent::Scroll(PointerScrollEvent { delta, state, .. }) => {
                let p = ctx.local_position(state.position);
                if !self.body.contains(p) || self.pdf_bar.contains(p) {
                    return;
                }
                let delta = crate::ui::scroll::pixels(delta, ctx.scale_factor(), self.body.height());
                let changed = if keymap::pointer::zoom(state.modifiers) {
                    self.session.pdf_mut().unwrap().zoom((delta.y * 0.01).exp(), p - self.body.origin().to_vec2())
                } else {
                    self.pdf_drag = None;
                    self.session.pdf_mut().unwrap().pan(delta)
                };
                self.moved(ctx, changed);
                ctx.set_handled();
            }
            PointerEvent::Down(down) if keymap::pointer::press(down.button) => {
                let p = ctx.local_position(down.state.position);
                if self.body.contains(p) && !self.pdf_bar.contains(p) {
                    self.pdf_drag = Some(p);
                    ctx.capture_pointer();
                    ctx.set_handled();
                }
            }
            PointerEvent::Move(update) => {
                let p = ctx.local_position(update.current.position);
                if let Some(last) = self.pdf_drag {
                    self.pdf_drag = Some(p);
                    let changed = self.session.pdf_mut().unwrap().pan(p - last);
                    self.moved(ctx, changed);
                    ctx.set_handled();
                }
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => {
                self.pdf_drag = None;
            }
            _ => {}
        }
    }
}
