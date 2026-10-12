use masonry::core::NewWidget;
use masonry::kurbo::Axis;
use masonry::peniko::Color;

use super::theme;
use super::widgets::Rule;

pub(super) fn border(axis: Axis, base: fn() -> Color) -> NewWidget<Rule> {
    Rule::on(axis, || theme::current().sidebar_border, base)
}

pub(super) fn on_header() -> Color {
    theme::current().tabs_bg
}

pub(super) fn on_sidebar() -> Color {
    theme::current().sidebar_bg
}
