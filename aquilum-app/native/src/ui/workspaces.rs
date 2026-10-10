use std::path::PathBuf;

use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerButton, PointerEvent,
    PropertiesMut, PropertiesRef, PropertySet, RegisterCtx, Update, UpdateCtx, Widget, WidgetId, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LenReq, Length};
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::dialog::{CardSize, Modal, header};
use super::icons;
use super::scroll::ScrollArea;
use super::text::label;
use super::tokens::size;
use super::widgets::{IconButton, Rule, TextButton, Variant};
use super::{text, theme};
use crate::i18n::{t, t_with};

#[derive(Debug)]
pub enum RowAction {
    Open,
    Forget,
}

pub struct WorkspaceRow {
    name: String,
    path: String,
    current: bool,
    forget_hovered: bool,
    down: bool,
    name_line: text::Line,
    path_line: text::Line,
}

const FORGET: f64 = 24.0;

impl WorkspaceRow {
    pub fn new(path: &str, current: bool) -> NewWidget<Self> {
        let name = std::path::Path::new(path).file_name().map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned());
        NewWidget::new(WorkspaceRow {
            name,
            path: path.to_owned(),
            current,
            forget_hovered: false,
            down: false,
            name_line: text::Line::default(),
            path_line: text::Line::default(),
        })
    }

    fn forget_rect(&self, bounds: Rect) -> Rect {
        let x = bounds.x1 - size::SPACE_8 - FORGET;
        let y = bounds.center().y - FORGET / 2.0;
        Rect::new(x, y, x + FORGET, y + FORGET)
    }
}

impl Widget for WorkspaceRow {
    type Action = RowAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        let bounds = ctx.content_box();
        match event {
            PointerEvent::Move(update) => {
                let over = !self.current && self.forget_rect(bounds).contains(ctx.local_position(update.current.position));
                if over != self.forget_hovered {
                    self.forget_hovered = over;
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                self.down = true;
                ctx.capture_pointer();
            }
            PointerEvent::Up(e) if e.button == Some(PointerButton::Primary) && self.down => {
                self.down = false;
                if !ctx.is_hovered() {
                    return;
                }
                let at = ctx.local_position(e.state.position);
                if !self.current && self.forget_rect(bounds).contains(at) {
                    ctx.submit_action::<RowAction>(RowAction::Forget);
                } else if !self.current {
                    ctx.submit_action::<RowAction>(RowAction::Open);
                }
            }
            PointerEvent::Cancel(_) => self.down = false,
            _ => {}
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click && !self.current {
            ctx.submit_action::<RowAction>(RowAction::Open);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(_) => {
                self.forget_hovered = false;
                ctx.request_paint_only();
            }
            Update::FontsChanged => {
                self.name_line.clear();
                self.path_line.clear();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(size::WORKSPACE_ROW_HEIGHT + 2.0),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::ZERO,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let hovered = ctx.is_hovered();
        let shape = RoundedRect::from_rect(bounds.inflate(-0.5, -0.5), size::WORKSPACE_ROW_RADIUS);
        if hovered {
            painter.fill(shape, t.workspace_row_bg_hover).draw();
            painter.stroke(&shape, &Stroke::new(size::BORDER_1), t.workspace_row_border).draw();
        }
        let gutter = size::WORKSPACE_ROW_GUTTER;
        let icon = size::SIZE_20;
        if self.current {
            icons::CHECK.draw(painter, Point::new(bounds.x0 + gutter, bounds.center().y - icon / 2.0), icon, 1.2, t.icon_primary);
        }
        let text_x = bounds.x0 + gutter + icon + size::SPACE_8;
        let right = if self.current { bounds.x1 - gutter } else { bounds.x1 - size::SPACE_8 - FORGET - size::SPACE_4 };
        let width = (right - text_x).max(0.0);
        let style = text::Style::ui(size::SIZE_20);
        let top = bounds.center().y - size::SIZE_20;
        let name = self.name_line.layout(ctx, &self.name, width, style);
        render_text(painter, Affine::translate((text_x, top)), name, &[t.text_secondary.into()], true);
        let path = self.path_line.layout(ctx, &self.path, width, style);
        render_text(painter, Affine::translate((text_x, top + size::SIZE_20)), path, &[t.text_tertiary.into()], true);
        if hovered && !self.current {
            let rect = self.forget_rect(bounds);
            if self.forget_hovered {
                painter.fill(RoundedRect::from_rect(rect, size::ROUNDED_LG), t.icon_button_bg_hover).draw();
            }
            let glyph = size::SIZE_16;
            let color = if self.forget_hovered { t.icon_button_icon_hover } else { t.icon_button_icon };
            icons::X.draw(painter, rect.center() - Vec2::new(glyph / 2.0, glyph / 2.0), glyph, 1.6, color);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.name.clone());
        node.set_description(self.path.clone());
        if self.current {
            node.set_selected(true);
        } else {
            node.add_action(Action::Click);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub struct WorkspaceDialog {
    pub layer: WidgetId,
    pub close: WidgetId,
    pub open_folder: WidgetId,
    pub rows: Vec<(WidgetId, PathBuf, uuid::Uuid)>,
}

pub fn dialog(workspaces: &[(uuid::Uuid, String)], current: Option<&std::path::Path>, error: Option<&str>) -> (NewWidget<Modal>, WorkspaceDialog) {
    let tm = theme::current();
    let close = NewWidget::new(IconButton::new(icons::X, t("workspaces.close")));
    let close_id = close.id();
    let mut list = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    let mut rows = Vec::new();
    if workspaces.is_empty() {
        list = list.with_fixed(super::search::empty_state(icons::LIBRARY, &t("workspaces.empty"), &t("workspaces.emptyHint")));
    }
    for (id, path) in workspaces {
        let is_current = current.is_some_and(|c| same_path(c, std::path::Path::new(path)));
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
        let error = NewWidget::new(Flex::row().with(label(error, tm.text_danger, None, size::SIZE_20 as f32), 1.0)).with_props(
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
    (super::search::empty_state_with(icons::FOLDER_OPEN, &t("fileTree.noFolder"), error, Some(open.erased())), id)
}

pub fn same_path(a: &std::path::Path, b: &std::path::Path) -> bool {
    let norm = |p: &std::path::Path| p.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_lowercase();
    norm(a) == norm(b)
}

pub fn forget_texts(path: &std::path::Path) -> (String, String) {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    (t("workspaces.forgetTitle"), t_with("workspaces.forgetDescription", &[("name", &name)]))
}
