mod cloud;
mod row;

use masonry::accesskit::Role;
use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::kurbo::Size;
use masonry::layout::Length;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::dialog::{CardSize, Modal, card, header};
use super::editor::TextEditor;
use super::handle::Handle;
use super::scroll::ScrollArea;
use super::text::selectable;
use super::tokens::size;
use super::widgets::{IconButton, Leading, Slot, TextButton, Variant};
use super::{icons, search};
use crate::i18n::t;

pub use cloud::{Cloud, section as cloud_section};
pub use row::{RowAction, WorkspaceRow};

pub struct WorkspaceDialog {
    pub layer: WidgetId,
    pub close: WidgetId,
    pub buttons: Buttons,
    pub list: Handle<Slot>,
    pub cloud: Option<Handle<Slot>>,
}

pub struct Buttons {
    pub create: WidgetId,
    pub open: Option<WidgetId>,
}

pub struct Rows {
    pub ids: Vec<WidgetId>,
    pub rename: Option<Handle<TextEditor>>,
}

pub fn dialog(list: NewWidget<dyn Widget>, cloud: bool) -> (NewWidget<Modal>, WorkspaceDialog) {
    let close = NewWidget::new(IconButton::new(icons::X, t("workspaces.close")));
    let close_id = close.id();
    let list = Slot::fit(list);
    let list_slot = Handle::of(&list);
    let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(list);
    let cloud = cloud.then(|| Slot::fit(cloud_section(Cloud::Loading).0));
    let cloud_slot = cloud.as_ref().map(Handle::of);
    if let Some(cloud) = cloud {
        column = column.with_fixed(cloud);
    }
    let column = NewWidget::new(column).with_props(
        PropertySet::new().with(Gap::new(Length::px(size::SPACE_4))).with(Padding::all(Length::px(size::DIALOG_PADDING))),
    );
    let (buttons, ids) = buttons();
    let footer = NewWidget::new(Flex::row().main_axis_alignment(MainAxisAlignment::End).with_fixed(buttons))
        .with_props(PropertySet::new().with(Padding::all(Length::px(size::DIALOG_PADDING))));
    let body = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with(NewWidget::new(ScrollArea::new(column)), 1.0);
    let card = card(header(&t("workspaces.title"), Some(close)), NewWidget::new(body), footer);
    let layer = Modal::new(card, &t("workspaces.title"), CardSize::Window(card_size), Role::Dialog, false);
    let ids = WorkspaceDialog { layer: layer.id(), close: close_id, buttons: ids, list: list_slot, cloud: cloud_slot };
    (layer, ids)
}

pub fn rows(vaults: &[&str], current: Option<usize>, renaming: Option<usize>, error: Option<&str>, always: bool) -> (NewWidget<dyn Widget>, Rows) {
    let mut list = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    if let Some(error) = error {
        list = list.with_fixed(selectable(error, |t| t.text_danger, None, Leading::Px(size::SIZE_20 as f32)));
    }
    if vaults.is_empty() {
        list = list.with_fixed(search::empty_state(icons::LIBRARY, &t("workspaces.empty"), &t("workspaces.emptyHint")));
    }
    let mut rows = Rows { ids: Vec::new(), rename: None };
    for (index, path) in vaults.iter().enumerate() {
        let current = current == Some(index);
        let row = if renaming == Some(index) {
            let (row, field) = WorkspaceRow::renaming(path, current, always);
            rows.rename = Some(field);
            row
        } else {
            WorkspaceRow::new(path, current, always)
        };
        rows.ids.push(row.id());
        list = list.with_fixed(row);
    }
    (NewWidget::new(list).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_4)))).erased(), rows)
}

fn card_size(room: Size) -> Size {
    Size::new(room.width.min(size::WORKSPACE_DIALOG_WIDTH), room.height.min(size::WORKSPACE_DIALOG_MAX_HEIGHT))
}

pub fn no_vault(error: Option<&str>) -> (NewWidget<Flex>, Buttons) {
    let (buttons, ids) = buttons();
    (search::empty_state_with(icons::LIBRARY, &t("fileTree.noFolder"), error, Some(buttons.erased())), ids)
}

fn buttons() -> (NewWidget<Flex>, Buttons) {
    let create = NewWidget::new(TextButton::new(t("workspaces.create"), Variant::Primary).icon_before(icons::FOLDER_PLUS));
    let mut ids = Buttons { create: create.id(), open: None };
    let mut row = Flex::row().main_axis_alignment(MainAxisAlignment::Center);
    if crate::system::DESKTOP {
        let open = NewWidget::new(TextButton::new(t("common.openFolder"), Variant::Secondary).icon_before(icons::FOLDER_OPEN));
        ids.open = Some(open.id());
        row = row.with_fixed(open);
    }
    (NewWidget::new(row.with_fixed(create)).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_8)))), ids)
}
