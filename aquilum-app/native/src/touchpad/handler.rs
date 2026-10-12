use std::cell::RefCell;
use std::rc::Rc;

use windows::Win32::Graphics::DirectManipulation::{
    DIRECTMANIPULATION_INERTIA, DIRECTMANIPULATION_INTERACTION_BEGIN, DIRECTMANIPULATION_INTERACTION_END, DIRECTMANIPULATION_INTERACTION_TYPE,
    DIRECTMANIPULATION_READY, DIRECTMANIPULATION_RUNNING, DIRECTMANIPULATION_STATUS, IDirectManipulationContent, IDirectManipulationInteractionEventHandler,
    IDirectManipulationInteractionEventHandler_Impl, IDirectManipulationViewport, IDirectManipulationViewport2, IDirectManipulationViewportEventHandler,
    IDirectManipulationViewportEventHandler_Impl,
};
use windows::core::{Ref, implement};

use super::{Gesture, SIZE};

#[implement(IDirectManipulationViewportEventHandler, IDirectManipulationInteractionEventHandler)]
pub(super) struct Handler(pub(super) Rc<RefCell<Gesture>>);

impl IDirectManipulationInteractionEventHandler_Impl for Handler_Impl {
    fn OnInteraction(&self, _viewport: Ref<IDirectManipulationViewport2>, interaction: DIRECTMANIPULATION_INTERACTION_TYPE) -> windows::core::Result<()> {
        aq_trace::counter("тачпад: касание", f64::from(interaction.0));
        match interaction {
            DIRECTMANIPULATION_INTERACTION_BEGIN => {
                let mut gesture = self.0.borrow_mut();
                gesture.touching = true;
                gesture.fling = None;
            }
            DIRECTMANIPULATION_INTERACTION_END => self.0.borrow_mut().touching = false,
            _ => {}
        }
        Ok(())
    }
}

impl IDirectManipulationViewportEventHandler_Impl for Handler_Impl {
    fn OnViewportStatusChanged(&self, viewport: Ref<IDirectManipulationViewport>, current: DIRECTMANIPULATION_STATUS, _previous: DIRECTMANIPULATION_STATUS) -> windows::core::Result<()> {
        aq_trace::counter("тачпад: состояние", f64::from(current.0));
        let moved = {
            let mut gesture = self.0.borrow_mut();
            gesture.running = current == DIRECTMANIPULATION_RUNNING || current == DIRECTMANIPULATION_INERTIA;
            if current == DIRECTMANIPULATION_RUNNING {
                gesture.inertia = false;
                gesture.fling = None;
                gesture.trail.clear();
                gesture.contact = gesture.last;
            }
            if current == DIRECTMANIPULATION_INERTIA {
                gesture.inertia = true;
                gesture.released = !gesture.pinching;
            }
            if current != DIRECTMANIPULATION_READY {
                return Ok(());
            }
            gesture.inertia = false;
            gesture.pinching = false;
            gesture.contact = (0.0, 0.0);
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
        aq_trace::counter("тачпад: смещение", f64::from(y));
        aq_trace::counter("тачпад: масштаб", f64::from(scale));
        if gesture.scale == 0.0 {
            gesture.scale = 1.0;
        }
        if gesture.pinch == 0.0 {
            gesture.pinch = 1.0;
        }
        if (scale - gesture.scale).abs() > 0.002 {
            gesture.pinch *= scale / gesture.scale;
            gesture.scale = scale;
            gesture.pinching = true;
        }
        if !gesture.pinching && !gesture.inertia {
            gesture.delta.0 += x - gesture.last.0;
            gesture.delta.1 += y - gesture.last.1;
            gesture.contact = (x, y);
            gesture.moved = true;
        }
        gesture.last = (x, y);
        Ok(())
    }
}
