use masonry::core::NewWidget;
use masonry::layout::Length;
use masonry::properties::Gap;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::Flex;
use masonry::core::PropertySet;

use super::{IconView, Label, Scale};
use crate::ui::icons::{self, Icon};
use crate::ui::theme;
use crate::ui::tokens::size;

const ICON_STROKE_GRID: f64 = 2.2;
const ICON_STROKE: f64 = ICON_STROKE_GRID * size::FONT_SIZE_12 / 24.0;

fn icon(token: &str) -> Option<Icon> {
    Some(match token {
        "↵" => icons::CORNER_DOWN_LEFT,
        "↑" => icons::ARROW_UP,
        "↓" => icons::ARROW_DOWN,
        "←" => icons::ARROW_LEFT,
        "→" => icons::ARROW_RIGHT,
        "↕" => icons::ARROW_UP_DOWN,
        _ => return None,
    })
}

pub fn kbd(keys: &str, gap: f64) -> NewWidget<Flex> {
    let mut row = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
    for token in keys.split_whitespace() {
        row = match icon(token) {
            Some(icon) => row.with_fixed(IconView::new(icon, size::FONT_SIZE_12, size::FONT_SIZE_12, ICON_STROKE, || theme::current().search_key_hint_color)),
            None => row.with_fixed(NewWidget::new(Label::new(token, |t| t.search_key_hint_color).scale(Scale::Small).weight(500.0).nowrap())),
        };
    }
    NewWidget::new(row).with_props(PropertySet::new().with(Gap::new(Length::px(gap))))
}

#[cfg(test)]
mod tests {
    #[test]
    fn arrows_and_enter_are_icons() {
        assert!(["↵", "↑", "↓", "←", "→", "↕"].iter().all(|t| super::icon(t).is_some()));
        assert!(["Ctrl", "Shift", "+", "Tab", "B"].iter().all(|t| super::icon(t).is_none()));
    }
}
