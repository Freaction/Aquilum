use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::kurbo::Axis;
use masonry::layout::Length;
use masonry::properties::Padding;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::Flex;

use super::muted;
use crate::history::Change;
use crate::i18n::t;
use crate::ui::scroll::ScrollArea;
use crate::ui::tokens::size;
use crate::ui::widgets::{Font, Label, Leading, MaxWidth, Rule, TextButton, Variant, Wrap};
use crate::ui::{style, theme};

pub struct VersionView {
    pub revert: Option<WidgetId>,
    pub restore: WidgetId,
    pub back: WidgetId,
}

pub fn version_view(heading: &str, can_revert: bool, can_restore: bool, lines: &[(Change, String)], notice: Option<&str>) -> (NewWidget<dyn Widget>, VersionView) {
    let mut buttons: Vec<NewWidget<dyn Widget>> = Vec::new();
    let mut revert = None;
    if can_revert {
        let button = NewWidget::new(TextButton::new(t("history.revert"), Variant::Link));
        revert = Some(button.id());
        buttons.push(button.erased());
    }
    let restore = NewWidget::new(TextButton::new(t("history.restore"), Variant::Link)).disabled(!can_restore);
    let back = NewWidget::new(TextButton::new(t("history.back"), Variant::Link));
    let (restore_id, back_id) = (restore.id(), back.id());
    buttons.push(restore.erased());
    buttons.push(back.erased());
    let actions = Wrap::new(buttons, 0.0);
    let title = NewWidget::new(Label::new(heading, |t| t.text_secondary).nowrap());
    let title = NewWidget::new(Flex::row().with(title, 1.0)).with_props(PropertySet::new().with(Padding::from_vh(Length::ZERO, Length::px(size::SPACE_12))));
    let bar = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(title).with_fixed(actions);
    let bar = NewWidget::new(bar).with_props(PropertySet::new().with(Padding {
        top: Length::px(size::SPACE_8),
        bottom: Length::px(size::SPACE_2),
        left: Length::px(32.0 - size::SPACE_12),
        right: Length::px(32.0 - size::SPACE_12),
    }));
    let mut text = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    if let Some(notice) = notice {
        text = text.with_fixed(muted(notice));
    }
    for (change, line) in lines {
        let shown = if line.is_empty() { " " } else { line.as_str() };
        let widget = Label::new(shown, |t| t.editor_text).font(Font::Editor).leading(Leading::Editor);
        let widget = match change {
            Change::Added => widget.background(|t| t.bg_success_subtle),
            Change::Removed => widget.background(|t| t.bg_danger_subtle),
            Change::Same => widget,
        };
        text = text.with_fixed(NewWidget::new(widget));
    }
    let column = MaxWidth::new(NewWidget::new(text), 760.0);
    let content = NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(column))
        .with_props(PropertySet::new().with(Padding::all(Length::px(32.0))));
    let view = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(bar)
        .with_fixed(Rule::new(Axis::Horizontal, || theme::current().sidebar_border))
        .with(NewWidget::new(ScrollArea::new(content)), 1.0);
    let view = NewWidget::new(view).with_class(style::EDITOR);
    (view.erased(), VersionView { revert, restore: restore_id, back: back_id })
}
