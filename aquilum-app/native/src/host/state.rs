use super::*;

impl Host {
    pub(super) fn window(&self) -> &dyn Window {
        self.window.as_deref().expect("окно создано")
    }

    pub(super) fn scale(&self) -> f64 {
        self.window().scale_factor() * self.app.scale()
    }

    pub(super) fn rescale(&mut self) {
        let scale = self.scale();
        if let Some(root) = &mut self.root {
            ui::rescale(root, scale);
        }
        self.fit_window();
        self.request_redraw();
    }

    pub(super) fn logical_size(&self) -> masonry::kurbo::Size {
        let w = self.window();
        let size = w.surface_size().to_logical::<f64>(w.scale_factor() * self.app.scale());
        masonry::kurbo::Size::new(size.width, size.height)
    }

    fn safe_area(&self) -> masonry::kurbo::Insets {
        let window = self.window();
        #[cfg(target_os = "android")]
        let area = crate::android::safe_area();
        #[cfg(not(target_os = "android"))]
        let area = window.safe_area();
        let area = area.to_logical::<f64>(window.scale_factor() * self.app.scale());
        masonry::kurbo::Insets::new(area.left, area.top, area.right, area.bottom)
    }

    pub(super) fn fit_window(&mut self) {
        let (window, insets) = (self.logical_size(), self.safe_area());
        if let Some(root) = &mut self.root {
            self.app.window_resized(root, window);
            self.app.safe_area_changed(root, insets);
        }
    }

    pub(super) fn restore_maximized(&self, window: &dyn Window) {
        if self.args.size.is_none() && self.window_state.maximized() {
            window.set_maximized(true);
        }
    }

    pub(super) fn reveal(&mut self) {
        if !std::mem::replace(&mut self.shown, true) {
            cloak(self.window(), false);
            self.restore_maximized(self.window());
            self.full_next = true;
            self.request_redraw();
        }
    }

    pub(super) fn request_redraw(&mut self) {
        self.redraw_pending = true;
        if !self.occluded {
            self.vsync.request();
        }
    }

    pub(super) fn show_title(&self) {
        let Some(window) = self.window.as_deref() else { return };
        let title = match self.metrics.summary() {
            Some(summary) => format!("{} · {summary}", self.app.title()),
            None => self.app.title(),
        };
        window.set_title(&title);
    }

    pub(super) fn touchpad_scroll(&mut self, event_loop: &dyn ActiveEventLoop, vblank: Instant) {
        let Some(touchpad) = &self.touchpad else { return };
        let (dx, dy, pinch) = touchpad.update(vblank, self.scale());
        if pinch != 1.0 {
            self.pinch(f64::from(pinch) - 1.0);
        }
        if (dx, dy) == (0.0, 0.0) {
            self.handle_signals(event_loop);
            return;
        }
        aq_trace::counter("тачпад: пиксели", f64::from(dy));
        let state = self.pointer_state();
        let delta = ScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(f64::from(dx), f64::from(dy)));
        self.pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: PRIMARY_MOUSE, delta, state }));
        self.handle_signals(event_loop);
    }

    pub(super) fn trim(&mut self) {
        if let Some(root) = self.root.as_mut() {
            self.app.forget(crate::app::Moment::Hide, root);
        }
        self.trim_renderer();
        if let Some(output) = &mut self.output {
            output.trim();
        }
        self.layers = None;
    }

    pub(super) fn trim_renderer(&mut self) {
        if crate::render::take_trim() {
            self.renderer.trim();
            self.full_next = true;
        }
    }

    pub(super) fn close(&mut self, event_loop: &dyn ActiveEventLoop) {
        if event_loop.exiting() {
            return;
        }
        #[cfg(not(feature = "installed"))]
        if let Some(out) = &self.args.trace_frames {
            let _ = aq_trace::write(out);
        }
        event_loop.exit();
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        crate::clipboard::release();
        if self.args.size.is_none()
            && let Some(window) = &self.window
        {
            self.window_state.persist(&**window);
        }
        self.metrics.write();
    }
}
