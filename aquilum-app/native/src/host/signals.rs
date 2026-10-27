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
                RenderRootSignal::StartIme => self.ime_start(&*window),
                RenderRootSignal::EndIme => self.ime_end(&*window),
                RenderRootSignal::ImeMoved(position, size) => self.ime_moved(&*window, position, size),
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
        self.sync_ime_text(&*window);
        // Обработка действий могла породить новые сигналы (перерисовка, фокус).
        if !self.signals.borrow().is_empty() {
            self.handle_signals(event_loop);
        }
    }
}
