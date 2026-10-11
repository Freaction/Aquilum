use super::*;

impl Host {
    pub(super) fn wake(&self) -> Option<Instant> {
        #[cfg(not(feature = "installed"))]
        if let Some(at) = self.replay.as_ref().and_then(crate::replay::Replay::wake) {
            return Some(at);
        }
        #[cfg(not(feature = "installed"))]
        if self.replay.as_ref().is_some_and(|r| !r.running()) {
            return Some(self.started + Duration::from_millis(2100));
        }
        #[cfg(feature = "memory-probe")]
        if let Some(at) = self.probe.as_ref().and_then(crate::memory_probe::Probe::wake) {
            return Some(at);
        }
        None
    }

    #[cfg(not(feature = "installed"))]
    pub(super) fn replay_scroll(&mut self, event_loop: &dyn ActiveEventLoop, now: Instant) {
        let settling = self.last_anim.is_some() || self.started.elapsed() < Duration::from_secs(2);
        if self.metrics.frames == 0 || settling && self.replay.as_ref().is_some_and(|r| !r.running()) {
            return;
        }
        if self.replay.as_ref().is_some_and(crate::replay::Replay::is_graph) {
            let Some(root) = &mut self.root else { return };
            if !self.app.graph_ready() {
                self.app.open_graph_for_replay(root);
                return;
            }
        }
        let Some(step) = self.replay.as_mut().and_then(|r| r.step(now)) else {
            if self.replay.as_ref().is_some_and(crate::replay::Replay::finished) {
                if let Some(replay) = &self.replay {
                    replay.write();
                }
                self.close(event_loop);
            }
            return;
        };
        let _zone = aq_trace::zone("прогон: колесо");
        let scale = self.scale();
        let target = self.app.graph_view().or_else(|| self.app.editor_id());
        let center = self.root.as_ref().zip(target).and_then(|(root, id)| root.get_widget(id)).map(|body| body.ctx().bounding_box().center());
        if let Some(center) = center {
            self.pointer.position = winit::dpi::PhysicalPosition::new(center.x * scale * 0.8, (center.y * scale).min(f64::from(self.window().surface_size().height) * 0.8));
        }
        let state = self.pointer_state();
        match step {
            crate::replay::Step::Wheel(lines) => {
                self.pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: PRIMARY_MOUSE, delta: ScrollDelta::LineDelta(0.0, lines), state }));
            }
            crate::replay::Step::Pinch(delta) => self.pinch(f64::from(delta)),
            crate::replay::Step::Focus => {
                if let Some(root) = &mut self.root {
                    self.app.toggle_focus_mode(root);
                }
            }
            crate::replay::Step::Pan(dx, dy) => {
                let delta = ScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(f64::from(dx), f64::from(dy)));
                self.pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: PRIMARY_MOUSE, delta, state }));
            }
        }
        self.handle_signals(event_loop);
    }

    #[cfg(feature = "memory-probe")]
    pub(super) fn probe_step(&mut self, event_loop: &dyn ActiveEventLoop, now: Instant) {
        if self.metrics.frames == 0 {
            return;
        }
        let Some(probe) = &mut self.probe else { return };
        if probe.finished() {
            self.close(event_loop);
            return;
        }
        let Some(root) = &mut self.root else { return };
        if let Some(step) = probe.step(now).cloned() {
            if let crate::memory_probe::Step::Minimize(minimized) = step {
                self.window().set_minimized(minimized);
                return;
            }
            self.app.probe(root, &step);
            if let crate::memory_probe::Step::Mark(name) | crate::memory_probe::Step::Evict(name) = &step {
                probe.record(name, &self.app.probe_stats(root));
            }
            self.handle_signals(event_loop);
            self.request_redraw();
        }
    }
}
