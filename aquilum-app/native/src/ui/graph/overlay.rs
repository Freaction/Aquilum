use super::*;

impl GraphView {
    pub(super) fn paint_status(&mut self, ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_>, graph: &Graph) {
        let tm = theme::current();
        let total = graph.len();
        let first = if self.visible < total { t_with("graph.notesVisible", &[("visible", &self.visible.to_string()), ("total", &total.to_string())]) } else { plural("graph.notes", total as u64) };
        let mut parts = vec![first, plural("graph.edges", graph.edges.len() as u64)];
        if graph.edges.len() > MAX_EDGES {
            parts.push(t_with("graph.drawnEdges", &[("count", &MAX_EDGES.to_string())]));
        }
        parts.push(t_with("graph.scale", &[("scale", &format!("{:.2}", self.camera.scale))]));
        self.pill(ctx, painter, &parts);
        let _ = tm;
    }

    pub(super) fn pill(&mut self, ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_>, parts: &[String]) {
        let tm = theme::current();
        let line = (f64::from(theme::ui_font_settings().size) * number::FONT_LINE_HEIGHT_NORMAL).round();
        self.status_lines.resize_with(parts.len(), Line::default);
        let widths: Vec<f64> = parts.iter().zip(&mut self.status_lines).map(|(text, cache)| f64::from(cache.layout(ctx, text, f64::MAX, Style::ui(line)).width()).ceil()).collect();
        let width = widths.iter().sum::<f64>() + size::SPACE_12 * (widths.len().saturating_sub(1)) as f64 + size::SPACE_8 * 2.0;
        let height = line + size::SPACE_4 * 2.0;
        let rect = Rect::new(size::SPACE_12, self.size.height - size::SPACE_12 - height, size::SPACE_12 + width, self.size.height - size::SPACE_12);
        painter.fill(RoundedRect::from_rect(rect, size::ROUNDED_SM), tm.graph_status_bg).draw();
        let mut x = rect.x0 + size::SPACE_8;
        for ((text, cache), w) in parts.iter().zip(&mut self.status_lines).zip(widths) {
            let layout = cache.layout(ctx, text, f64::MAX, Style::ui(line));
            render_text(painter, Affine::translate((x, rect.y0 + size::SPACE_4)), layout, &[tm.graph_status_fg.into()], true);
            x += w + size::SPACE_12;
        }
    }

    pub(super) fn paint_notice(&mut self, ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_>, title: &str, description: Option<&str>) {
        let tm = theme::current();
        let line = (f64::from(theme::ui_font_settings().size) * number::FONT_LINE_HEIGHT_NORMAL).round();
        let center = Point::new(self.size.width / 2.0, self.size.height / 2.0);
        let icon = size::SIZE_24;
        crate::ui::icons::NETWORK.draw(painter, center - Vec2::new(icon / 2.0, line + icon), icon, 1.6, tm.text_muted);
        let width = (self.size.width - size::SPACE_24 * 2.0).max(1.0);
        let medium = Style { weight: number::FONT_WEIGHT_UI_MEDIUM as f32, ..Style::ui(line) };
        let layout = self.notice[0].layout(ctx, title, width, medium);
        render_text(painter, Affine::translate((center.x - f64::from(layout.width()) / 2.0, center.y - line / 2.0)), layout, &[tm.text_primary.into()], true);
        if let Some(description) = description {
            let layout = self.notice[1].layout(ctx, description, width, Style::ui(line));
            render_text(painter, Affine::translate((center.x - f64::from(layout.width()) / 2.0, center.y + line / 2.0 + size::SPACE_4)), layout, &[tm.text_secondary.into()], true);
        }
    }
}
