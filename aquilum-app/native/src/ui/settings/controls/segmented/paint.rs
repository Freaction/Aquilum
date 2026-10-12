use super::*;

impl Segmented {
    pub(super) fn draw(&mut self, ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let shape = RoundedRect::from_rect(bounds, size::RADIUS_XL);
        painter.fill(shape, t.bg_surface).draw();
        ring(painter, shape, t.border_default);
        if let Some(indicator) = self.indicator() {
            painter.fill(RoundedRect::from_rect(indicator, size::RADIUS_LG), t.bg_accent).draw();
        }
        let style = Self::style();
        for (i, option) in self.options.iter().enumerate() {
            let Some(rect) = self.rects.get(i).copied() else { break };
            let color = if (self.shown - i as f64).abs() < 0.5 {
                t.text_on_accent
            } else if self.hovered == Some(i) {
                t.text_secondary
            } else {
                t.text_tertiary
            };
            let layout = self.lines[i].layout(ctx, option, rect.width(), style);
            let x = rect.x0 + (rect.width() - f64::from(layout.width())) / 2.0;
            let y = rect.center().y - style.line_height / 2.0;
            render_text(painter, Affine::translate((x, y)), layout, &[color.into()], true);
        }
        if ctx.is_focus_target() {
            painter.stroke(&RoundedRect::from_rect(bounds.inflate(1.0, 1.0), size::RADIUS_XL + 1.0), &Stroke::new(size::BORDER_1), t.text_accent).draw();
        }
    }
}
