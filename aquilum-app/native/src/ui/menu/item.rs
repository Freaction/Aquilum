use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx,
    Widget, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Circle, Point, RoundedRect, Size, Stroke};
use masonry::layout::{LenReq, Length};
use masonry::peniko::Color;

use super::MenuChoice;
use crate::ui::tokens::size;
use crate::ui::widgets::Press;
use crate::ui::{icons, text, theme};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Kind {
    Plain,
    Watched,
    Parent,
    Sub,
}

pub(super) struct MenuItem {
    index: usize,
    label: String,
    icon: Option<icons::Icon>,
    disabled: bool,
    checked: bool,
    checkable: bool,
    kind: Kind,
    swatch: Option<Option<Color>>,
    line: text::Line,
    press: Press,
}

impl MenuItem {
    pub(super) fn new(index: usize, label: &str, icon: Option<icons::Icon>, disabled: bool, checked: bool, checkable: bool, kind: Kind) -> Self {
        MenuItem { index, label: label.to_owned(), icon, disabled, checked, checkable, kind, swatch: None, line: text::Line::default(), press: Press::default() }
    }

    pub(super) fn swatch(index: usize, label: &str, color: Option<Color>, checked: bool) -> Self {
        MenuItem { swatch: Some(color), ..Self::new(index, label, None, false, checked, true, Kind::Sub) }
    }

    fn choice(&self) -> Option<MenuChoice> {
        match self.kind {
            Kind::Parent => None,
            Kind::Sub => Some(MenuChoice::Sub(self.index)),
            Kind::Plain | Kind::Watched => Some(MenuChoice::Item(self.index)),
        }
    }

    fn leads(&self) -> bool {
        self.icon.is_some() || self.swatch.is_some()
    }

    pub(super) fn height() -> f64 {
        size::PADDING_2XS * 2.0 + size::SIZE_20
    }
}

impl Widget for MenuItem {
    type Action = MenuChoice;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.press.pointer(ctx, event) && !self.disabled
            && let Some(choice) = self.choice()
        {
            ctx.submit_action::<MenuChoice>(choice);
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click && !self.disabled
            && let Some(choice) = self.choice()
        {
            ctx.submit_action::<MenuChoice>(choice);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(hovered) => {
                ctx.request_paint_only();
                if *hovered && matches!(self.kind, Kind::Watched | Kind::Parent) {
                    ctx.submit_action::<MenuChoice>(MenuChoice::Hover(self.index));
                }
            }
            Update::FontsChanged => {
                self.line.clear();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(Self::height()),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => {
                let width = text::measure(ctx, &self.label, text::Style::ui(size::SIZE_20));
                let icon = if self.leads() { size::SIZE_20 + size::GAP_MD } else { 0.0 };
                let check = if self.checkable || self.kind == Kind::Parent { size::SIZE_20 + size::GAP_MD } else { 0.0 };
                Length::px(width.ceil() + size::PADDING_XS * 2.0 + icon + check)
            }
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let hovered = ctx.is_hovered() && !self.disabled;
        if hovered {
            painter.fill(RoundedRect::from_rect(bounds, size::RADIUS_LG), t.bg_accent).draw();
        }
        let lead = if self.leads() { size::SIZE_20 + size::GAP_MD } else { 0.0 };
        if let Some(color) = self.swatch {
            let dot = Circle::new(Point::new(bounds.x0 + size::PADDING_XS + size::SIZE_20 / 2.0, bounds.center().y), size::SIZE_20 * 0.3);
            match color {
                Some(color) => painter.fill(dot, color).draw(),
                None => painter.stroke(&dot, &Stroke::new(1.4), t.icon_muted).draw(),
            }
        }
        if let Some(icon) = self.icon {
            let color = if self.disabled {
                t.text_disabled
            } else if hovered {
                t.icon_on_accent
            } else {
                t.icon_muted
            };
            icon.draw(painter, Point::new(bounds.x0 + size::PADDING_XS, bounds.y0 + size::PADDING_2XS), size::SIZE_20, 1.2, color);
        }
        let check = if self.checkable || self.kind == Kind::Parent { size::SIZE_20 + size::GAP_MD } else { 0.0 };
        let available = (bounds.width() - size::PADDING_XS * 2.0 - check - lead).max(0.0);
        let layout = self.line.layout(ctx, &self.label, available, text::Style::ui(size::SIZE_20));
        let color = if self.disabled {
            t.text_disabled
        } else if hovered {
            t.text_on_accent
        } else {
            t.text_secondary
        };
        let origin = Affine::translate((bounds.x0 + size::PADDING_XS + lead, bounds.y0 + size::PADDING_2XS));
        render_text(painter, origin, layout, &[color.into()], true);
        if self.checked || self.kind == Kind::Parent {
            let at = Point::new(bounds.x1 - size::PADDING_XS - size::SIZE_20, bounds.y0 + size::PADDING_2XS);
            let color = if hovered { t.icon_on_accent } else { t.icon_muted };
            let icon = if self.kind == Kind::Parent { icons::CHEVRON_RIGHT } else { icons::CHECK };
            icon.draw(painter, at, size::SIZE_20, 1.6, color);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::MenuItem
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
        if self.checked {
            node.set_selected(true);
        }
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
