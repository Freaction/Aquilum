use std::collections::HashMap;

use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::layout::Length;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{BorderWidth, CornerRadius, Gap, Padding};
use masonry::widgets::Flex;

use super::Binding;
use super::control::{LEADING, control_widget, status};
use super::row::SettingRow;
use super::schema::{Block, Control, Row};
use crate::ui::scroll::ScrollArea;
use crate::ui::text::selectable;
use crate::ui::tokens::{number, size};
use crate::ui::widgets::{Countdown, Font, Label, Leading, Rule, Scale};
use crate::ui::style;

pub(super) fn rows_column(rows: Vec<Row>, bindings: &mut HashMap<WidgetId, Binding>) -> Flex {
    let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    let count = rows.len();
    for (i, row) in rows.into_iter().enumerate() {
        column = column.with_fixed(row_widget(row, bindings));
        if i + 1 < count {
            column = column.with_fixed(Rule::divider());
        }
    }
    column
}

pub(super) fn block_widget(block: Block, bindings: &mut HashMap<WidgetId, Binding>) -> NewWidget<Flex> {
    let title = NewWidget::new(Flex::row().with(selectable(&block.title, |t| t.text_secondary, Some(600.0), Leading::Px(size::SIZE_20 as f32)), 1.0))
        .with_props(PropertySet::new().with(Padding {
            top: Length::px(size::GAP_XL),
            bottom: Length::px(size::GAP_LG),
            left: Length::px(size::GAP_LG),
            right: Length::px(size::GAP_LG),
        }));
    let card = NewWidget::new(rows_column(block.rows, bindings)).with_class(style::CARD).with_props(
        PropertySet::new()
            .with(CornerRadius { radius: Length::px(size::RADIUS_2XL) })
            .with(Padding { top: Length::ZERO, bottom: Length::ZERO, left: Length::px(size::GAP_XL), right: Length::px(size::GAP_XL) }),
    );
    NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(title).with_fixed(card))
}

fn row_widget(row: Row, bindings: &mut HashMap<WidgetId, Binding>) -> NewWidget<dyn Widget> {
    let code = row.code.clone();
    let mut text = Flex::column()
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(selectable(&row.label, |t| t.text_secondary, None, LEADING));
    if let Some(description) = &row.description {
        text = text.with_fixed(selectable(description, |t| t.text_muted, None, LEADING));
    }
    if let Some((at, format)) = row.countdown {
        text = text.with_fixed(Countdown::new(at, format, |t| t.text_muted, LEADING));
    }
    if let Control::Status { text: line, tone } = &row.control {
        text = text.with_fixed(status(line, *tone));
    }
    let text = NewWidget::new(text).with_props(PropertySet::new().with(Gap::new(Length::px(size::GAP_SM))));
    let line = SettingRow::new(text, control_widget(&row, bindings));
    let Some(code) = code else {
        return line.with_props(PropertySet::new().with(Padding::from_vh(Length::px(size::GAP_XL), Length::ZERO))).erased();
    };
    let line = line.with_props(PropertySet::new().with(Padding {
        top: Length::px(size::GAP_XL),
        bottom: Length::px(size::GAP_LG),
        left: Length::ZERO,
        right: Length::ZERO,
    }));
    let column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(line).with_fixed(code_block(&code));
    NewWidget::new(column)
        .with_props(PropertySet::new().with(Padding {
            top: Length::ZERO,
            bottom: Length::px(size::GAP_XL),
            left: Length::ZERO,
            right: Length::ZERO,
        }))
        .erased()
}

fn code_block(code: &str) -> NewWidget<Flex> {
    let text = Label::new(code, |t| t.text_secondary)
        .selectable()
        .font(Font::Mono)
        .scale(Scale::Small)
        .leading(Leading::Times(number::FONT_LINE_HEIGHT_NORMAL as f32))
        .nowrap();
    let text = NewWidget::new(text).with_props(PropertySet::new().with(Padding::all(Length::px(size::GAP_LG))));
    NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(NewWidget::new(ScrollArea::horizontal(text))))
        .with_props(PropertySet::new().with(BorderWidth { width: Length::px(size::BORDER_1) }).with(CornerRadius { radius: Length::px(size::BLOCK_RADIUS) }))
        .with_class(style::BLOCK)
}
