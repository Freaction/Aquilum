use super::*;

impl ApplicationHandler for Host {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.create_window(event_loop);
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.on_wake(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.on_idle(event_loop);
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.on_window_event(event_loop, event);
    }
}

impl Host {
    pub(super) fn on_wake(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }
        let _zone = aq_trace::zone("пробуждение");
        let mut retitle = false;
        if let Some(root) = &mut self.root {
            retitle = self.app.on_core_events(root);
            for request in self.accessibility.as_ref().map(Accessibility::drain).unwrap_or_default() {
                match request {
                    a11y::Request::Activate => {
                        if self.args.trace {
                            eprintln!("доступность: клиент подключился");
                        }
                        root.handle_window_event(MasonryWindowEvent::EnableAccessTree);
                    }
                    a11y::Request::Action(action) => root.handle_access_event(action),
                }
            }
        }
        if retitle {
            self.show_title();
        }
        self.handle_signals(event_loop);
    }

    fn on_idle(&mut self, event_loop: &dyn ActiveEventLoop) {
        let _zone = aq_trace::zone("конец пачки событий");
        let now = Instant::now();
        let minimized = self.window.as_ref().is_some_and(|w| w.is_minimized() == Some(true));
        if minimized != self.minimized {
            self.minimized = minimized;
            if minimized {
                self.trim();
            } else {
                self.request_redraw();
            }
        }
        let vblank = self.vsync.take();
        if vblank.is_some() {
            self.flush_move();
        }
        if let Some(root) = &mut self.root {
            self.app.tick(root, now);
            for id in ui::timer::due(now) {
                if root.get_widget(id).is_some() {
                    root.edit_widget(id, |mut widget| widget.ctx.request_anim_frame());
                }
            }
        }
        self.handle_signals(event_loop);
        if self.app.hold_due(now) {
            self.request_redraw();
        }
        #[cfg(not(feature = "installed"))]
        self.replay_scroll(event_loop, now);
        #[cfg(feature = "memory-probe")]
        self.probe_step(event_loop, now);
        if self.touchpad.as_ref().is_some_and(crate::touchpad::Touchpad::active) {
            self.vsync.request();
        }
        if self.live_at.is_some_and(|t| now >= t) {
            self.live_at = None;
            self.vsync.request();
        }
        if let Some(vblank) = vblank {
            let previous = self.last_vblank.replace(vblank);
            self.touchpad_scroll(event_loop, vblank);
            if std::mem::take(&mut self.anim_pending) {
                self.anim_tick(event_loop, vblank, previous);
            }
            if std::mem::take(&mut self.redraw_pending) {
                self.redraw();
            } else if self.renderer.has_live() {
                self.live_tick();
            }
            if self.renderer.overlay_busy() {
                self.request_redraw();
            }
        }
        let wake = [self.app.deadline(), self.wake(), self.live_at, ui::timer::next()].into_iter().flatten().min();
        event_loop.set_control_flow(match wake {
            _ if event_loop.exiting() => ControlFlow::Poll,
            Some(t) => ControlFlow::WaitUntil(t),
            None => ControlFlow::Wait,
        });
    }
}
