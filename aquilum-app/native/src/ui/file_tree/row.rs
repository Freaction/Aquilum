//! Строка дерева файлов — `FileTreeRow.tsx` и `.q-file-item` из `Sidebar.css`.
//!
//! Строка 32 px: отступ 6, слот иконки 20 (шеврон 16 у папок, пустой у файлов), зазор 6, имя
//! с многоточием. Отступ уровня — `sidebar-tree-indent`, направляющая — по центру шеврона
//! уровня-предка. Состояния: наведение, открыта (активная заметка), выделена.

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerButton,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetMut,
    render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size};
use masonry::layout::{LenReq, Length};

use super::model::{Modifiers, Row};
use crate::ui::{icons, text};
use crate::ui::theme;
use crate::ui::tokens::size;

/// Действие строки для окна.
#[derive(Debug)]
pub enum RowAction {
    Press(Modifiers),
    /// Перетаскивание началось (сдвиг на `DRAG_THRESHOLD`); точки — в логических пикселях окна.
    DragStart(Point),
    DragMove(Point),
    Drop(Point),
    /// Правый щелчок; точка — в логических пикселях окна, для меню.
    ContextMenu(Point),
}

/// Сдвиг указателя, после которого нажатие становится перетаскиванием, как во фронтенде.
const DRAG_THRESHOLD: f64 = 5.0;

pub struct TreeRow {
    row: Row,
    pressed: bool,
    /// Где нажали — для порога перетаскивания.
    down_at: Option<Point>,
    dragging: bool,
    /// Строка — цель броска (`data-drop-over`).
    drop_over: bool,
    name: text::Line,
}

impl TreeRow {
    pub fn new(row: Row) -> Self {
        TreeRow { row, pressed: false, down_at: None, dragging: false, drop_over: false, name: text::Line::default() }
    }

    /// Подсветка цели броска; меняет дерево во время перетаскивания.
    pub fn set_drop_over(this: &mut WidgetMut<'_, Self>, drop_over: bool) {
        if this.widget.drop_over != drop_over {
            this.widget.drop_over = drop_over;
            this.ctx.request_paint_only();
        }
    }

    fn height() -> f64 {
        size::SIDEBAR_ITEM_PADDING * 2.0 + size::SIDEBAR_ICON_SIZE
    }

    /// Левый край слота иконки.
    fn icon_x(&self) -> f64 {
        size::SIDEBAR_ITEM_PADDING + self.row.depth as f64 * size::SIDEBAR_TREE_INDENT
    }

    fn text_x(&self) -> f64 {
        self.icon_x() + size::SIDEBAR_ICON_SIZE + size::SIDEBAR_ITEM_GAP
    }
}

impl Widget for TreeRow {
    type Action = RowAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                self.pressed = true;
                self.dragging = false;
                self.down_at = Some(logical(e.state.position, e.state.scale_factor));
                ctx.capture_pointer();
            }
            PointerEvent::Move(update) if self.pressed => {
                let at = logical(update.current.position, update.current.scale_factor);
                if !self.dragging
                    && let Some(start) = self.down_at
                    && ((at.x - start.x).abs() >= DRAG_THRESHOLD || (at.y - start.y).abs() >= DRAG_THRESHOLD)
                {
                    self.dragging = true;
                    ctx.submit_action::<RowAction>(RowAction::DragStart(at));
                }
                if self.dragging {
                    ctx.submit_action::<RowAction>(RowAction::DragMove(at));
                }
            }
            PointerEvent::Up(e) if e.button == Some(PointerButton::Primary) && self.pressed && self.dragging => {
                self.pressed = false;
                self.dragging = false;
                let at = logical(e.state.position, e.state.scale_factor);
                ctx.submit_action::<RowAction>(RowAction::Drop(at));
            }
            PointerEvent::Up(e) if e.button == Some(PointerButton::Primary) && self.pressed => {
                self.pressed = false;
                if ctx.is_hovered() {
                    let m = e.state.modifiers;
                    let action = if cfg!(target_os = "macos") { m.meta() } else { m.ctrl() };
                    ctx.submit_action::<RowAction>(RowAction::Press(Modifiers { action, shift: m.shift() }));
                }
            }
            PointerEvent::Down(e) if e.button == Some(PointerButton::Secondary) => {
                let at = logical(e.state.position, e.state.scale_factor);
                ctx.submit_action::<RowAction>(RowAction::ContextMenu(at));
            }
            PointerEvent::Cancel(_) => {
                self.pressed = false;
                self.dragging = false;
            }
            _ => {}
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == masonry::accesskit::Action::Click {
            ctx.submit_action::<RowAction>(RowAction::Press(Modifiers::default()));
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(_) => ctx.request_paint_only(),
            // Шрифты регистрируются после создания окна: текст надо разложить и измерить заново.
            Update::FontsChanged => {
                self.name.clear();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        _cross_length: Option<Length>,
    ) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(Self::height()),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::px(self.text_x() + size::SIDEBAR_ITEM_PADDING),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let hovered = ctx.is_hovered();

        let background = if self.drop_over {
            Some(t.bg_accent.multiply_alpha(0.05))
        } else if self.row.selected {
            Some(t.bg_accent.multiply_alpha(0.1))
        } else if hovered {
            Some(t.sidebar_item_hover_bg)
        } else if self.row.active {
            Some(t.sidebar_item_active_bg)
        } else {
            None
        };
        if let Some(color) = background {
            painter.fill(RoundedRect::from_rect(bounds, size::SIDEBAR_ITEM_RADIUS), color).draw();
        }
        if self.drop_over {
            // `box-shadow: inset 0 0 0 1.2px var(--q-border-accent)`.
            let ring = RoundedRect::from_rect(bounds.inflate(-0.6, -0.6), size::SIDEBAR_ITEM_RADIUS - 0.6);
            painter.stroke(&ring, &masonry::kurbo::Stroke::new(1.2), t.border_accent).draw();
        }

        for &guide in &self.row.guides {
            // Центр шеврона уровня-предка: отступ + уровень × шаг + половина слота иконки.
            let x = bounds.x0
                + size::SIDEBAR_ITEM_PADDING
                + guide as f64 * size::SIDEBAR_TREE_INDENT
                + size::SIDEBAR_ICON_SIZE / 2.0;
            painter.fill(Rect::new(x, bounds.y0, x + 1.0, bounds.y1), t.sidebar_tree_guide).draw();
        }

        let icon_color = if self.row.selected {
            t.icon_primary
        } else if hovered || self.row.active {
            t.sidebar_icon_hover
        } else {
            t.sidebar_icon
        };
        if self.row.folder {
            let icon = if self.row.expanded { icons::CHEVRON_DOWN } else { icons::CHEVRON_RIGHT };
            let glyph = 16.0;
            let inset = (size::SIDEBAR_ICON_SIZE - glyph) / 2.0;
            let origin = Point::new(bounds.x0 + self.icon_x() + inset, bounds.y0 + size::SIDEBAR_ITEM_PADDING + inset);
            // `.q-file-item svg { stroke-width: 1.6 }` в Sidebar.css.
            icon.draw(painter, origin, glyph, 1.6, icon_color);
        }

        let text_x = self.text_x();
        let available = (bounds.width() - text_x - size::SIDEBAR_ITEM_PADDING).max(0.0);
        let text_color = if self.row.selected { t.text_primary } else { t.sidebar_text };
        let layout = self.name.layout(ctx, &self.row.name, available, text::Style::ui(size::SIDEBAR_ICON_SIZE));
        let origin = Affine::translate((bounds.x0 + text_x, bounds.y0 + size::SIDEBAR_ITEM_PADDING));
        render_text(painter, origin, layout, &[text_color.into()], true);
    }

    fn get_cursor(&self, _ctx: &masonry::core::QueryCtx<'_>, _pos: Point) -> masonry::core::CursorIcon {
        if self.dragging { masonry::core::CursorIcon::Grabbing } else { masonry::core::CursorIcon::Default }
    }

    fn accessibility_role(&self) -> Role {
        Role::TreeItem
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.row.name.clone());
        if self.row.folder {
            node.set_expanded(self.row.expanded);
        }
        node.set_selected(self.row.selected || self.row.active);
        node.add_action(masonry::accesskit::Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

/// Положение указателя (физические пиксели окна) в логических.
fn logical(position: masonry::dpi::PhysicalPosition<f64>, scale: f64) -> Point {
    Point::new(position.x / scale, position.y / scale)
}
