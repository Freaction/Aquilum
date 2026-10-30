use super::*;

impl TextEditor {
    pub(super) fn refresh(&mut self, ctx: &mut EventCtx<'_>) {
        if self.doc.markdown.is_some() {
            ctx.request_layout();
        }
        self.reveal_caret();
        if self.laid_out() {
            let caret = self.caret();
            ctx.set_ime_area(caret);
            if !ctx.is_active() {
                ctx.request_scroll_to(caret);
            }
        }
        ctx.request_render();
    }

    pub(super) fn changed(&mut self, ctx: &mut EventCtx<'_>) {
        ctx.submit_action::<TextAction>(self.doc.changed());
        ctx.request_layout();
        ctx.request_render();
    }

    pub(super) fn ensure_layout(&mut self, fcx: &mut masonry::parley::FontContext, lcx: &mut masonry::parley::LayoutContext<BrushIndex>, width: f64) {
        let _zone = aq_trace::zone("редактор: вёрстка");
        self.width = width;
        let compose = self.compose.as_ref().map(|(text, _)| (self.focus, text.as_str()));
        let wrap = (!self.single_line || self.wraps).then(|| (width - size::CARET_WIDTH * 2.0).max(0.0) as f32);
        let selection = self.selection();
        self.doc.source = self.doc.source.filter(|&source| {
            let p = self.doc.paragraph(self.doc.at_offset(source));
            p.start == source && markdown::touches(Some(&selection), &p.range())
        });
        if self.single_line || self.doc.print {
            self.doc.keep = (f64::NEG_INFINITY, f64::INFINITY);
        }
        self.doc.pins = [self.anchor, self.focus].into_iter().chain(self.extra_cursors.iter().flat_map(|&(a, f)| [a, f])).collect();
        self.doc.layout(fcx, lcx, &self.styles, wrap, compose, self.focused.then(|| self.selection()));
    }
}
