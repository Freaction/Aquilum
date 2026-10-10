use std::path::{Path, PathBuf};

use aquilum_core::bases::BaseColumn;
use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerButton,
    PointerEvent, PointerUpdate, PropertiesMut, PropertiesRef, PropertySet, RegisterCtx, Widget,
    WidgetId, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::Length;
use masonry::parley::LineHeight;
use masonry::parley::style::StyleProperty;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Background, Gap, Padding};
use masonry::properties::{CaretColor, ContentColor, SelectionColor};
use masonry::widgets::{Flex, SizedBox};
use serde_json::Value;

use super::editor::TextEditor;
use super::scroll::ScrollArea;
use super::text::label;
use super::tokens::size;
use super::widgets::{DocumentItem, IconButton, Rule, TextButton, Variant, Wrap};
use super::{icons, theme};
use crate::i18n::t;

pub enum KanbanStatus {
    Loading,
    Ready,
    Error(String),
    NoWorkspace,
}

#[derive(Clone, Debug, PartialEq)]
pub enum KanbanAction {
    OpenNote(PathBuf),
    OpenBoard(PathBuf),
    ClosePicker,
    CreateBoard,
    CreateCard {
        column: Value,
    },
    MoveCard {
        path: PathBuf,
        expected: Value,
        target: Value,
    },
    RenameCard(PathBuf),
    CommitRenameCard(PathBuf),
    DeleteCard(PathBuf),
    DragCard {
        path: PathBuf,
        source: Value,
    },
    DropColumn(Value),
    SelectView(usize),
}

#[derive(Debug)]
pub enum CardDragAction {
    Drop(Point),
}

const CARD_DRAG_THRESHOLD: f64 = 5.0;

fn crossed_drag_threshold(start: Point, at: Point) -> bool {
    (at.x - start.x).abs() >= CARD_DRAG_THRESHOLD || (at.y - start.y).abs() >= CARD_DRAG_THRESHOLD
}

struct DraggableCard {
    child: WidgetPod<dyn Widget>,
    down_at: Option<Point>,
    dragging: bool,
}

impl DraggableCard {
    fn new(child: NewWidget<dyn Widget>) -> NewWidget<Self> {
        NewWidget::new(Self {
            child: child.to_pod(),
            down_at: None,
            dragging: false,
        })
    }

    fn point(ctx: &EventCtx<'_>, position: masonry::dpi::PhysicalPosition<f64>) -> Point {
        ctx.to_window(ctx.local_position(position))
    }
}

impl Widget for DraggableCard {
    type Action = CardDragAction;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(event) if event.button == Some(PointerButton::Primary) => {
                self.down_at = Some(Self::point(ctx, event.state.position));
                self.dragging = false;
            }
            PointerEvent::Move(PointerUpdate { current, .. }) if self.down_at.is_some() => {
                let at = Self::point(ctx, current.position);
                if !self.dragging
                    && self
                        .down_at
                        .is_some_and(|start| crossed_drag_threshold(start, at))
                {
                    self.dragging = true;
                    ctx.capture_pointer();
                }
            }
            PointerEvent::Up(event) if event.button == Some(PointerButton::Primary) => {
                if self.dragging {
                    ctx.submit_action::<CardDragAction>(CardDragAction::Drop(Self::point(
                        ctx,
                        event.state.position,
                    )));
                }
                self.down_at = None;
                self.dragging = false;
            }
            PointerEvent::Cancel(_) => {
                self.down_at = None;
                self.dragging = false;
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: masonry::layout::LenReq,
        cross: Option<Length>,
    ) -> Length {
        ctx.compute_length(
            &mut self.child,
            len_req.into(),
            masonry::layout::LayoutSize::maybe(axis.cross(), cross),
            axis,
            cross,
        )
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ZERO);
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
    }

    fn get_cursor(
        &self,
        ctx: &masonry::core::QueryCtx<'_>,
        _pos: Point,
    ) -> masonry::core::CursorIcon {
        if self.dragging {
            masonry::core::CursorIcon::Grabbing
        } else if ctx.is_hovered() {
            masonry::core::CursorIcon::Grab
        } else {
            masonry::core::CursorIcon::Default
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}

pub struct KanbanView;

impl KanbanView {
    pub fn new(
        columns: &[BaseColumn],
        status: KanbanStatus,
        editable: bool,
        views: &[String],
        selected_view: usize,
        renaming: Option<&Path>,
    ) -> (NewWidget<dyn Widget>, Vec<(WidgetId, KanbanAction)>) {
        let tm = theme::current();
        let mut actions = Vec::new();
        let create_board = NewWidget::new(TextButton::new(t("kanban.newBoard"), Variant::Ghost));
        let create_board_id = create_board.id();
        actions.push((create_board_id, KanbanAction::CreateBoard));
        let mut header = Flex::row()
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .with_fixed(label(
                &t("kanban.title"),
                tm.text_primary,
                None,
                size::SIZE_24 as f32,
            ));
        for (index, name) in views.iter().enumerate() {
            let name = if name.is_empty() {
                format!("{} {}", t("kanban.view"), index + 1)
            } else {
                name.clone()
            };
            let view_button = NewWidget::new(
                TextButton::new(name, Variant::Ghost).pressed(index == selected_view),
            );
            let view_id = view_button.id();
            actions.push((view_id, KanbanAction::SelectView(index)));
            header = header.with_fixed(view_button);
        }
        header = header.with_fixed(create_board);
        let header = NewWidget::new(header).with_props(
            PropertySet::new()
                .with(Gap::new(Length::px(size::SPACE_8)))
                .with(Padding::all(Length::px(size::SPACE_16)))
                .with(Background::Color(tm.tabs_bg)),
        );
        let body = match status {
            KanbanStatus::Loading => status_page(&t("kanban.loading"), tm.text_secondary),
            KanbanStatus::Error(error) => status_page(&error, tm.text_danger),
            KanbanStatus::NoWorkspace => status_page(&t("kanban.noWorkspace"), tm.text_secondary),
            KanbanStatus::Ready => board(columns, editable, renaming, &mut actions),
        };
        let page = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(header)
            .with_fixed(Rule::new(Axis::Horizontal, || {
                theme::current().sidebar_border
            }))
            .with(body, 1.0);
        (
            NewWidget::new(page)
                .with_props(PropertySet::new().with(Background::Color(tm.editor_bg)))
                .erased(),
            actions,
        )
    }
}

pub struct BoardPicker;

impl BoardPicker {
    pub fn new(boards: &[PathBuf]) -> (NewWidget<dyn Widget>, Vec<(WidgetId, KanbanAction)>) {
        let tm = theme::current();
        let mut actions = Vec::new();
        let create = NewWidget::new(TextButton::new(t("kanban.newBoard"), Variant::Primary));
        actions.push((create.id(), KanbanAction::CreateBoard));
        let mut rows = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(create);
        for path in boards {
            let title = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            let item = DocumentItem::new(
                title,
                path.parent().and_then(Path::to_str).map(str::to_owned),
            );
            actions.push((item.id(), KanbanAction::OpenBoard(path.clone())));
            rows = rows.with_fixed(item);
        }
        if boards.is_empty() {
            rows = rows.with_fixed(label(
                &t("kanban.noBoards"),
                tm.text_secondary,
                None,
                size::SIZE_20 as f32,
            ));
        }
        let content = NewWidget::new(ScrollArea::new(NewWidget::new(rows))).erased();
        let close = NewWidget::new(IconButton::new(icons::X, t("common.close")));
        actions.push((close.id(), KanbanAction::ClosePicker));
        let title = NewWidget::new(
            Flex::row()
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .with(
                    label(
                        &t("kanban.boards"),
                        tm.text_primary,
                        None,
                        size::SIZE_24 as f32,
                    ),
                    1.0,
                )
                .with_fixed(close),
        )
        .with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_16))));
        let page = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(title)
            .with(content, 1.0);
        (
            NewWidget::new(page)
                .with_props(PropertySet::new().with(Background::Color(tm.editor_bg)))
                .erased(),
            actions,
        )
    }
}

fn status_page(message: &str, color: masonry::peniko::Color) -> NewWidget<dyn Widget> {
    let content = Flex::column()
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(label(message, color, None, size::SIZE_20 as f32));
    NewWidget::new(content)
        .with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_24))))
        .erased()
}

fn board(
    columns: &[BaseColumn],
    editable: bool,
    renaming: Option<&Path>,
    actions: &mut Vec<(WidgetId, KanbanAction)>,
) -> NewWidget<dyn Widget> {
    let tm = theme::current();
    if columns.is_empty() {
        return status_page(&t("kanban.noColumns"), tm.text_secondary);
    }
    let mut widgets = Vec::with_capacity(columns.len());
    for (column_index, column) in columns.iter().enumerate() {
        let value = column.value.clone();
        let title = group_title(&value);
        let heading = NewWidget::new(Flex::row().with_fixed(label(
            &title,
            tm.text_primary,
            None,
            size::SIZE_20 as f32,
        )));
        let mut cards = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
        if column.rows.is_empty() {
            cards = cards.with_fixed(label(
                &t("kanban.noCards"),
                tm.text_tertiary,
                None,
                size::SIZE_16 as f32,
            ));
        }
        for row in &column.rows {
            let path = PathBuf::from(&row.path);
            let title = row
                .fields
                .get("title")
                .and_then(Value::as_str)
                .filter(|title| !title.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    path.file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or_default()
                        .to_owned()
                });
            let mut card = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
            if renaming == Some(path.as_path()) {
                let font = theme::ui_font_settings();
                let field = NewWidget::new(
                    TextEditor::single_line(&title)
                        .with_style(StyleProperty::FontFamily(theme::ui_font(&font.family)))
                        .with_style(StyleProperty::FontSize(font.size))
                        .with_style(StyleProperty::LineHeight(LineHeight::Absolute(
                            size::SIZE_20 as f32,
                        ))),
                )
                .with_props(
                    PropertySet::new()
                        .with(ContentColor::new(tm.text_primary))
                        .with(CaretColor {
                            color: tm.text_primary,
                        })
                        .with(SelectionColor {
                            color: tm.selection_bg,
                        }),
                );
                actions.push((field.id(), KanbanAction::CommitRenameCard(path.clone())));
                card = card.with(field, 1.0);
            } else {
                let item = DocumentItem::new(&title, None);
                let item_id = item.id();
                actions.push((item_id, KanbanAction::OpenNote(path.clone())));
                if editable {
                    let drag = DraggableCard::new(item.erased());
                    actions.push((
                        drag.id(),
                        KanbanAction::DragCard {
                            path: path.clone(),
                            source: value.clone(),
                        },
                    ));
                    card = card.with(drag, 1.0);
                } else {
                    card = card.with(item, 1.0);
                }
                let rename = NewWidget::new(IconButton::new(icons::PENCIL, t("common.rename")));
                actions.push((rename.id(), KanbanAction::RenameCard(path.clone())));
                card = card.with_fixed(rename);
                let delete = NewWidget::new(IconButton::new(icons::TRASH, t("common.delete")));
                actions.push((delete.id(), KanbanAction::DeleteCard(path.clone())));
                card = card.with_fixed(delete);
            }
            if editable && column_index > 0 {
                let target = columns[column_index - 1].value.clone();
                let button =
                    NewWidget::new(IconButton::new(icons::ARROW_LEFT, t("kanban.movePrevious")));
                actions.push((
                    button.id(),
                    KanbanAction::MoveCard {
                        path: path.clone(),
                        expected: value.clone(),
                        target,
                    },
                ));
                card = card.with_fixed(button);
            }
            if editable && column_index + 1 < columns.len() {
                let target = columns[column_index + 1].value.clone();
                let button =
                    NewWidget::new(IconButton::new(icons::ARROW_RIGHT, t("kanban.moveNext")));
                actions.push((
                    button.id(),
                    KanbanAction::MoveCard {
                        path,
                        expected: value.clone(),
                        target,
                    },
                ));
                card = card.with_fixed(button);
            }
            cards = cards.with_fixed(
                NewWidget::new(card).with_props(
                    PropertySet::new()
                        .with(Background::Color(tm.sidebar_bg))
                        .with(Padding::all(Length::px(size::SPACE_6))),
                ),
            );
        }
        let list = NewWidget::new(ScrollArea::new(NewWidget::new(cards))).erased();
        let column_widget = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(heading)
            .with(list, 1.0);
        let column_widget = if editable {
            let create = NewWidget::new(TextButton::new(t("kanban.addCard"), Variant::Ghost));
            actions.push((
                create.id(),
                KanbanAction::CreateCard {
                    column: value.clone(),
                },
            ));
            column_widget.with_fixed(create)
        } else {
            column_widget
        };
        let column_widget = NewWidget::new(
            SizedBox::new(NewWidget::new(column_widget)).width(Length::px(size::BACKLINKS_WIDTH)),
        )
        .with_props(
            PropertySet::new()
                .with(Background::Color(tm.sidebar_bg))
                .with(Padding::all(Length::px(size::SPACE_12))),
        );
        if editable {
            actions.push((column_widget.id(), KanbanAction::DropColumn(value)));
        }
        widgets.push(column_widget.erased());
    }
    let layout = Wrap::new(widgets, size::SPACE_12);
    NewWidget::new(ScrollArea::new(layout)).erased()
}

fn group_title(value: &Value) -> String {
    match value {
        Value::Null => t("kanban.unassigned"),
        Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{crossed_drag_threshold, group_title};
    use masonry::kurbo::Point;
    use serde_json::json;

    #[test]
    fn group_title_keeps_scalar_values_and_names_unassigned() {
        assert!(!group_title(&json!(null)).is_empty());
        assert_eq!(group_title(&json!("Doing")), "Doing");
        assert_eq!(group_title(&json!(3)), "3");
        assert_eq!(group_title(&json!(true)), "true");
    }

    #[test]
    fn card_drag_threshold_preserves_clicks_and_starts_pointer_drag() {
        assert!(!crossed_drag_threshold(Point::ZERO, Point::new(4.9, 0.0)));
        assert!(crossed_drag_threshold(Point::ZERO, Point::new(5.0, 0.0)));
        assert!(crossed_drag_threshold(Point::ZERO, Point::new(0.0, -5.0)));
    }
}
