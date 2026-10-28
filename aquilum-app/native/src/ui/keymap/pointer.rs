use masonry::core::keyboard::Modifiers;
use masonry::core::{CursorIcon, PointerButton, PointerType};

use super::gesture::{DRAG_SLOP, PAGE_ZONE, PAGE_ZONE_MAX, PAGE_ZONE_MIN, TOUCH_EDGE, TOUCH_REACH, TOUCH_SLOP};

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

pub fn drags(cursor: CursorIcon) -> bool {
    use CursorIcon::*;
    matches!(cursor, Grab | Grabbing | Move | ColResize | RowResize | EwResize | NsResize | NwseResize | NeswResize)
}

pub fn thumb(cursor: CursorIcon) -> bool {
    matches!(cursor, CursorIcon::Grab | CursorIcon::Grabbing)
}

pub fn selects(cursor: CursorIcon) -> bool {
    cursor == CursorIcon::Text
}

pub fn edge(pointer: PointerType) -> f64 {
    if pointer == PointerType::Touch { TOUCH_EDGE } else { 0.0 }
}

pub fn page_zone(pointer: PointerType, width: f64) -> f64 {
    if pointer == PointerType::Touch { width / 3.0 } else { (width * PAGE_ZONE).clamp(PAGE_ZONE_MIN, PAGE_ZONE_MAX) }
}

pub fn pages_track(pointer: PointerType) -> bool {
    pointer != PointerType::Touch
}

pub fn reach(pointer: PointerType) -> f64 {
    if pointer == PointerType::Touch { TOUCH_REACH } else { 0.0 }
}
