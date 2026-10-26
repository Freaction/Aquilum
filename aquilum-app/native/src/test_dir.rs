pub fn tempdir() -> tempfile::TempDir {
    let root = crate::workspace::normalize(&std::env::temp_dir());
    tempfile::Builder::new().tempdir_in(root).expect("временный каталог")
}

pub fn command() -> masonry::core::keyboard::Modifiers {
    if cfg!(target_os = "macos") { masonry::core::keyboard::Modifiers::META } else { masonry::core::keyboard::Modifiers::CONTROL }
}

pub fn word_modifier() -> masonry::core::keyboard::Modifiers {
    if cfg!(target_os = "macos") { masonry::core::keyboard::Modifiers::ALT } else { masonry::core::keyboard::Modifiers::CONTROL }
}

pub(crate) use crate::host::PRIMARY_MOUSE as MOUSE;

pub fn options(width: u32, height: u32, scale_factor: f64) -> masonry::app::RenderRootOptions {
    masonry::app::RenderRootOptions {
        default_properties: std::sync::Arc::new(masonry::theme::default_property_set()),
        use_system_fonts: true,
        size_policy: masonry::app::WindowSizePolicy::User,
        size: masonry::dpi::PhysicalSize::new(width, height),
        scale_factor,
        test_font: None,
    }
}

pub fn key(root: &mut masonry::app::RenderRoot, key: masonry::core::keyboard::Key, modifiers: masonry::core::keyboard::Modifiers) {
    use masonry::core::keyboard::{Code, KeyState, KeyboardEvent, Location};
    for state in [KeyState::Down, KeyState::Up] {
        let event = KeyboardEvent { state, key: key.clone(), code: Code::Unidentified, location: Location::Standard, modifiers, repeat: false, is_composing: false };
        root.handle_text_event(masonry::core::TextEvent::Keyboard(event));
    }
}

pub fn pointer_at(x: f64, y: f64, scale_factor: f64) -> masonry::core::PointerState {
    masonry::core::PointerState { position: masonry::dpi::PhysicalPosition::new(x, y), count: 1, scale_factor, ..Default::default() }
}

pub fn moved(current: masonry::core::PointerState) -> masonry::core::PointerEvent {
    masonry::core::PointerEvent::Move(masonry::core::PointerUpdate { pointer: MOUSE, current, coalesced: vec![], predicted: vec![] })
}

pub fn primary(state: masonry::core::PointerState) -> masonry::core::PointerButtonEvent {
    masonry::core::PointerButtonEvent { pointer: MOUSE, button: Some(masonry::core::PointerButton::Primary), state }
}
