mod row;

use std::path::{Path, PathBuf};

use masonry::accesskit::Role;
use masonry::core::{NewWidget, PropertySet, WidgetId};
use masonry::kurbo::{Axis, Size};
use masonry::layout::Length;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::dialog::{CardSize, Modal, header};
use super::scroll::ScrollArea;
use super::text::selectable;
use super::tokens::size;
use super::widgets::{IconButton, Leading, Rule, TextButton, Variant};
use super::{icons, search, theme};
use crate::i18n::{t, t_with};

pub use row::{RowAction, WorkspaceRow};

pub struct WorkspaceDialog {
    pub layer: WidgetId,
    pub close: WidgetId,
    pub open_folder: WidgetId,
    pub rows: Vec<(WidgetId, PathBuf, uuid::Uuid)>,
}

pub fn dialog(workspaces: &[(uuid::Uuid, String)], current: Option<&Path>, error: Option<&str>) -> (NewWidget<Modal>, WorkspaceDialog) {
    let close = NewWidget::new(IconButton::new(icons::X, t("workspaces.close")));
    let close_id = close.id();
    let mut list = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    let mut rows = Vec::new();
    if workspaces.is_empty() {
        list = list.with_fixed(search::empty_state(icons::LIBRARY, &t("workspaces.empty"), &t("workspaces.emptyHint")));
    }
    for (id, path) in workspaces {
        let is_current = current.is_some_and(|c| same_path(c, Path::new(path)));
        let row = WorkspaceRow::new(path, is_current);
        rows.push((row.id(), PathBuf::from(path), *id));
        list = list.with_fixed(row);
    }
    let list = NewWidget::new(list).with_props(
        PropertySet::new().with(Gap::new(Length::px(size::SPACE_4))).with(Padding::all(Length::px(size::DIALOG_PADDING))),
    );
    let open = NewWidget::new(TextButton::new(t("common.openFolder"), Variant::Primary).with_icon(icons::FOLDER_OPEN));
    let open_id = open.id();
    let footer = NewWidget::new(Flex::row().main_axis_alignment(MainAxisAlignment::End).with_fixed(open))
        .with_props(PropertySet::new().with(Padding::all(Length::px(size::DIALOG_PADDING))));
    let border = || Rule::new(Axis::Horizontal, || theme::current().border_default);
    let mut card = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(header(&t("workspaces.title"), Some(close)))
        .with_fixed(border())
        .with(NewWidget::new(ScrollArea::new(list)), 1.0);
    if let Some(error) = error {
        let error = NewWidget::new(Flex::row().with(selectable(error, |t| t.text_danger, None, Leading::Px(size::SIZE_20 as f32)), 1.0)).with_props(
            PropertySet::new().with(Padding { top: Length::ZERO, bottom: Length::px(size::DIALOG_PADDING), left: Length::px(size::DIALOG_PADDING), right: Length::px(size::DIALOG_PADDING) }),
        );
        card = card.with_fixed(error);
    }
    let card = card.with_fixed(border()).with_fixed(footer);
    let layer = Modal::new(NewWidget::new(card), &t("workspaces.title"), CardSize::Window(card_size), Role::Dialog, false);
    let ids = WorkspaceDialog { layer: layer.id(), close: close_id, open_folder: open_id, rows };
    (layer, ids)
}

fn card_size(room: Size) -> Size {
    Size::new(room.width.min(size::WORKSPACE_DIALOG_WIDTH), room.height.min(size::WORKSPACE_DIALOG_MAX_HEIGHT))
}

pub fn no_vault(error: Option<&str>) -> (NewWidget<Flex>, WidgetId) {
    let open = NewWidget::new(TextButton::new(t("common.openFolder"), Variant::Primary).with_icon(icons::FOLDER_OPEN));
    let id = open.id();
    (search::empty_state_with(icons::FOLDER_OPEN, &t("fileTree.noFolder"), error, Some(open.erased())), id)
}

pub fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| p.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_lowercase();
    norm(a) == norm(b)
}

pub fn forget_texts(path: &Path) -> (String, String) {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    (t("workspaces.forgetTitle"), t_with("workspaces.forgetDescription", &[("name", &name)]))
}
