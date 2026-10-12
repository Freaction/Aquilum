mod handler;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::DirectManipulation::{
    DIRECTMANIPULATION_CONFIGURATION_INTERACTION, DIRECTMANIPULATION_CONFIGURATION_SCALING, DIRECTMANIPULATION_CONFIGURATION_TRANSLATION_INERTIA,
    DIRECTMANIPULATION_CONFIGURATION_TRANSLATION_X, DIRECTMANIPULATION_CONFIGURATION_TRANSLATION_Y, DIRECTMANIPULATION_VIEWPORT_OPTIONS_DISABLEPIXELSNAPPING,
    DIRECTMANIPULATION_VIEWPORT_OPTIONS_MANUALUPDATE, DirectManipulationManager, IDirectManipulationManager, IDirectManipulationUpdateManager, IDirectManipulationViewport,
    IDirectManipulationViewportEventHandler,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx};
use windows::Win32::UI::Input::Pointer::GetPointerType;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{DM_POINTERHITTEST, POINTER_INPUT_TYPE, PT_TOUCHPAD};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use handler::Handler;

const SIZE: f32 = 1000.0;
const SUBCLASS: usize = 0x4151_544d;

#[derive(Default)]
struct Gesture {
    running: bool,
    touching: bool,
    last: (f32, f32),
    delta: (f32, f32),
    scale: f32,
    pinch: f32,
    pinching: bool,
    inertia: bool,
    released: bool,
    contact: (f32, f32),
    moved: bool,
    trail: crate::fling::Trail,
    fling: Option<crate::fling::Fling>,
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
                | DIRECTMANIPULATION_CONFIGURATION_SCALING;
            viewport.ActivateConfiguration(configuration).ok()?;
            viewport.SetViewportOptions(DIRECTMANIPULATION_VIEWPORT_OPTIONS_MANUALUPDATE | DIRECTMANIPULATION_VIEWPORT_OPTIONS_DISABLEPIXELSNAPPING).ok()?;
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
        let gesture = self.gesture().borrow();
        gesture.running || gesture.touching || gesture.fling.is_some()
    }

    pub fn update(&self, at: Instant, scale: f64) -> (f32, f32, f32) {
        let _zone = aq_trace::zone("тачпад: обновление");
        let contact = {
            let gesture = self.gesture().borrow();
            gesture.running && !gesture.inertia && !gesture.pinching
        };
        let _ = unsafe { self.updates.Update(None) };
        let mut gesture = self.gesture().borrow_mut();
        let (mut dx, mut dy) = std::mem::take(&mut gesture.delta);
        if contact || std::mem::take(&mut gesture.moved) {
            let (x, y) = gesture.contact;
            gesture.trail.push(at, f64::from(x), f64::from(y));
        }
        if std::mem::take(&mut gesture.released) {
            gesture.fling = gesture.trail.release(scale);
        }
        if let Some(fling) = gesture.fling.as_mut() {
            match fling.advance(at) {
                Some((fx, fy)) => {
                    dx += fx as f32;
                    dy += fy as f32;
                }
                None => gesture.fling = None,
            }
        }
        let pinch = match std::mem::replace(&mut gesture.pinch, 1.0) {
            0.0 => 1.0,
            pinch => pinch,
        };
        drop(gesture);
        if aq_trace::enabled() && (dx, dy) != (0.0, 0.0) {
            aq_trace::counter("тачпад: сырые", f64::from(dy));
            aq_trace::counter("тачпад: от такта, мс", at.elapsed().as_secs_f64() * 1000.0);
        }
        (dx, dy, pinch)
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
