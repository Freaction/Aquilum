use super::*;
use crate::ui::editor::resolve::Request;

impl TextEditor {
    fn link_at(&self, point: Point) -> Option<&crate::ui::editor::markdown::Link> {
        let i = self.index_at(point)?;
        let p = self.doc.paragraph(i);
        if matches!(p.fold, Fold::Head | Fold::Member) {
            return None;
        }
        let local = point - self.shift() - self.doc.origin(i);
        p.decor().link_at(p.laid()?, local.x, local.y)
    }

    fn task_at(&self, point: Point) -> Option<(Range<usize>, bool)> {
        let i = self.index_at(point)?;
        let p = self.doc.paragraph(i);
        let (range, done) = p.block.item()?.task()?;
        if p.laid().is_none() || matches!(p.fold, Fold::Head | Fold::Member) || p.decor().caret.is_some_and(|c| range.start <= c && c <= range.end) {
            return None;
        }
        let rect = self.checkbox_rect(i, range.start);
        rect.contains(point).then_some((p.start + range.start + 1..p.start + range.end - 1, done))
    }

    fn card_action_at(&self, point: Point) -> Option<crate::ui::editor::embed::Action> {
        match self.head_embed_at(point)? {
            (_, crate::ui::editor::embed::View::Drawing(drawing), local) => drawing.action_at(local).cloned(),
            _ => None,
        }
    }

    fn card_link_at(&self, point: Point) -> Option<(usize, usize)> {
        match self.head_embed_at(point)? {
            (p, crate::ui::editor::embed::View::Drawing(drawing), local) => drawing.link_at(local).map(|hit| (p.start, hit)),
            _ => None,
        }
    }

    pub(super) fn hover_text(&mut self, ctx: &mut EventCtx<'_>, point: Point) {
        let code = self.code_hover(ctx, point);
        let link = self.card_link_at(point).or(code.map(|start| (start, 0)));
        self.pointer_hover = self.link_at(point).is_some() || self.task_at(point).is_some() || self.card_action_at(point).is_some() || link.is_some();
        if link != self.hovered_link {
            self.hovered_link = link;
            ctx.request_render();
        }
    }

    pub(super) fn paint_link_hover(&self, painter: &mut Painter<'_>, i: usize) {
        let p = self.doc.paragraph(i);
        if let Some((_, hit)) = self.hovered_link.filter(|(start, _)| *start == p.start)
            && let Some(crate::ui::editor::embed::View::Drawing(drawing)) = p.embed.map(|e| &e.view)
        {
            drawing.paint_hover(painter, self.embed_x(i), self.doc.top(i), hit);
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
        let Some((mark, done)) = self.task_at(point) else { return false };
        self.dispatch(vec![(mark, if done { " " } else { "x" }.to_owned())], None, Kind::Other);
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
