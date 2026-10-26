use masonry::kurbo::Insets;

use super::{Interface, Side};
use crate::ui::tokens::size;

const MEDIUM: f64 = 600.0;
const EXPANDED: f64 = 840.0;
const DRAWER_MAX: f64 = 360.0;
const DRAWER_GAP: f64 = 56.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidthClass {
    Compact,
    Medium,
    Expanded,
}

impl WidthClass {
    pub fn of(width: f64) -> Self {
        if width < MEDIUM {
            WidthClass::Compact
        } else if width < EXPANDED {
            WidthClass::Medium
        } else {
            WidthClass::Expanded
        }
    }
}

pub fn drawer_width(room: f64, side: Side) -> f64 {
    let whole = (room - DRAWER_GAP).min(DRAWER_MAX);
    let rail = if side == Side::Sidebar { size::SIDEBAR_RAIL_WIDTH } else { 0.0 };
    (whole - rail).max(0.0)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Screen {
    pub compact: bool,
    pub insets: Insets,
    pub drawer: Option<Side>,
}

impl Interface {
    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn set_screen(&mut self, screen: Screen) {
        self.screen = screen;
    }
}
