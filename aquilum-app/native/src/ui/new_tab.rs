use masonry::core::{NewWidget, PropertySet, WidgetId};
use masonry::layout::Length;
use masonry::properties::Gap;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::widgets::Flex;

use super::tokens::size;
use super::widgets::{TextButton, Variant};
use crate::i18n;

pub(super) fn new_tab_view() -> (NewWidget<Flex>, WidgetId, WidgetId, WidgetId) {
    let create = NewWidget::new(TextButton::new(i18n::t("editor.createNote"), Variant::Link));
    let open = NewWidget::new(TextButton::new(i18n::t("editor.openFile"), Variant::Link));
    let close = NewWidget::new(TextButton::new(i18n::t("editor.close"), Variant::Link));
    let (create_id, open_id, close_id) = (create.id(), open.id(), close.id());
    let column = Flex::column()
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(create)
        .with_fixed(open)
        .with_fixed(close);
    let column = NewWidget::new(column).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_6))));
    (column, create_id, open_id, close_id)
}
