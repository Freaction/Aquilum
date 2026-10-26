use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update,
    UpdateCtx, Widget, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Stroke};
use masonry::layout::{LenReq, Length};

use crate::i18n::t;
use crate::ui::tokens::size;
use crate::ui::widgets::{EASE_OUT, FAST, Press, Tween, paint_close};
use crate::ui::{icons, text, theme};

#[derive(Debug)]
pub enum RowAction {
    Open,
    Forget,
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
    forget_hovered: bool,
    forget: Tween,
    press: Press,
    name_line: text::Line,
    detail_line: text::Line,
}

const FORGET: f64 = 24.0;

impl WorkspaceRow {
    pub fn new(path: &str, current: bool) -> NewWidget<Self> {
        let name = std::path::Path::new(path).file_name().map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned());
        Self::build(name, path.to_owned(), if current { Kind::Current } else { Kind::Known })
    }

    pub fn cloud(name: &str) -> NewWidget<Self> {
        Self::build(name.to_owned(), t("workspaces.cloudHint"), Kind::Cloud)
    }

    fn build(name: String, detail: String, kind: Kind) -> NewWidget<Self> {
        NewWidget::new(WorkspaceRow {
            name,
            detail,
            kind,
            forget_hovered: false,
            forget: Tween::new(0.0, FAST, EASE_OUT),
            press: Press::default(),
            name_line: text::Line::default(),
            detail_line: text::Line::default(),
        })
    }

    fn forgets(&self) -> bool {
        self.kind == Kind::Known
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
        if let PointerEvent::Move(update) = event {
            let over = self.forgets() && self.forget_rect(bounds).contains(ctx.local_position(update.current.position));
            if over != self.forget_hovered {
                self.forget_hovered = over;
                ctx.request_paint_only();
            }
        }
        if let Some(e) = self.press.click(ctx, event)
            && self.kind != Kind::Current
        {
            let forget = self.forgets() && self.forget_rect(bounds).contains(ctx.local_position(e.state.position));
            ctx.submit_action::<RowAction>(if forget { RowAction::Forget } else { RowAction::Open });
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click && self.kind != Kind::Current {
            ctx.submit_action::<RowAction>(RowAction::Open);
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        if self.forget.advance(interval) {
            ctx.request_anim_frame();
        }
        ctx.request_paint_only();
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(hovered) => {
                self.forget_hovered = false;
                if self.forget.set(if *hovered { 1.0 } else { 0.0 }) {
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
        let mark = match self.kind {
            Kind::Current => Some((icons::CHECK, t.icon_primary)),
            Kind::Cloud => Some((icons::CLOUD, t.icon_secondary)),
            Kind::Known => None,
        };
        if let Some((mark, color)) = mark {
            mark.draw(painter, Point::new(bounds.x0 + gutter, bounds.center().y - icon / 2.0), icon, 1.2, color);
        }
        let text_x = bounds.x0 + gutter + icon + size::SPACE_8;
        let right = if self.forgets() { bounds.x1 - size::SPACE_8 - FORGET - size::SPACE_4 } else { bounds.x1 - gutter };
        let width = (right - text_x).max(0.0);
        let style = text::Style::ui(size::SIZE_20);
        let top = bounds.center().y - size::SIZE_20;
        let name = self.name_line.layout(ctx, &self.name, width, style);
        render_text(painter, Affine::translate((text_x, top)), name, &[t.text_secondary.into()], true);
        let detail = self.detail_line.layout(ctx, &self.detail, width, style);
        render_text(painter, Affine::translate((text_x, top + size::SIZE_20)), detail, &[t.text_tertiary.into()], true);
        let alpha = self.forget.value() as f32;
        if alpha > 0.0 && self.forgets() {
            paint_close(painter, self.forget_rect(bounds), self.forget_hovered, alpha);
        }
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
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
