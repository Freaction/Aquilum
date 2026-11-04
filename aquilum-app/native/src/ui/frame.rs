use masonry::peniko::Color;

use super::theme;

pub(super) fn on_header() -> Color {
    theme::current().tabs_bg
}

pub(super) fn on_sidebar() -> Color {
    theme::current().sidebar_bg
}
