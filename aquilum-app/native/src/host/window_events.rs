use super::*;

fn masonry_ime(ime: Ime) -> Option<MasonryIme> {
    match ime {
        Ime::Enabled => Some(MasonryIme::Enabled),
        Ime::Disabled => Some(MasonryIme::Disabled),
        Ime::Preedit(text, cursor) => Some(MasonryIme::Preedit(text, cursor)),
        Ime::Commit(text) => Some(MasonryIme::Commit(text)),
        _ => None,
    }
}

fn masonry_modifiers(state: ModifiersState) -> Modifiers {
    let mut modifiers = Modifiers::empty();
    modifiers.set(Modifiers::SHIFT, state.contains(ModifiersState::SHIFT));
    modifiers.set(Modifiers::CONTROL, state.contains(ModifiersState::CONTROL));
    modifiers.set(Modifiers::ALT, state.contains(ModifiersState::ALT));
    modifiers.set(Modifiers::META, state.contains(ModifiersState::META));
    modifiers
}

impl Host {
    pub(super) fn on_window_event(&mut self, event_loop: &dyn ActiveEventLoop, event: WindowEvent) {
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
        if !matches!(event, WindowEvent::PointerMoved { .. } | WindowEvent::RedrawRequested) {
            self.flush_move();
        }
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
                self.occluded = occluded;
                if !occluded {
                    self.full_next = true;
                    self.request_redraw();
                }
            }
            WindowEvent::SurfaceResized(_) if window.is_minimized() == Some(true) => {}
            WindowEvent::SurfaceResized(size) => self.surface_resized(&*window, size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let scale = scale_factor * self.app.scale();
                if let Some(root) = &mut self.root {
                    ui::rescale(root, scale);
                }
            }
            WindowEvent::Focused(focused) => {
                if !focused {
                    self.app.window_unfocused();
                }
                self.text_event(TextEvent::WindowFocusChange(focused));
            }
            WindowEvent::ModifiersChanged(m) => self.modifiers = masonry_modifiers(m.state()),
            WindowEvent::KeyboardInput { event, is_synthetic: false, .. } => self.keyboard(event),
            WindowEvent::Ime(ime) => {
                let Some(ime) = masonry_ime(ime) else { return };
                self.text_event(TextEvent::Ime(ime));
            }
            WindowEvent::PointerEntered { .. } => self.pointer_event(PointerEvent::Enter(PRIMARY_MOUSE)),
            WindowEvent::PointerLeft { .. } => self.pointer_event(PointerEvent::Leave(PRIMARY_MOUSE)),
            WindowEvent::PointerMoved { position, .. } => self.pointer_moved(position),
            WindowEvent::PointerButton { state, position, button, .. } => {
                if state == ElementState::Pressed
                    && button.clone().mouse_button() == Some(MouseButton::Left)
                    && let Some(edge) = self.resize_edge
                {
                    let _ = window.drag_resize_window(edge);
                    return;
                }
                self.pointer_button(state, position, button);
            }
            WindowEvent::PinchGesture { delta, .. } => self.pinch(delta),
            WindowEvent::MouseWheel { delta, .. } if (self.modifiers.ctrl() || self.modifiers.meta()) && !self.app.graph_shown() => {
                let Some(up) = wheel_up(delta) else { return };
                self.wheel_zoom(up);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let Some(delta) = self.wheel_delta(delta) else { return };
                let state = self.pointer_state();
                self.pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: PRIMARY_MOUSE, delta, state }));
            }
            _ => {}
        }
        self.handle_signals(event_loop);
    }

    fn surface_resized(&mut self, window: &dyn Window, size: winit::dpi::PhysicalSize<u32>) {
        let maximized = window.is_maximized();
        let logical = self.logical_size();
        if let Some(root) = &mut self.root {
            self.app.set_maximized(root, maximized);
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
}

fn wheel_up(delta: MouseScrollDelta) -> Option<bool> {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => Some(y > 0.0),
        MouseScrollDelta::PixelDelta(p) => Some(p.y > 0.0),
        _ => None,
    }
}
