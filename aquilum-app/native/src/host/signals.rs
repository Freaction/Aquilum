use super::*;

impl Host {
    pub(super) fn handle_signals(&mut self, event_loop: &dyn ActiveEventLoop) {
        let Some(window) = self.window.clone() else { return };
        let signals = std::mem::take(&mut *self.signals.borrow_mut());
        let mut actions = Vec::new();
        for signal in signals {
            match signal {
                RenderRootSignal::RequestRedraw => self.request_redraw(),
                RenderRootSignal::RequestAnimFrame => {
                    self.anim_pending = true;
                    if !self.occluded {
                        self.vsync.request();
                    }
                }
                RenderRootSignal::StartIme => {
                    if !self.ime_enabled {
                        // winit 0.31 отклоняет запрос, если возможность заявлена без начального
                        // значения; настоящую позицию каретки Masonry пришлёт сигналом ImeMoved.
                        let data = ImeRequestData::default().with_cursor_area(
                            Position::Logical((0.0, 0.0).into()),
                            Size::Logical((1.0, 1.0).into()),
                        );
                        let request = ImeEnableRequest::new(ImeCapabilities::new().with_cursor_area(), data)
                            .expect("запрос IME: возможности и данные согласованы");
                        self.ime_enabled = window.request_ime_update(ImeRequest::Enable(request)).is_ok();
                    }
                }
                RenderRootSignal::EndIme => {
                    let _ = window.request_ime_update(ImeRequest::Disable);
                    self.ime_enabled = false;
                    self.ime_area = None;
                }
                RenderRootSignal::ImeMoved(position, size) => {
                    if self.ime_enabled && self.ime_area != Some((position, size)) {
                        self.ime_area = Some((position, size));
                        // Логические пиксели Masonry включают масштаб интерфейса, а winit ждёт
                        // логические пиксели окна.
                        let k = self.app.scale();
                        let position = LogicalPosition::new(position.x * k, position.y * k);
                        let size = LogicalSize::new(size.width * k, size.height * k);
                        let data = ImeRequestData::default()
                            .with_cursor_area(Position::Logical(position), Size::Logical(size));
                        let _ = window.request_ime_update(ImeRequest::Update(data));
                    }
                }
                RenderRootSignal::ClipboardStore(text) => {
                    crate::clipboard::copy(text);
                }
                RenderRootSignal::SetCursor(cursor) => {
                    self.cursor = cursor;
                    if self.resize_edge.is_none() {
                        window.set_cursor(cursor.into());
                    }
                }
                RenderRootSignal::SetTitle(title) => window.set_title(&title),
                RenderRootSignal::TakeFocus => window.focus_window(),
                RenderRootSignal::Exit => event_loop.exit(),
                RenderRootSignal::NewLayer(_, root, pos) => {
                    self.root.as_mut().expect("корень есть").add_layer(root, pos)
                }
                RenderRootSignal::RemoveLayer(id) => self.root.as_mut().expect("корень есть").remove_layer(id),
                RenderRootSignal::RepositionLayer(id, pos) => {
                    self.root.as_mut().expect("корень есть").reposition_layer(id, pos)
                }
                RenderRootSignal::Action(action, id) => actions.push((action, id)),
                // Перетаскивание окна, меню окна — по мере переноса интерфейса.
                _ => {}
            }
        }
        // Действия обрабатываются после разбора сигналов: им нужен изменяемый RenderRoot.
        let mut title_changed = false;
        let mut rescale = false;
        let mut commands = Vec::new();
        let size = self.logical_size();
        if let Some(root) = &mut self.root {
            for (action, id) in actions {
                let outcome = self.app.on_action(root, id, action, size);
                title_changed |= outcome.title;
                rescale |= outcome.rescale;
                commands.extend(outcome.window);
            }
        }
        if rescale {
            self.rescale();
        }
        for command in commands {
            match command {
                WindowCommand::Drag => {
                    let _ = window.drag_window();
                }
                WindowCommand::ToggleMaximize => window.set_maximized(!window.is_maximized()),
                WindowCommand::Minimize => window.set_minimized(true),
                WindowCommand::Close => {
                    self.close(event_loop);
                    return;
                }
            }
        }
        if title_changed {
            self.show_title();
        }
        // Обработка действий могла породить новые сигналы (перерисовка, фокус).
        if !self.signals.borrow().is_empty() {
            self.handle_signals(event_loop);
        }
    }
}
