use masonry::core::keyboard::Modifiers;
use masonry::core::{PointerButton, PointerType};

use super::gesture::{DRAG_SLOP, TOUCH_SLOP};

pub fn press(button: Option<PointerButton>) -> bool {
    matches!(button, None | Some(PointerButton::Primary))
}

pub fn context(button: Option<PointerButton>) -> bool {
    button == Some(PointerButton::Secondary)
}

pub fn close(button: Option<PointerButton>) -> bool {
    button == Some(PointerButton::Auxiliary)
}

pub fn new_tab(modifiers: Modifiers) -> bool {
    super::action(modifiers)
}

pub fn toggle(modifiers: Modifiers) -> bool {
    super::action(modifiers)
}

pub fn add_cursor(modifiers: Modifiers) -> bool {
    modifiers.alt()
}

pub fn zoom(modifiers: Modifiers) -> bool {
    modifiers.ctrl() || modifiers.meta()
}

pub fn sideways(modifiers: Modifiers) -> bool {
    modifiers.shift()
}

pub fn drag_slop(pointer: PointerType) -> f64 {
    if pointer == PointerType::Touch { TOUCH_SLOP } else { DRAG_SLOP }
}
