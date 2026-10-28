mod confirm;
mod modal;
mod widget;

use masonry::accesskit::Role;
use masonry::core::{NewWidget, PropertySet, Widget, WidgetId, WidgetPod};
use masonry::kurbo::{Rect, Size};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::text::label;
use super::tokens::size;
use super::widgets::{Intro, Leading, Rule};

pub use confirm::{ConfirmDialog, DialogIds};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DialogAnswer {
    Confirm,
    Cancel,
    Closed,
}

#[derive(Clone, Copy)]
pub enum CardSize {
    Fit(f64),
    Window(fn(Size) -> Size),
    Anchored(fn(Size) -> Size),
}

pub struct Modal {
    card: WidgetPod<Flex>,
    title: String,
    size: CardSize,
    role: Role,
    enter_confirms: bool,
    card_rect: Rect,
    intro: Intro,
    closing: bool,
    id: Option<u64>,
}

pub fn header(title: &str, close: Option<NewWidget<impl Widget>>) -> NewWidget<Flex> {
    let title = label(title, |t| t.text_secondary, None, Leading::Px(size::SIZE_20 as f32));
    let mut row = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with(title, 1.0);
    if let Some(close) = close {
        row = row.with_fixed(close);
    }
    NewWidget::new(row).with_props(
        PropertySet::new().with(Gap::new(Length::px(size::GAP_XL))).with(Padding::all(Length::px(size::DIALOG_PADDING))),
    )
}

pub fn ruled(head: NewWidget<impl Widget + ?Sized>) -> Flex {
    Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(head).with_fixed(Rule::divider())
}

pub fn card(head: NewWidget<impl Widget + ?Sized>, body: NewWidget<impl Widget + ?Sized>, footer: NewWidget<impl Widget + ?Sized>) -> NewWidget<Flex> {
    NewWidget::new(ruled(head).with(body, 1.0).with_fixed(Rule::divider()).with_fixed(footer))
}

pub fn close(root: &mut masonry::app::RenderRoot, layer: WidgetId) {
    root.edit_widget(layer, |mut widget| Modal::close(&mut widget.downcast()));
}
