use masonry::accesskit::Role;
use masonry::core::{ErasedAction, NewWidget, PropertySet, WidgetId};
use masonry::kurbo::Axis;
use masonry::layout::Length;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::{CardSize, DialogAnswer, Modal, header};
use crate::ui::text::selectable;
use crate::ui::theme;
use crate::ui::tokens::size;
use crate::ui::widgets::{Leading, Pressed, Rule, TextButton, Variant};

pub struct ConfirmDialog;

impl ConfirmDialog {
    pub fn new(title: &str, description: &str, cancel_label: &str, confirm_label: &str) -> (NewWidget<Modal>, DialogIds) {
        let label = |text: &str, paint| selectable(text, paint, None, Leading::Px(size::SIZE_20 as f32));
        let header = header(title, None::<NewWidget<Flex>>);
        let content = NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(label(description, |t| t.text_secondary)))
            .with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_24))));
        let cancel = NewWidget::new(TextButton::new(cancel_label, Variant::Ghost));
        let confirm = NewWidget::new(TextButton::new(confirm_label, Variant::Primary));
        let (cancel_id, confirm_id) = (cancel.id(), confirm.id());
        let footer = NewWidget::new(
            Flex::row().main_axis_alignment(MainAxisAlignment::End).with_fixed(cancel).with_fixed(confirm),
        )
        .with_props(
            PropertySet::new()
                .with(Gap::new(Length::px(size::DIALOG_GAP)))
                .with(Padding::all(Length::px(size::DIALOG_PADDING))),
        );
        let border = || Rule::new(Axis::Horizontal, || theme::current().border_default);
        let card = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(header)
            .with_fixed(border())
            .with_fixed(content)
            .with_fixed(border())
            .with_fixed(footer);
        let dialog = Modal::new(NewWidget::new(card), title, CardSize::Fit(size::CONFIRM_DIALOG_WIDTH), Role::AlertDialog, true);
        let ids = DialogIds::of(&dialog, (cancel_id, confirm_id));
        (dialog, ids)
    }
}

pub struct DialogIds {
    pub layer: WidgetId,
    cancel: WidgetId,
    confirm: WidgetId,
}

impl DialogIds {
    pub fn of(dialog: &NewWidget<Modal>, inner: (WidgetId, WidgetId)) -> Self {
        DialogIds { layer: dialog.id(), cancel: inner.0, confirm: inner.1 }
    }

    pub fn answer(&self, id: WidgetId, action: &ErasedAction) -> Option<DialogAnswer> {
        if id == self.layer {
            return action.downcast_ref::<DialogAnswer>().copied().filter(|answer| *answer != DialogAnswer::Closed);
        }
        action.downcast_ref::<Pressed>()?;
        if id == self.confirm {
            Some(DialogAnswer::Confirm)
        } else if id == self.cancel {
            Some(DialogAnswer::Cancel)
        } else {
            None
        }
    }
}
