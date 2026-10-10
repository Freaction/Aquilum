use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::DirectManipulation::{
    DIRECTMANIPULATION_CONFIGURATION_INTERACTION, DIRECTMANIPULATION_CONFIGURATION_RAILS_X, DIRECTMANIPULATION_CONFIGURATION_RAILS_Y,
    DIRECTMANIPULATION_CONFIGURATION_SCALING,
    DIRECTMANIPULATION_CONFIGURATION_TRANSLATION_INERTIA, DIRECTMANIPULATION_CONFIGURATION_TRANSLATION_X,
    DIRECTMANIPULATION_CONFIGURATION_TRANSLATION_Y, DIRECTMANIPULATION_INERTIA, DIRECTMANIPULATION_READY, DIRECTMANIPULATION_RUNNING,
    DIRECTMANIPULATION_STATUS,
    DIRECTMANIPULATION_VIEWPORT_OPTIONS_MANUALUPDATE, DirectManipulationManager, IDirectManipulationContent, IDirectManipulationManager,
    IDirectManipulationUpdateManager, IDirectManipulationViewport, IDirectManipulationViewportEventHandler,
    IDirectManipulationViewportEventHandler_Impl,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx};
use windows::Win32::UI::Input::Pointer::GetPointerType;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{DM_POINTERHITTEST, POINTER_INPUT_TYPE, PT_TOUCHPAD};
use windows::core::{Ref, implement};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

const SIZE: f32 = 1000.0;
const INERTIA_SECONDS: f32 = 0.12;
const SUBCLASS: usize = 0x4151_544d;

#[derive(Default)]
struct Gesture {
    active: bool,
    inertia: Option<Instant>,
    last: (f32, f32),
    delta: (f32, f32),
    scale: f32,
    pinch: f32,
}

#[implement(IDirectManipulationViewportEventHandler)]
struct Handler(Rc<RefCell<Gesture>>);

impl IDirectManipulationViewportEventHandler_Impl for Handler_Impl {
    fn OnViewportStatusChanged(&self, viewport: Ref<IDirectManipulationViewport>, current: DIRECTMANIPULATION_STATUS, _previous: DIRECTMANIPULATION_STATUS) -> windows::core::Result<()> {
        let moved = {
            let mut gesture = self.0.borrow_mut();
            gesture.active = current == DIRECTMANIPULATION_RUNNING || current == DIRECTMANIPULATION_INERTIA;
            gesture.inertia = (current == DIRECTMANIPULATION_INERTIA).then(Instant::now);
            if current != DIRECTMANIPULATION_READY {
                return Ok(());
            }
            let pinched = std::mem::replace(&mut gesture.scale, 1.0) != 1.0;
            std::mem::take(&mut gesture.last) != (0.0, 0.0) || pinched
        };
        if moved && let Some(viewport) = viewport.as_ref() {
            unsafe { viewport.ZoomToRect(0.0, 0.0, SIZE, SIZE, false)? };
        }
        Ok(())
    }

    fn OnViewportUpdated(&self, _viewport: Ref<IDirectManipulationViewport>) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnContentUpdated(&self, _viewport: Ref<IDirectManipulationViewport>, content: Ref<IDirectManipulationContent>) -> windows::core::Result<()> {
        let Some(content) = content.as_ref() else { return Ok(()) };
        let mut transform = [0.0f32; 6];
        unsafe { content.GetContentTransform(&mut transform)? };
        let mut gesture = self.0.borrow_mut();
        let (scale, x, y) = (transform[0], transform[4], transform[5]);
        if gesture.scale == 0.0 {
            gesture.scale = 1.0;
        }
        if gesture.pinch == 0.0 {
            gesture.pinch = 1.0;
        }
        if (scale - gesture.scale).abs() > f32::EPSILON {
            gesture.pinch *= scale / gesture.scale;
            gesture.scale = scale;
        } else {
            gesture.delta.0 += x - gesture.last.0;
            gesture.delta.1 += y - gesture.last.1;
        }
        gesture.last = (x, y);
        Ok(())
    }
}

struct Shared {
    viewport: IDirectManipulationViewport,
    gesture: Rc<RefCell<Gesture>>,
}

pub struct Touchpad {
    hwnd: HWND,
    _manager: IDirectManipulationManager,
    updates: IDirectManipulationUpdateManager,
    shared: *mut Shared,
}

unsafe extern "system" fn hit_test(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, data: usize) -> LRESULT {
    if message == DM_POINTERHITTEST {
        let pointer = (wparam.0 & 0xffff) as u32;
        let mut kind = POINTER_INPUT_TYPE::default();
        if unsafe { GetPointerType(pointer, &mut kind) }.is_ok() && kind == PT_TOUCHPAD {
            let shared = unsafe { &*(data as *const Shared) };
            let _ = unsafe { shared.viewport.SetContact(pointer) };
        }
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

impl Touchpad {
    pub fn attach(window: &dyn Window) -> Option<Self> {
        let Ok(RawWindowHandle::Win32(handle)) = window.window_handle().map(|h| h.as_raw()) else { return None };
        let hwnd = HWND(handle.hwnd.get() as *mut std::ffi::c_void);
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let manager: IDirectManipulationManager = CoCreateInstance(&DirectManipulationManager, None, CLSCTX_INPROC_SERVER).ok()?;
            let updates: IDirectManipulationUpdateManager = manager.GetUpdateManager().ok()?;
            let viewport: IDirectManipulationViewport = manager.CreateViewport(None, hwnd).ok()?;
            let configuration = DIRECTMANIPULATION_CONFIGURATION_INTERACTION
                | DIRECTMANIPULATION_CONFIGURATION_TRANSLATION_X
                | DIRECTMANIPULATION_CONFIGURATION_TRANSLATION_Y
                | DIRECTMANIPULATION_CONFIGURATION_TRANSLATION_INERTIA
                | DIRECTMANIPULATION_CONFIGURATION_RAILS_X
                | DIRECTMANIPULATION_CONFIGURATION_RAILS_Y
                | DIRECTMANIPULATION_CONFIGURATION_SCALING;
            viewport.ActivateConfiguration(configuration).ok()?;
            viewport.SetViewportOptions(DIRECTMANIPULATION_VIEWPORT_OPTIONS_MANUALUPDATE).ok()?;
            let gesture = Rc::new(RefCell::new(Gesture::default()));
            let handler: IDirectManipulationViewportEventHandler = Handler(gesture.clone()).into();
            viewport.AddEventHandler(Some(hwnd), &handler).ok()?;
            let rect = RECT { left: 0, top: 0, right: SIZE as i32, bottom: SIZE as i32 };
            viewport.SetViewportRect(&rect).ok()?;
            manager.Activate(hwnd).ok()?;
            viewport.Enable().ok()?;
            let shared = Box::into_raw(Box::new(Shared { viewport, gesture }));
            if !SetWindowSubclass(hwnd, Some(hit_test), SUBCLASS, shared as usize).as_bool() {
                drop(Box::from_raw(shared));
                return None;
            }
            Some(Touchpad { hwnd, _manager: manager, updates, shared })
        }
    }

    fn gesture(&self) -> &RefCell<Gesture> {
        unsafe { &(*self.shared).gesture }
    }

    pub fn active(&self) -> bool {
        self.gesture().borrow().active
    }

    pub fn update(&self) -> (f32, f32, f32) {
        let _ = unsafe { self.updates.Update(None) };
        let mut gesture = self.gesture().borrow_mut();
        let (dx, dy) = std::mem::take(&mut gesture.delta);
        let pinch = match std::mem::replace(&mut gesture.pinch, 1.0) {
            0.0 => 1.0,
            pinch => pinch,
        };
        let Some(start) = gesture.inertia else { return (dx, dy, pinch) };
        let fade = (-start.elapsed().as_secs_f32() / INERTIA_SECONDS).exp();
        if fade < 0.02 {
            drop(gesture);
            let _ = unsafe { (*self.shared).viewport.Stop() };
            return (0.0, 0.0, 1.0);
        }
        (dx * fade, dy * fade, pinch)
    }
}

impl Drop for Touchpad {
    fn drop(&mut self) {
        unsafe {
            let _ = RemoveWindowSubclass(self.hwnd, Some(hit_test), SUBCLASS);
            drop(Box::from_raw(self.shared));
        }
    }
}
