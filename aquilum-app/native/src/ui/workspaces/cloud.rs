use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::{Flex, SizedBox};

use super::WorkspaceRow;
use crate::i18n::{t, t_with};
use crate::ui::text::{label, selectable};
use crate::ui::tokens::size;
use crate::ui::widgets::Leading;

pub enum Cloud<'a> {
    Loading,
    Failed(&'a str),
    Vaults(&'a [String]),
}

pub fn section(cloud: Cloud<'_>) -> (NewWidget<dyn Widget>, Vec<(WidgetId, String)>) {
    let mut rows = Vec::new();
    if matches!(cloud, Cloud::Vaults([])) {
        return (NewWidget::new(SizedBox::empty()).erased(), rows);
    }
    let leading = Leading::Px(size::SIZE_20 as f32);
    let mut column = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(label(&t("workspaces.cloud"), |t| t.text_secondary, Some(600.0), leading));
    match cloud {
        Cloud::Loading => column = column.with_fixed(label(&t("workspaces.cloudLoading"), |t| t.text_tertiary, None, leading)),
        Cloud::Failed(reason) => {
            column = column.with_fixed(selectable(&t_with("workspaces.cloudFailed", &[("reason", reason)]), |t| t.text_danger, None, leading));
        }
        Cloud::Vaults(names) => {
            for name in names {
                let row = WorkspaceRow::cloud(name);
                rows.push((row.id(), name.clone()));
                column = column.with_fixed(row);
            }
        }
    }
    let props = PropertySet::new().with(Gap::new(Length::px(size::SPACE_4))).with(Padding::top(Length::px(size::SPACE_8)));
    (NewWidget::new(column).with_props(props).erased(), rows)
}
