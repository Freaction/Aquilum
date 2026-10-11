//! Панель поиска по заметке (`PageSearchBar.tsx`): поле со счётчиком и кнопки переходов.

use masonry::core::{NewWidget, Widget, WidgetId};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;
use masonry::core::PropertySet;

use super::editor::TextEditor;
use super::handle::Handle;
use super::icons;
use super::search::{self, Placeholder};
use super::settings::controls::InputBox;
use super::theme;
use super::tokens::size;
use super::widgets::{IconButton, IconView, Label};
use crate::i18n::t;

pub struct FindBar {
    pub field: Handle<TextEditor>,
    pub placeholder: Handle<Placeholder>,
    pub counter: Handle<Label>,
    pub prev: WidgetId,
    pub next: WidgetId,
    pub all: WidgetId,
    pub close: WidgetId,
}

pub fn find_bar() -> (NewWidget<dyn Widget>, FindBar) {
    let (placeholder, field, placeholder_handle) = search::field("", t("editor.find"));
    let counter = NewWidget::new(Label::new("0/0", |t| t.text_muted).nowrap());
    let counter_handle = Handle::of(&counter);
    let input = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(IconView::new(icons::SEARCH, size::SIZE_16, size::SIZE_16, 1.4, || theme::current().icon_secondary))
        .with(placeholder, 1.0)
        .with_fixed(counter);
    let input = InputBox::new(NewWidget::new(input).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_6)))), size::PAGE_SEARCH_MAX_WIDTH);
    let button = |icon, label: &str| NewWidget::new(IconButton::new(icon, t(label)));
    let (prev, next, all, close) =
        (button(icons::ARROW_UP, "editor.prevMatch"), button(icons::ARROW_DOWN, "editor.nextMatch"), button(icons::TEXT_SELECT, "editor.selectAllMatches"), button(icons::X, "editor.closeSearch"));
    let ids = FindBar { field, placeholder: placeholder_handle, counter: counter_handle, prev: prev.id(), next: next.id(), all: all.id(), close: close.id() };
    let actions = Flex::row().with_fixed(prev).with_fixed(next).with_fixed(all).with_fixed(close);
    let actions = NewWidget::new(actions).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_4))));
    let group = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with(input, 1.0).with_fixed(actions);
    let group = NewWidget::new(group).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_8))));
    let widest = size::PAGE_SEARCH_MAX_WIDTH + size::SPACE_8 + size::SIZE_32 * 4.0 + size::SPACE_4 * 3.0;
    let bar = NewWidget::new(Flex::row().with(super::widgets::MaxWidth::new(group, widest), 1.0))
        .with_props(PropertySet::new().with(Padding::from_vh(Length::px(size::SPACE_4), Length::px(size::SPACE_8))))
        .with_class(super::style::EDITOR);
    (bar.erased(), ids)
}
