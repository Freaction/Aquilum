use super::*;

impl ApplicationHandler for Host {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let attributes = WindowAttributes::default()
            .with_title("Aquilum")
            .with_decorations(false)
            .with_visible(false)
            .with_window_icon(window_icon());
        #[cfg(target_os = "windows")]
        let attributes = attributes.with_platform_attributes(Box::new(
            winit::platform::windows::WindowAttributesWindows::default().with_undecorated_shadow(true).with_class_name(crate::instance::WINDOW_CLASS),
        ));
        let attributes = match self.args.size {
            Some(size) => attributes.with_surface_size(size),
            None => attributes.with_surface_size(self.window_state.size()),
        };
        let attributes = match self.window_state.position().filter(|_| self.args.keep_position) {
            Some(position) => attributes.with_position(position),
            None => attributes,
        };
        let window = event_loop.create_window(attributes).expect("окно");
        self.accessibility = Accessibility::attach(&*window, event_loop.create_proxy());
        cloak(&*window, true);
        window.set_visible(true);
        if self.args.size.is_none() && self.window_state.maximized() {
            window.set_maximized(true);
        }
        self.metrics.window_created(self.started, window.scale_factor());
        let widget = self.app.root_widget(window.theme() == Some(winit::window::Theme::Dark));
        let options = RenderRootOptions {
            default_properties: Arc::new(ui::theme::default_properties()),
            use_system_fonts: true,
            size_policy: WindowSizePolicy::User,
            size: window.surface_size(),
            scale_factor: window.scale_factor() * self.app.scale(),
            test_font: None,
        };
        let signals = self.signals.clone();
        let root = RenderRoot::new(widget, move |signal| signals.borrow_mut().push(signal), options);
        let mut root = root;
        ui::theme::register_fonts(&mut root);
        if !self.args.restore {
            self.app.forget_session();
        }
        let logical = window.surface_size().to_logical::<f64>(window.scale_factor() * self.app.scale());
        self.app.attach_window_controls(&mut root, masonry::kurbo::Size::new(logical.width, logical.height));
        if self.app.restore(&mut root) {
            window.set_title(&self.app.title());
        }
        let window: std::rc::Rc<dyn Window> = std::rc::Rc::from(window);
        match crate::output::Output::new(event_loop.owned_display_handle(), window.clone()) {
            Ok(output) => self.output = Some(output),
            Err(err) => report(&format!("вывод кадра: {err}")),
        }
        self.touchpad = crate::touchpad::Touchpad::attach(&*window);
        self.window = Some(window);
        if let Some(note) = self.args.open.clone() {
            self.app.open(&mut root, note);
        }
        if self.args.focus {
            root.focus_on(self.app.editor_id());
        }
        self.root = Some(root);
        self.proxy_wake_up(event_loop);
        self.redraw();
        if self.output.is_none() {
            self.shown = true;
            cloak(self.window(), false);
        }
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
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

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
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
        if let Some(vblank) = self.vsync.take() {
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

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let _zone = aq_trace::zone(match &event {
            WindowEvent::RedrawRequested => "WM_PAINT",
            WindowEvent::MouseWheel { .. } => "колесо",
            WindowEvent::PointerMoved { .. } => "указатель",
            _ => "событие окна",
        });
        if self.args.trace {
            eprintln!("{:>8.1} мс  {event:?}", self.started.elapsed().as_secs_f64() * 1000.0);
        }
        let Some(window) = self.window.clone() else { return };
        if let Some(accessibility) = self.accessibility.as_mut() {
            accessibility.window_event(&*window, &event);
        }
        match event {
            WindowEvent::CloseRequested => {
                self.close(event_loop);
                return;
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::DragEntered { id, .. } => {
                use winit::data_transfer::TypeHint;
                use winit::event_loop::DndAction;
                let files = event_loop.data_transfer(id).is_ok_and(|data| data.has_type(&TypeHint::UriList));
                let _ = event_loop.set_valid_dnd_actions(id, if files { &[DndAction::Copy] } else { &[] });
            }
            WindowEvent::DragPosition { position, .. } => self.drop_at = position.to_logical::<f64>(self.scale()),
            WindowEvent::DragDropped { id, .. } => {
                let _ = event_loop.fetch_data_transfer(id, &winit::data_transfer::TypeHint::UriList);
            }
            WindowEvent::DataTransferReceived { value, .. } => {
                if let (Ok(paths), Some(root)) = (value.try_as_file_paths(), &mut self.root) {
                    self.app.drop_files(root, &paths, masonry::kurbo::Point::new(self.drop_at.x, self.drop_at.y));
                }
                self.handle_signals(event_loop);
            }
            WindowEvent::Occluded(occluded) => {
                if self.args.trace {
                    eprintln!("окно скрыто: {occluded}");
                }
                self.occluded = occluded;
                if !occluded {
                    self.full_next = true;
                    self.request_redraw();
                }
            }
            WindowEvent::SurfaceResized(_) if window.is_minimized() == Some(true) => {}
            WindowEvent::SurfaceResized(size) => {
                let maximized = window.is_maximized();
                let logical = self.logical_size();
                if let Some(root) = &mut self.root {
                    self.app.set_maximized(root, maximized);
                }
                if let Some(root) = &mut self.root {
                    root.handle_window_event(MasonryWindowEvent::Resize(size));
                    self.app.window_resized(root, logical);
                }
                if self.args.size.is_none()
                    && let Some(window) = &self.window
                {
                    self.window_state.observe(&**window);
                }
                self.redraw();
                self.redraw_pending = false;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let scale = scale_factor * self.app.scale();
                if let Some(root) = &mut self.root {
                    ui::rescale(root, scale);
                }
            }
            WindowEvent::Focused(focused) => self.text_event(TextEvent::WindowFocusChange(focused)),
            WindowEvent::ModifiersChanged(m) => {
                let s = m.state();
                let mut modifiers = Modifiers::empty();
                modifiers.set(Modifiers::SHIFT, s.contains(ModifiersState::SHIFT));
                modifiers.set(Modifiers::CONTROL, s.contains(ModifiersState::CONTROL));
                modifiers.set(Modifiers::ALT, s.contains(ModifiersState::ALT));
                modifiers.set(Modifiers::META, s.contains(ModifiersState::META));
                self.modifiers = modifiers;
            }
            WindowEvent::KeyboardInput { event, is_synthetic: false, .. } => self.keyboard(event),
            WindowEvent::Ime(ime) => {
                let ime = match ime {
                    Ime::Enabled => MasonryIme::Enabled,
                    Ime::Disabled => MasonryIme::Disabled,
                    Ime::Preedit(text, cursor) => MasonryIme::Preedit(text, cursor),
                    Ime::Commit(text) => MasonryIme::Commit(text),
                    _ => return,
                };
                self.text_event(TextEvent::Ime(ime));
            }
            WindowEvent::PointerEntered { .. } => self.pointer_event(PointerEvent::Enter(PRIMARY_MOUSE)),
            WindowEvent::PointerLeft { .. } => self.pointer_event(PointerEvent::Leave(PRIMARY_MOUSE)),
            WindowEvent::PointerMoved { position, .. } => {
                self.update_resize_edge(position);
                self.pointer.position = position;
                let current = self.pointer_state();
                self.pointer_event(PointerEvent::Move(PointerUpdate {
                    pointer: PRIMARY_MOUSE,
                    current,
                    coalesced: vec![],
                    predicted: vec![],
                }));
            }
            WindowEvent::PointerButton { state, position, button, .. } => {
                if state == ElementState::Pressed
                    && button.clone().mouse_button() == Some(MouseButton::Left)
                    && let Some(edge) = self.resize_edge
                {
                    let _ = window.drag_resize_window(edge);
                    return;
                }
                self.pointer.position = position;
                let button = button.mouse_button().and_then(|b| match b {
                    MouseButton::Left => Some(PointerButton::Primary),
                    MouseButton::Right => Some(PointerButton::Secondary),
                    MouseButton::Middle => Some(PointerButton::Auxiliary),
                    MouseButton::Back => Some(PointerButton::X1),
                    MouseButton::Forward => Some(PointerButton::X2),
                    _ => None,
                });
                let pressed = state == ElementState::Pressed;
                if let Some(b) = button {
                    if pressed {
                        self.pointer.buttons.insert(b);
                    } else {
                        self.pointer.buttons.remove(b);
                    }
                }
                if pressed {
                    self.pointer.count = self.click_count(button, position);
                }
                let state = self.pointer_state();
                let event = PointerButtonEvent { pointer: PRIMARY_MOUSE, button, state };
                self.pointer_event(if pressed { PointerEvent::Down(event) } else { PointerEvent::Up(event) });
                let delta = match button {
                    Some(PointerButton::X1) => -1,
                    Some(PointerButton::X2) => 1,
                    _ => 0,
                };
                if !pressed
                    && delta != 0
                    && let Some(root) = &mut self.root
                    && self.app.navigate(root, delta).title
                {
                    self.show_title();
                }
            }
            WindowEvent::PinchGesture { delta, .. } => self.pinch(delta),
            WindowEvent::MouseWheel { delta, .. } if (self.modifiers.ctrl() || self.modifiers.meta()) && !self.app.graph_shown() => {
                let up = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y > 0.0,
                    MouseScrollDelta::PixelDelta(p) => p.y > 0.0,
                    _ => return,
                };
                if self.app.zoom_wheel(up) {
                    self.rescale();
                    if let Some(root) = &mut self.root {
                        self.app.scale_changed(root);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        aq_trace::counter("колесо: строки", f64::from(y));
                        ScrollDelta::LineDelta(x, y)
                    }
                    MouseScrollDelta::PixelDelta(p) => {
                        if self.touchpad.as_ref().is_some_and(crate::touchpad::Touchpad::active) {
                            return;
                        }
                        aq_trace::counter("колесо: пиксели", p.y);
                        ScrollDelta::PixelDelta(p)
                    }
                    _ => return,
                };
                let state = self.pointer_state();
                self.pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: PRIMARY_MOUSE, delta, state }));
            }
            _ => {}
        }
        self.handle_signals(event_loop);
    }
}
