use super::*;

impl ReaderView {
    pub(super) fn arrange(&mut self, ctx: &mut LayoutCtx<'_>, size: Size) {
        let line = Self::line_height();
        self.header = (size::SPACE_8 * 2.0 + line * 2.0 + size::SPACE_2).max(size::SPACE_48);
        let button = Size::new(size::SIZE_32, size::SIZE_32);
        let y = ((self.header - size::SIZE_32) / 2.0).round();
        let close_x = size.width - size::SPACE_12 - size::SIZE_32;
        ctx.run_layout(&mut self.close, button);
        ctx.place_child(&mut self.close, Point::new(close_x, y));
        ctx.run_layout(&mut self.settings_button, button);
        ctx.place_child(&mut self.settings_button, Point::new(close_x - size::SPACE_4 - size::SIZE_32, y));
        self.body = Rect::new(0.0, self.header + size::BORDER_1, size.width, size.height - PROGRESS - size::BORDER_1);
        let (fcx, lcx) = ctx.text_contexts();
        self.session.ensure(self.body.size(), &self.settings, fcx, lcx);
        if let Some(popover) = &mut self.popover {
            let width = POPOVER_WIDTH.min(size.width - size::SPACE_24);
            let auto = SizeDef::new(LenDef::FitContent(Length::px(width)), LenDef::MaxContent);
            let measured = ctx.compute_size(popover, auto, LayoutSize::maybe(Axis::Horizontal, Some(Length::px(width))));
            ctx.run_layout(popover, Size::new(width, measured.height));
            let origin = Point::new(size.width - size::SPACE_12 - width, self.header + size::GAP_XS);
            ctx.place_child(popover, origin);
            self.popover_rect = Rect::from_origin_size(origin, Size::new(width, measured.height));
        }
        let selected = self.ordered().is_some() && !self.selecting;
        if let Some(button) = &mut self.quote_button {
            ctx.set_stashed(button, !selected);
            if selected {
                let width = TOOLBAR_WIDTH.min(size.width - size::SPACE_24);
                let auto = SizeDef::new(LenDef::MaxContent, LenDef::MaxContent);
                let measured = ctx.compute_size(button, auto, LayoutSize::maybe(Axis::Horizontal, Some(Length::px(width))));
                ctx.run_layout(button, measured);
                let height = measured.height + size::SPACE_8 * 2.0;
                let left = ((size.width - width) / 2.0).round();
                self.toolbar = Rect::new(left, self.body.y1 - size::SPACE_16 - height, left + width, self.body.y1 - size::SPACE_16);
                let button_x = self.toolbar.x1 - size::SPACE_12 - measured.width;
                ctx.place_child(button, Point::new(button_x, self.toolbar.y0 + size::SPACE_8));
                self.preview = Rect::new(self.toolbar.x0 + size::SPACE_12, self.toolbar.y0, button_x - size::GAP_MD, self.toolbar.y1);
            }
        }
        self.emit(ctx);
    }
}
