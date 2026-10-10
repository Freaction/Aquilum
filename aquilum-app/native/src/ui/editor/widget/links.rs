use super::*;
use crate::ui::editor::resolve::Request;

impl TextEditor {
    fn link_at(&self, point: Point) -> Option<&crate::ui::editor::markdown::Link> {
        if self.doc.markdown.is_none() || !self.laid_out() {
            return None;
        }
        let p = self.doc.paragraph(self.doc.at_y(point.y.max(0.0)));
        if p.fold != Fold::None || p.decor.links.is_empty() {
            return None;
        }
        let local = point - self.shift() - p.origin();
        let layout = p.layout();
        if local.y < 0.0 || local.y > f64::from(layout.height()) || local.x < 0.0 || local.x > f64::from(layout.width()) {
            return None;
        }
        let at = Cursor::from_point(layout, local.x as f32, local.y as f32).index();
        p.decor.links.iter().find(|l| l.label.start <= at && at < l.label.end)
    }

    fn task_at(&self, point: Point) -> Option<(usize, bool)> {
        if self.doc.markdown.is_none() || !self.laid_out() {
            return None;
        }
        let p = self.doc.paragraph(self.doc.at_y(point.y.max(0.0)));
        let Block::Item { task: Some((range, done)), .. } = &p.block else { return None };
        if p.fold != Fold::None || p.decor.caret.is_some_and(|c| range.start <= c && c <= range.start + super::marks::TASK_BOX) {
            return None;
        }
        let rect = self.checkbox_rect(p, range.start);
        rect.contains(point).then_some((p.start + range.start, *done))
    }

    fn card_action_at(&self, point: Point) -> Option<crate::ui::editor::embed::Action> {
        if self.doc.markdown.is_none() || !self.laid_out() {
            return None;
        }
        let p = self.doc.paragraph(self.doc.at_y(point.y.max(0.0)));
        match (p.fold, p.embed.as_deref().map(|e| &e.view)) {
            (Fold::Head, Some(crate::ui::editor::embed::View::Drawing(drawing))) => drawing.action_at(Point::new(point.x - self.shift().x, point.y - p.top)).cloned(),
            _ => None,
        }
    }

    fn card_link_at(&self, point: Point) -> Option<(usize, usize)> {
        if self.doc.markdown.is_none() || !self.laid_out() {
            return None;
        }
        let p = self.doc.paragraph(self.doc.at_y(point.y.max(0.0)));
        match (p.fold, p.embed.as_deref().map(|e| &e.view)) {
            (Fold::Head, Some(crate::ui::editor::embed::View::Drawing(drawing))) => drawing.link_at(Point::new(point.x - self.shift().x, point.y - p.top)).map(|hit| (p.start, hit)),
            _ => None,
        }
    }

    pub(super) fn hover_text(&mut self, ctx: &mut EventCtx<'_>, point: Point) {
        self.pointer_hover = self.link_at(point).is_some() || self.task_at(point).is_some() || self.card_action_at(point).is_some();
        let link = self.card_link_at(point);
        if link != self.hovered_link {
            self.hovered_link = link;
            ctx.request_render();
        }
    }

    pub(super) fn paint_link_hover(&self, painter: &mut Painter<'_>, p: &Paragraph) {
        if let Some((_, hit)) = self.hovered_link.filter(|(start, _)| *start == p.start)
            && let Some(crate::ui::editor::embed::View::Drawing(drawing)) = p.embed.as_deref().map(|e| &e.view)
        {
            drawing.paint_hover(painter, self.shift().x, p.top, hit);
        }
    }

    pub(super) fn card_click(&mut self, ctx: &mut EventCtx<'_>, point: Point, new_tab: bool) -> bool {
        let Some(resolver) = self.doc.resolver.clone() else { return false };
        let Some(action) = self.card_action_at(point) else { return false };
        resolver.request(match action {
            crate::ui::editor::embed::Action::Task { target, line } => Request::ToggleTask { target, line },
            crate::ui::editor::embed::Action::Link { target } => Request::Open { target, wiki: true, new_tab },
            crate::ui::editor::embed::Action::Read { file, page, title } => Request::Read { file, page, title },
        });
        ctx.set_handled();
        true
    }

    pub(super) fn toggle_task(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
        let Some((at, done)) = self.task_at(point) else { return false };
        let keep = (self.anchor, self.focus);
        self.replace(at + 1..at + 2, if done { " " } else { "x" }, Kind::Other);
        (self.anchor, self.focus) = keep;
        ctx.set_handled();
        self.changed(ctx);
        true
    }

    pub(super) fn open_link(&mut self, ctx: &mut EventCtx<'_>, point: Point, new_tab: bool) -> bool {
        let Some(resolver) = self.doc.resolver.clone() else { return false };
        let Some(link) = self.link_at(point) else { return false };
        resolver.request(Request::Open { target: link.target.clone(), wiki: link.wiki, new_tab });
        ctx.set_handled();
        true
    }
}
