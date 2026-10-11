use super::*;

impl Host {
    pub(super) fn redraw(&mut self) {
        let _zone = aq_trace::zone("кадр");
        let t0 = Instant::now();
        let (Some(window), Some(output), Some(root)) = (self.window.clone(), self.output.as_mut(), self.root.as_mut()) else { return };
        let size = window.surface_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        if let Err(err) = output.resize(size.width, size.height) {
            report(&err);
            return;
        }
        let (layers, access_update) = {
            let _zone = aq_trace::zone("сцена");
            root.run_rewrite_passes();
            self.app.restore_view(root);
            let painted = root.redraw();
            self.signals.borrow_mut().retain(|signal| !matches!(signal, RenderRootSignal::RequestRedraw));
            painted
        };
        let scene = t0.elapsed();
        if let (Some(update), Some(accessibility)) = (access_update, self.accessibility.as_mut()) {
            if self.args.trace {
                eprintln!("доступность: обновление дерева, узлов {}", update.nodes.len());
            }
            accessibility.update(update);
        }
        if self.app.holding() {
            return;
        }
        let (width, height) = (size.width.min(u32::from(u16::MAX)) as u16, size.height.min(u32::from(u16::MAX)) as u16);
        let scale = window.scale_factor() * self.app.scale();
        window.pre_present_notify();
        let background = ui::theme::background();
        if self.background != Some(background) {
            self.background = Some(background);
            set_background(&*window, background);
            let rgba = background.to_rgba8();
            output.set_background(u32::from_be_bytes([rgba.a, rgba.r, rgba.g, rgba.b]));
        }
        let full = std::mem::take(&mut self.full_next) || output.fresh();
        let raster_start = Instant::now();
        let drawn = match self.renderer.draw(&layers, width, height, scale, background, full, output) {
            Ok(drawn) => drawn,
            Err(err) => {
                report(&format!("кадр не нарисован: {err}"));
                self.full_next = true;
                return;
            }
        };
        let raster = raster_start.elapsed();
        let present_start = Instant::now();
        if !drawn.is_empty() {
            output.present(&drawn);
        }
        self.layers = Some(layers);
        self.schedule_live();
        if !self.shown {
            self.shown = true;
            cloak(&*window, false);
        }
        let present = present_start.elapsed();
        #[cfg(not(feature = "installed"))]
        if let Some(replay) = &mut self.replay {
            replay.frame(Instant::now(), t0.elapsed());
        }
        if self.metrics.frame(self.started, [scene, raster, present, t0.elapsed()]) {
            self.show_title();
        }
        if self.metrics.frames == 1 {
            self.metrics.write();
        }
    }

    /// Кадр живых слоёв: без сцены Masonry, только анимация поверх кэшей.
    pub(super) fn live_tick(&mut self) {
        let (Some(output), Some(layers)) = (self.output.as_mut(), self.layers.as_ref()) else { return };
        match self.renderer.live(layers, output) {
            Ok(areas) if !areas.is_empty() => output.present(&areas),
            Ok(_) => {}
            Err(err) => {
                report(&format!("живой слой не нарисован: {err}"));
                self.full_next = true;
            }
        }
        self.schedule_live();
    }

    fn schedule_live(&mut self) {
        let next = self.renderer.next_live().filter(|_| !self.occluded);
        self.live_at = next.map(|wait| Instant::now() + wait);
    }

    /// Анимационный тик: проходы обновления Masonry без растеризации.
    pub(super) fn anim_tick(&mut self, event_loop: &dyn ActiveEventLoop, vblank: Instant, previous: Option<Instant>) {
        let _zone = aq_trace::zone("тик анимации");
        let Some(root) = &mut self.root else { return };
        let since = |t: Instant| vblank.saturating_duration_since(t);
        let elapsed = match self.last_anim.take() {
            Some(t) => since(t),
            None => previous.map_or(Duration::ZERO, |t| since(t).min(Duration::from_micros(16_667))),
        };
        root.handle_window_event(MasonryWindowEvent::AnimFrame(elapsed));
        self.last_anim = root.needs_anim().then_some(vblank);
        self.metrics.anim_ticks += 1;
        self.handle_signals(event_loop);
    }
}
