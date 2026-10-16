use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, ComposeCtx, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update,
    UpdateCtx, Widget, WidgetMut, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Stroke};
use masonry::layout::{LenReq, Length};

use super::marked::{CardLayouts, paint_marked, snippet_line};
use crate::ui::icons;
use crate::ui::theme;
use crate::ui::tokens::size;
use crate::ui::widgets::Press;

pub type Marks = Vec<(usize, usize)>;

pub struct CardData {
    pub title: String,
    pub extension: String,
    pub snippet: String,
    pub count: String,
    pub title_marks: Marks,
    pub extension_marks: Marks,
    pub snippet_marks: Marks,
}

#[derive(Debug)]
pub enum CardAction {
    Hover,
    Open { new_tab: bool },
}

#[derive(Clone, Copy, PartialEq)]
pub enum CardKind {
    Result,
    Template,
}

impl CardKind {
    pub fn height(self) -> f64 {
        match self {
            CardKind::Result => size::SEARCH_RESULT_HEIGHT,
            CardKind::Template => size::SIZE_44,
        }
    }
}

pub struct ResultCard {
    data: CardData,
    kind: CardKind,
    selected: bool,
    press: Press,
    layouts: Option<CardLayouts>,
    shown: bool,
    painted: bool,
}

impl ResultCard {
    pub fn new(data: CardData, selected: bool) -> NewWidget<Self> {
        Self::of(data, CardKind::Result, selected)
    }

    pub fn template(data: CardData, selected: bool) -> NewWidget<Self> {
        Self::of(data, CardKind::Template, selected)
    }

    fn of(data: CardData, kind: CardKind, selected: bool) -> NewWidget<Self> {
        NewWidget::new(ResultCard { data, kind, selected, press: Press::default(), layouts: None, shown: false, painted: false })
    }

    pub fn set_selected(this: &mut WidgetMut<'_, Self>, selected: bool) {
        if this.widget.selected != selected {
            this.widget.selected = selected;
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }
}

impl Widget for ResultCard {
    type Action = CardAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if let Some(e) = self.press.click(ctx, event) {
            let m = e.state.modifiers;
            ctx.submit_action::<CardAction>(CardAction::Open { new_tab: crate::ui::keymap::action(m) });
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            ctx.submit_action::<CardAction>(CardAction::Open { new_tab: false });
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(true) => ctx.submit_action::<CardAction>(CardAction::Hover),
            Update::HoveredChanged(false) => ctx.request_paint_only(),
            Update::FontsChanged => {
                self.layouts = None;
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(self.kind.height()),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::ZERO,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {
        let bounds = ctx.window_transform().transform_rect_bbox(ctx.border_box());
        self.shown = bounds.y1 > 0.0 && bounds.y0 < ctx.window_size().height;
        if self.shown && !self.painted {
            ctx.request_paint_only();
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        self.painted = self.shown;
        if !self.shown {
            return;
        }
        let t = theme::current();
        let bounds = ctx.content_box();
        let shape = RoundedRect::from_rect(bounds.inflate(-0.5, -0.5), size::ROUNDED_XL);
        let hot = self.selected || ctx.is_hovered();
        let template = self.kind == CardKind::Template;
        if hot || !template {
            painter.fill(shape, if hot { t.search_result_bg_hover } else { t.search_result_bg }).draw();
        }
        if !template {
            painter.stroke(&shape, &Stroke::new(size::BORDER_1), t.search_result_border).draw();
        }

        let pad = size::SPACE_12;
        let top = bounds.y0 + pad;
        let (icon, tint, extension_color) = if template { (icons::FILE_TEXT, t.icon_muted, t.text_disabled) } else { (icons::FILE, t.icon_secondary, t.text_tertiary) };
        icon.draw(painter, Point::new(bounds.x0 + pad, top), size::SIZE_20, 1.2, tint);

        let snippet_x = bounds.x0 + pad + size::SIZE_28;
        let width = (bounds.x1 - pad - snippet_x).max(0.0) as f32;
        if !self.layouts.as_ref().is_some_and(|layouts| layouts.fresh(width)) {
            self.layouts = Some(CardLayouts::build(ctx, &self.data, width));
        }
        let Some(layouts) = &self.layouts else { return };

        let count_x = bounds.x1 - pad - f64::from(layouts.count.width());
        render_text(painter, Affine::translate((count_x, top)), &layouts.count, &[t.text_tertiary.into()], true);

        let name_x = bounds.x0 + pad + size::SIZE_20 + size::SPACE_6;
        let name_right = count_x - size::SPACE_12;
        painter.push_fill_clip(Rect::new(name_x, top, name_right.max(name_x), top + size::SIZE_20));
        paint_marked(painter, &layouts.title, &self.data.title_marks, Point::new(name_x, top), t.text_secondary);
        if let Some((extension, marks)) = &layouts.extension {
            paint_marked(painter, extension, marks, Point::new(name_x + f64::from(layouts.title.width()), top), extension_color);
        }
        painter.pop_clip();

        let snippet_y = top + size::SIZE_20 + size::SPACE_6;
        let line_height = f64::from(snippet_line());
        painter.push_fill_clip(Rect::new(snippet_x, snippet_y, bounds.x1 - pad, snippet_y + line_height * 3.0));
        paint_marked(painter, &layouts.snippet, &self.data.snippet_marks, Point::new(snippet_x, snippet_y), t.text_tertiary);
        painter.pop_clip();
    }

    fn accessibility_role(&self) -> Role {
        Role::ListBoxOption
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.data.title.clone());
        node.set_selected(self.selected);
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
