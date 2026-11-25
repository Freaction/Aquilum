//! Linux window identity, independent of CSD themes.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_identity_matches_the_packaged_desktop_entry() {
        let desktop = include_str!("../../packaging/aquilum-app.desktop");
        assert!(desktop.lines().any(|line| line == format!("StartupWMClass={APP_ID}")));
        assert!(desktop.lines().any(|line| line == format!("Icon={APP_ID}")));
        assert!(include_str!("../../packaging/nfpm.yaml").contains(&format!("/usr/share/applications/{APP_ID}.desktop")));
    }
}
