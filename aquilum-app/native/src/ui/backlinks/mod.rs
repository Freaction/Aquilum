mod history;
mod list;
mod panel;
mod version;

use masonry::core::{NewWidget, PropertySet};
use masonry::layout::Length;
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::text::label;
use super::tokens::{number, size};
use super::widgets::{Label, Leading};

pub use history::{HistoryBody, HistoryRow, history_body};
pub use list::{Body, body};
pub use panel::{Panel, panel};
pub use version::{VersionView, version_view};

fn muted(text: &str) -> NewWidget<Label> {
    label(text, |t| t.text_tertiary, None, Leading::Px(size::SIZE_20 as f32))
}

fn notice(text: &str) -> NewWidget<Flex> {
    NewWidget::new(Flex::row().with(muted(text), 1.0)).with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_6))))
}

fn heading_label(text: &str) -> NewWidget<Label> {
    let line = (size::FONT_SIZE_14 * number::FONT_LINE_HEIGHT_TIGHT) as f32;
    label(text, |t| t.text_secondary, Some(500.0), Leading::Px(line))
}

fn heading_row(row: Flex) -> NewWidget<Flex> {
    NewWidget::new(row).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_8))).with(Padding {
        top: Length::px(size::SPACE_8),
        bottom: Length::px(size::SPACE_4),
        left: Length::px(size::SPACE_6),
        right: Length::px(size::SPACE_6),
    }))
}
