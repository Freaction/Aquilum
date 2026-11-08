//! Linux window identity and event-driven desktop appearance, independent of CSD themes.
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use ashpd::desktop::settings::{ColorScheme, Settings};
use futures_lite::{StreamExt, future::block_on};
use winit::event_loop::ActiveEventLoop;
use winit::platform::wayland::{ActiveEventLoopExtWayland, WindowAttributesWayland};
use winit::platform::x11::WindowAttributesX11;
use winit::window::WindowAttributes;

const APP_ID: &str = "aquilum-app";

pub(super) fn attributes(attributes: WindowAttributes, event_loop: &dyn ActiveEventLoop) -> WindowAttributes {
    if event_loop.is_wayland() {
        attributes.with_platform_attributes(Box::new(WindowAttributesWayland::default().with_name(APP_ID, APP_ID)))
    } else {
        attributes.with_platform_attributes(Box::new(WindowAttributesX11::default().with_name(APP_ID, APP_ID)))
    }
}

pub(super) struct Theme {
    current: Arc<AtomicU8>,
    taken: Option<bool>,
}

impl Theme {
    pub(super) fn new(wake: impl Fn() + Send + 'static) -> Self {
        let current = Arc::new(AtomicU8::new(0));
        let shared = Arc::clone(&current);
        std::thread::Builder::new().name("desktop appearance".into()).spawn(move || {
            let result = block_on(async {
                let settings = Settings::new().await?;
                // Subscribe before the initial read so startup cannot miss a change.
                let changes = settings.receive_color_scheme_changed().await?;
                let mut changes = std::pin::pin!(changes);
                publish(&shared, settings.color_scheme().await.unwrap_or_default(), &wake);
                while let Some(scheme) = changes.next().await {
                    publish(&shared, scheme, &wake);
                }
                Ok::<_, ashpd::Error>(())
            });
            if let Err(error) = result { super::report(&format!("desktop appearance: {error}")); }
            publish(&shared, ColorScheme::NoPreference, &wake);
        }).expect("desktop appearance thread");
        Theme { current, taken: None }
    }

    pub(super) fn current(&self) -> Option<bool> {
        match self.current.load(Ordering::Acquire) { 1 => Some(false), 2 => Some(true), _ => None }
    }

    pub(super) fn take(&mut self) -> Option<bool> {
        let current = self.current();
        if current == self.taken { return None; }
        self.taken = current;
        current
    }
}

fn publish(current: &AtomicU8, scheme: ColorScheme, wake: &impl Fn()) {
    let value = if scheme == ColorScheme::PreferDark { 2 } else { 1 };
    if current.swap(value, Ordering::AcqRel) != value { wake(); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_changes_are_coalesced_and_no_preference_is_light() {
        let mut theme = Theme { current: Arc::new(AtomicU8::new(0)), taken: None };
        let wakes = std::cell::Cell::new(0);
        let wake = || wakes.set(wakes.get() + 1);
        assert_eq!(theme.take(), None);
        publish(&theme.current, ColorScheme::PreferDark, &wake);
        assert_eq!(theme.current(), Some(true));
        assert_eq!(theme.take(), Some(true));
        publish(&theme.current, ColorScheme::PreferDark, &wake);
        assert_eq!(theme.take(), None);
        assert_eq!(wakes.get(), 1);
        publish(&theme.current, ColorScheme::PreferLight, &wake);
        publish(&theme.current, ColorScheme::PreferDark, &wake);
        assert_eq!(theme.take(), None, "intermediate changes must not restyle the UI");
        publish(&theme.current, ColorScheme::NoPreference, &wake);
        assert_eq!(theme.take(), Some(false));
    }

    #[test]
    fn window_identity_matches_the_packaged_desktop_entry() {
        let desktop = include_str!("../../packaging/aquilum-app.desktop");
        assert!(desktop.lines().any(|line| line == format!("StartupWMClass={APP_ID}")));
        assert!(desktop.lines().any(|line| line == format!("Icon={APP_ID}")));
        assert!(include_str!("../../packaging/nfpm.yaml").contains(&format!("/usr/share/applications/{APP_ID}.desktop")));
    }
}
