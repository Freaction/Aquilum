use std::path::Path;

use masonry::core::{NewWidget, PropertySet, WidgetId};
use masonry::layout::Length;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Gap, Padding};
use masonry::widgets::{Flex, SizedBox};

use crate::i18n::{t, t_with};
use crate::ui::icons;
use crate::ui::tokens::size;
use crate::ui::widgets::{ButtonSize, IconButton, TextButton, Variant};

pub struct SaveErrorIds {
    pub retry: WidgetId,
}

pub struct OpenErrorIds {
    pub retry: WidgetId,
    pub reveal: WidgetId,
    pub close: WidgetId,
}

/// Баннер об ошибке сохранения, размещаемый над заголовком открытой заметки.
pub fn save_error_banner(text: &str) -> (NewWidget<Flex>, SaveErrorIds) {
    let retry = NewWidget::new(TextButton::new(t("editor.saveRetry"), Variant::Primary).size(ButtonSize::Xs));
    let retry_id = retry.id();

    let label = NewWidget::new(super::column::Caption::new(text));
    let mut row = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .main_axis_alignment(MainAxisAlignment::SpaceBetween);
    row = row.with(label, 1.0).with_fixed(retry);

    let banner = NewWidget::new(row)
        .with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_8))).with(Gap::new(Length::px(size::SPACE_8))))
        .with_class(crate::ui::style::DANGER);

    (banner, SaveErrorIds { retry: retry_id })
}

/// Полноразмерный экран ошибки, если заметку не удалось прочитать с диска.
pub fn open_error_page(path: &Path, error: &str) -> (NewWidget<Flex>, OpenErrorIds) {
    let file_str = path.to_string_lossy();
    let desc = t_with("editor.openFailed", &[("path", &file_str), ("reason", error)]);

    let retry = NewWidget::new(TextButton::new(t("error.retry"), Variant::Primary).size(ButtonSize::S));
    let reveal = NewWidget::new(TextButton::new(t("editor.revealFile"), Variant::Ghost).size(ButtonSize::S));
    let close = NewWidget::new(TextButton::new(t("editor.close"), Variant::Ghost).size(ButtonSize::S));

    let (retry_id, reveal_id, close_id) = (retry.id(), reveal.id(), close.id());

    let actions = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .main_axis_alignment(MainAxisAlignment::Center)
        .with_fixed(retry)
        .with_fixed(reveal)
        .with_fixed(close);
    let actions = NewWidget::new(actions).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_8))));

    let icon = NewWidget::new(IconButton::new(icons::FILE_X, t("editor.openFailedTitle"))).disabled(true);
    let title = NewWidget::new(super::column::Caption::new(&t("editor.openFailedTitle")));
    let desc_w = NewWidget::new(super::column::Caption::new(&desc));

    let content = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .main_axis_alignment(MainAxisAlignment::Center)
        .with_fixed(icon)
        .with_fixed(NewWidget::new(SizedBox::empty().height(Length::px(size::SPACE_12))))
        .with_fixed(title)
        .with_fixed(NewWidget::new(SizedBox::empty().height(Length::px(size::SPACE_8))))
        .with_fixed(desc_w)
        .with_fixed(NewWidget::new(SizedBox::empty().height(Length::px(size::SPACE_20))))
        .with_fixed(actions);

    let page = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .main_axis_alignment(MainAxisAlignment::Center)
        .with(NewWidget::new(content), 1.0);

    let page = NewWidget::new(page).with_class(crate::ui::style::EDITOR);

    (page, OpenErrorIds { retry: retry_id, reveal: reveal_id, close: close_id })
}
