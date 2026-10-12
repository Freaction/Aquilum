use std::collections::HashMap;

use masonry::core::{NewWidget, PropertySet, WidgetId};
use masonry::kurbo::Axis;
use masonry::layout::Length;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{BorderWidth, CornerRadius, Gap, Padding};
use masonry::widgets::{Flex, SizedBox};

use super::Binding;
use super::control::control_widget;
use super::schema::{Block, Row};
use crate::ui::scroll::ScrollArea;
use crate::ui::text::label;
use crate::ui::tokens::{number, size};
use crate::ui::widgets::{Font, Label, Leading, Rule, Scale};
use crate::ui::{style, theme};

pub(super) fn rows_column(rows: Vec<Row>, bindings: &mut HashMap<WidgetId, Binding>) -> Flex {
    let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    let count = rows.len();
    for (i, row) in rows.into_iter().enumerate() {
        column = column.with_fixed(row_widget(row, bindings));
        if i + 1 < count {
            column = column.with_fixed(Rule::new(Axis::Horizontal, || theme::current().border_default));
        }
    }
    column
}

pub(super) fn block_widget(block: Block, bindings: &mut HashMap<WidgetId, Binding>) -> NewWidget<Flex> {
    let title = NewWidget::new(Flex::row().with(label(&block.title, |t| t.text_secondary, Some(600.0), Leading::Px(size::SIZE_20 as f32)), 1.0))
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

fn row_widget(row: Row, bindings: &mut HashMap<WidgetId, Binding>) -> NewWidget<Flex> {
    let code = row.code.clone();
    let mut text = Flex::column()
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(label(&row.label, |t| t.text_secondary, None, Leading::Times(1.21)));
    if let Some(description) = &row.description {
        text = text.with_fixed(label(description, |t| t.text_muted, None, Leading::Times(1.21)));
    }
    let text = NewWidget::new(text).with_props(PropertySet::new().with(Gap::new(Length::px(size::GAP_SM))));
    let strut = NewWidget::new(SizedBox::empty().height(Length::px(size::SETTINGS_ROW_TEXT_MIN_HEIGHT)));
    let control = NewWidget::new(Flex::column().main_axis_alignment(MainAxisAlignment::Center).with_fixed(control_widget(&row, bindings)));
    let flex = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(strut)
        .with(text, 1.0)
        .with_fixed(control);
    let Some(code) = code else {
        return NewWidget::new(flex).with_props(
            PropertySet::new().with(Gap::new(Length::px(size::GAP_LG))).with(Padding::from_vh(Length::px(size::GAP_XL), Length::ZERO)),
        );
    };
    let flex = NewWidget::new(flex).with_props(PropertySet::new().with(Gap::new(Length::px(size::GAP_LG))).with(Padding {
        top: Length::px(size::GAP_XL),
        bottom: Length::px(size::GAP_LG),
        left: Length::ZERO,
        right: Length::ZERO,
    }));
    let column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(flex).with_fixed(code_block(&code));
    NewWidget::new(column).with_props(PropertySet::new().with(Padding {
        top: Length::ZERO,
        bottom: Length::px(size::GAP_XL),
        left: Length::ZERO,
        right: Length::ZERO,
    }))
}

fn code_block(code: &str) -> NewWidget<Flex> {
    let text = Label::new(code, |t| t.text_secondary)
        .font(Font::Mono)
        .scale(Scale::Small)
        .leading(Leading::Times(number::FONT_LINE_HEIGHT_NORMAL as f32))
        .nowrap();
    let text = NewWidget::new(text).with_props(PropertySet::new().with(Padding::all(Length::px(size::GAP_LG))));
    NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(NewWidget::new(ScrollArea::horizontal(text))))
        .with_props(PropertySet::new().with(BorderWidth { width: Length::px(size::BORDER_1) }).with(CornerRadius { radius: Length::px(size::BLOCK_RADIUS) }))
        .with_class(style::BLOCK)
}
