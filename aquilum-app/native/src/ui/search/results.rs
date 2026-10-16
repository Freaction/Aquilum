use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use crate::ui::keymap::{self, Command, Scope};
use masonry::kurbo::{Axis, Size};
use masonry::layout::Length;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::card::{CardKind, ResultCard};
use crate::i18n::t;
use crate::ui::icons::Icon;
use crate::ui::theme;
use crate::ui::tokens::{number, size};
use crate::ui::widgets::{IconView, Label, Leading, MaxWidth, Paint, Rule, Scale, Slot, kbd};

pub fn empty_state(icon: Icon, title: &str, description: &str) -> NewWidget<Flex> {
    empty_state_with(icon, title, Some(description), None)
}

pub fn empty_state_with(icon: Icon, title: &str, description: Option<&str>, action: Option<NewWidget<dyn Widget>>) -> NewWidget<Flex> {
    let mut copy = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(centered(title, |t| t.text_secondary));
    if let Some(description) = description {
        copy = copy.with_fixed(centered(description, |t| t.text_tertiary));
    }
    let copy = NewWidget::new(copy).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_8))));
    let mut column = Flex::column()
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(IconView::new(icon, size::EMPTY_STATE_ICON_CONTAINER, size::EMPTY_STATE_ICON_SIZE, 1.4, || theme::current().icon_muted))
        .with_fixed(MaxWidth::new(copy, size::EMPTY_STATE_WIDTH));
    if let Some(action) = action {
        column = column.with_fixed(action);
    }
    NewWidget::new(column).with_props(
        PropertySet::new()
            .with(Gap::new(Length::px(size::EMPTY_STATE_GAP)))
            .with(Padding::from_vh(Length::px(size::EMPTY_STATE_PADDING_Y), Length::ZERO)),
    )
}

fn centered(text: &str, paint: Paint) -> NewWidget<Label> {
    NewWidget::new(Label::new(text, paint).leading(Leading::Times(number::FONT_LINE_HEIGHT_SNUG as f32)).centered())
}

fn hint(keys: &str, text: &str) -> NewWidget<Flex> {
    let label = Label::new(text, |t| t.text_tertiary).scale(Scale::Small).nowrap();
    let row = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(kbd(keys, size::SPACE_2)).with_fixed(NewWidget::new(label));
    NewWidget::new(row).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_4))))
}

fn hints(list: &[(String, String)]) -> NewWidget<Flex> {
    let mut row = Flex::row().main_axis_alignment(MainAxisAlignment::Center).cross_axis_alignment(CrossAxisAlignment::Center);
    for (keys, text) in list {
        row = row.with_fixed(hint(keys, text));
    }
    NewWidget::new(row).with_props(
        PropertySet::new().with(Gap::new(Length::px(size::SPACE_12))).with(Padding::all(Length::px(size::DIALOG_PADDING))),
    )
}

fn key(command: Command) -> String {
    keymap::label(Scope::List, command)
}

fn arrows() -> String {
    format!("{} {}", key(Command::ListPrevious), key(Command::ListNext))
}

pub fn footer() -> NewWidget<Flex> {
    hints(&[
        (arrows(), t("search.navigate")),
        (key(Command::ListOpen), t("search.open")),
        (key(Command::ListOpenInNewTab), t("search.newPane")),
        (key(Command::ListCreate), t("search.create")),
    ])
}

pub fn template_footer() -> NewWidget<Flex> {
    hints(&[(arrows(), t("search.navigate")), (key(Command::ListOpen), t("search.open")), (key(Command::Close), t("templateDialog.close"))])
}

pub fn dialog_card(head: NewWidget<Flex>, body: NewWidget<Slot>, footer: NewWidget<Flex>) -> NewWidget<Flex> {
    let rule = || Rule::new(Axis::Horizontal, || theme::current().border_default);
    NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(head).with_fixed(rule()).with(body, 1.0).with_fixed(rule()).with_fixed(footer))
}

pub fn card_size(room: Size) -> Size {
    let window_height = room.height + size::SPACE_32;
    Size::new(room.width.min(size::SEARCH_WIDTH), (window_height * 0.7).min(room.height))
}

pub fn results_list(cards: Vec<NewWidget<ResultCard>>) -> (NewWidget<Flex>, Vec<WidgetId>) {
    let ids = cards.iter().map(NewWidget::id).collect();
    let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    for card in cards {
        column = column.with_fixed(card);
    }
    let column = NewWidget::new(column).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_6))).with(Padding {
        top: Length::px(size::SPACE_8),
        bottom: Length::px(size::SPACE_8),
        left: Length::px(size::SPACE_8),
        right: Length::px(size::SPACE_4),
    }));
    (column, ids)
}

pub fn offset_of(kind: CardKind, index: usize) -> (f64, f64) {
    let top = size::SPACE_8 + index as f64 * (kind.height() + size::SPACE_6);
    (top, top + kind.height())
}
