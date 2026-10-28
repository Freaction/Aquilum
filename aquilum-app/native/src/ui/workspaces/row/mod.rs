mod paint;
mod pointer;

use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update,
    UpdateCtx, Widget, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Rect, Size};
use masonry::layout::{LenDef, LenReq, Length, SizeDef};

use crate::i18n::t;
use crate::ui::editor::{TextEditor, rename_editor};
use crate::ui::handle::Handle;
use crate::ui::tokens::size;
use crate::ui::widgets::{EASE_OUT, FAST, Press, Tween};
use crate::ui::{style, text};

#[derive(Debug)]
pub enum RowAction {
    Open,
    Menu(Point),
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Current,
    Known,
    Cloud,
}

pub struct WorkspaceRow {
    name: String,
    detail: String,
    kind: Kind,
    always: bool,
    more_hovered: bool,
    more: Tween,
    press: Press,
    name_line: text::Line,
    detail_line: text::Line,
    field: Option<WidgetPod<TextEditor>>,
}

const MORE: f64 = 24.0;

impl WorkspaceRow {
    pub fn new(path: &str, current: bool, always: bool) -> NewWidget<Self> {
        let name = std::path::Path::new(path).file_name().map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned());
        Self::build(name, path.to_owned(), if current { Kind::Current } else { Kind::Known }, always)
    }

    pub fn renaming(path: &str, current: bool, always: bool) -> (NewWidget<Self>, Handle<TextEditor>) {
        let mut row = Self::new(path, current, always);
        let field = rename_editor(&row.widget.name, style::WORKSPACE_FIELD);
        let handle = Handle::of(&field);
        row.widget.field = Some(field.to_pod());
        (row, handle)
    }

    pub fn cloud(name: &str) -> NewWidget<Self> {
        Self::build(name.to_owned(), t("workspaces.cloudHint"), Kind::Cloud, false)
    }

    fn build(name: String, detail: String, kind: Kind, always: bool) -> NewWidget<Self> {
        NewWidget::new(WorkspaceRow {
            name,
            detail,
            kind,
            always,
            more_hovered: false,
            more: Tween::new(0.0, FAST, EASE_OUT),
            press: Press::default(),
            name_line: text::Line::default(),
            detail_line: text::Line::default(),
            field: None,
        })
    }

    fn has_menu(&self) -> bool {
        self.kind != Kind::Cloud
    }

    fn more_rect(bounds: Rect) -> Rect {
        let x = bounds.x1 - size::SPACE_8 - MORE;
        let y = bounds.center().y - MORE / 2.0;
        Rect::new(x, y, x + MORE, y + MORE)
    }

    fn text_span(&self, bounds: Rect) -> (f64, f64, f64) {
        let left = bounds.x0 + size::WORKSPACE_ROW_GUTTER + size::SIZE_20 + size::SPACE_8;
        let right = if self.has_menu() { Self::more_rect(bounds).x0 - size::SPACE_4 } else { bounds.x1 - size::WORKSPACE_ROW_GUTTER };
        (left, (right - left).max(0.0), bounds.center().y - size::SIZE_20)
    }

    fn menu_at(ctx: &EventCtx<'_>) -> Point {
        let more = Self::more_rect(ctx.content_box());
        ctx.to_window(Point::new(more.x0, more.y1 + size::SPACE_4))
    }
}

impl Widget for WorkspaceRow {
    type Action = RowAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        self.pointer(ctx, event);
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        match event.action {
            Action::Click if self.kind != Kind::Current => ctx.submit_action::<RowAction>(RowAction::Open),
            Action::ShowContextMenu if self.has_menu() => ctx.submit_action::<RowAction>(RowAction::Menu(Self::menu_at(ctx))),
            _ => {}
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        if self.more.advance(interval) {
            ctx.request_anim_frame();
        }
        ctx.request_paint_only();
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(hovered) => {
                self.more_hovered = false;
                if self.more.set(if *hovered { 1.0 } else { 0.0 }) {
                    ctx.request_anim_frame();
                }
                ctx.request_paint_only();
            }
            Update::FontsChanged => {
                self.name_line.clear();
                self.detail_line.clear();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(field) = &mut self.field {
            ctx.register_child(field);
        }
    }

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(size::WORKSPACE_ROW_HEIGHT + 2.0),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::ZERO,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let (left, width, top) = self.text_span(Rect::from_origin_size(Point::ORIGIN, size));
        let Some(field) = &mut self.field else { return };
        let fit = SizeDef::new(LenDef::FitContent(Length::px(width)), LenDef::MaxContent);
        let height = ctx.compute_size(field, fit, Size::new(width, size.height).into()).height;
        ctx.run_layout(field, Size::new(width, height));
        ctx.place_child(field, Point::new(left, top));
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        self.draw(ctx, painter);
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.name.clone());
        node.set_description(self.detail.clone());
        if self.kind == Kind::Current {
            node.set_selected(true);
        } else {
            node.add_action(Action::Click);
        }
        if self.has_menu() {
            node.add_action(Action::ShowContextMenu);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        self.field.as_ref().map_or_else(ChildrenIds::new, |field| ChildrenIds::from_slice(&[field.id()]))
    }
}
