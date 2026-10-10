//! Доступность: дерево AccessKit, которое строит Masonry, отдаётся платформенному адаптеру.
//!
//! `accesskit_winit` есть только под winit 0.30, поэтому адаптер подключается к окну напрямую
//! по его хэндлу: Windows — UIA через подмену оконной процедуры, macOS — подмена NSView,
//! Linux и BSD — AT-SPI через D-Bus.
//!
//! Обработчики адаптера вызываются из оконной процедуры, где к `RenderRoot` не подобраться:
//! они кладут запрос в канал и будят цикл winit, хост разбирает канал в `proxy_wake_up`.

use std::sync::mpsc::{Receiver, Sender, channel};

use masonry::accesskit::{ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, TreeUpdate};
use winit::event::WindowEvent;
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

pub enum Request {
    /// Экранный диктор или другой клиент UIA запросил дерево: его надо построить целиком.
    Activate,
    /// Действие над узлом: нажать, поставить фокус, прокрутить…
    Action(ActionRequest),
}

pub struct Accessibility {
    #[cfg(target_os = "windows")]
    adapter: accesskit_windows::SubclassingAdapter,
    #[cfg(target_os = "macos")]
    adapter: accesskit_macos::SubclassingAdapter,
    #[cfg(all(unix, not(target_os = "macos")))]
    adapter: accesskit_unix::Adapter,
    requests: Receiver<Request>,
}

struct Forward {
    sender: Sender<Request>,
    proxy: EventLoopProxy,
}

impl Forward {
    fn send(&self, request: Request) {
        if self.sender.send(request).is_ok() {
            self.proxy.wake_up();
        }
    }
}

impl ActivationHandler for Forward {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        // Полное дерево придёт с ближайшей перерисовкой после EnableAccessTree.
        self.send(Request::Activate);
        None
    }
}

impl ActionHandler for Forward {
    fn do_action(&mut self, request: ActionRequest) {
        self.send(Request::Action(request));
    }
}

impl DeactivationHandler for Forward {
    fn deactivate_accessibility(&mut self) {}
}

impl Accessibility {
    /// Подключает адаптер к окну. Окно должно быть ещё невидимым: так требует Windows-адаптер.
    pub fn attach(window: &dyn Window, proxy: EventLoopProxy) -> Option<Self> {
        let (sender, requests) = channel::<Request>();
        let activation = Forward { sender: sender.clone(), proxy: proxy.clone() };
        let action = Forward { sender, proxy };
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let handle = {
            use winit::raw_window_handle::HasWindowHandle;
            window.window_handle().ok()?.as_raw()
        };
        #[cfg(target_os = "windows")]
        let adapter = {
            let winit::raw_window_handle::RawWindowHandle::Win32(handle) = handle else { return None };
            accesskit_windows::SubclassingAdapter::new(accesskit_windows::HWND(handle.hwnd.get() as *mut std::ffi::c_void), activation, action)
        };
        #[cfg(target_os = "macos")]
        let adapter = {
            let winit::raw_window_handle::RawWindowHandle::AppKit(handle) = handle else { return None };
            unsafe { accesskit_macos::SubclassingAdapter::new(handle.ns_view.as_ptr(), activation, action) }
        };
        #[cfg(all(unix, not(target_os = "macos")))]
        let adapter = {
            let _ = window;
            let deactivation = Forward { sender: activation.sender.clone(), proxy: activation.proxy.clone() };
            accesskit_unix::Adapter::new(activation, action, deactivation)
        };
        Some(Accessibility { adapter, requests })
    }

    pub fn drain(&self) -> Vec<Request> {
        self.requests.try_iter().collect()
    }

    /// Передаёт адаптеру изменения дерева, если клиент доступности подключён.
    pub fn update(&mut self, update: TreeUpdate) {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        if let Some(events) = self.adapter.update_if_active(|| update) {
            events.raise();
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        self.adapter.update_if_active(|| update);
    }

    pub fn window_event(&mut self, window: &dyn Window, event: &WindowEvent) {
        #[cfg(target_os = "macos")]
        if let WindowEvent::Focused(focused) = event
            && let Some(events) = self.adapter.update_view_focus_state(*focused)
        {
            events.raise();
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        match event {
            WindowEvent::Focused(focused) => self.adapter.update_window_focus_state(*focused),
            WindowEvent::Moved(_) | WindowEvent::SurfaceResized(_) => {
                let rect = |at: winit::dpi::PhysicalPosition<i32>, size: winit::dpi::PhysicalSize<u32>| {
                    masonry::accesskit::Rect::from_origin_size((f64::from(at.x), f64::from(at.y)), (f64::from(size.width), f64::from(size.height)))
                };
                let outer = rect(window.outer_position().unwrap_or_default(), window.outer_size());
                let inner = rect(window.surface_position().cast(), window.surface_size());
                self.adapter.set_root_window_bounds(outer, inner);
            }
            _ => {}
        }
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let _ = (window, event);
    }
}
