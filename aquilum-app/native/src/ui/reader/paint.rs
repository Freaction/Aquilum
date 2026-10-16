use super::*;

impl ReaderView {
    pub(super) fn draw(&mut self, ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let window = ctx.content_box();
        painter.fill(window, tm.bg_canvas).draw();
        let header = Rect::new(0.0, 0.0, window.width(), self.header);
        painter.fill(header, tm.bg_surface).draw();
        let border = self.header + size::BORDER_1 / 2.0;
        painter.stroke(&Segment::new((0.0, border), (window.width(), border)), &Stroke::new(size::BORDER_1), tm.border_subtle).draw();

        let line = Self::line_height();
        let side = size::SPACE_12 + size::SIZE_32 * 2.0 + size::SPACE_4 + size::SPACE_12;
        let inner = (window.width() - side * 2.0).max(1.0);
        let top = ((self.header - line * 2.0 - size::SPACE_2) / 2.0).round();
        let title = if self.title.is_empty() { t("reader.book") } else { self.title.clone() };
        let layout = self.title_line.layout(ctx, &title, inner, Style::ui(line));
        render_text(painter, Affine::translate(((window.width() - f64::from(layout.width())) / 2.0, top)), layout, &[tm.text_primary.into()], true);
        let pages = format!("{}/{}", self.current(), self.total);
        let layout = self.pages_line.layout(ctx, &pages, inner, Style::ui(line));
        render_text(painter, Affine::translate(((window.width() - f64::from(layout.width())) / 2.0, top + line + size::SPACE_2)), layout, &[tm.text_secondary.into()], true);

        let body = self.body;
        let offset = body.origin().to_vec2();
        let marks = self.quote_shapes();
        self.quote_rects = marks.iter().map(|(rects, cfi, _)| (cfi.clone(), rects.clone())).collect();
        let selection = self.ordered().map(|(a, b)| self.session.rects(a, b)).unwrap_or_default();
        painter.with_fill_clip(body, |painter| {
            for (index, (rects, cfi, _)) in marks.iter().enumerate() {
                let fill = self.quote_fill(index, cfi);
                for r in rects {
                    painter.fill(*r + offset, fill).draw();
                }
            }
            for r in &selection {
                painter.fill(*r + offset, tm.selection_bg).draw();
            }
        });
        let palette = [tm.text_primary.into(), tm.text_secondary.into()];
        let items = self.session.items();
        painter.with_fill_clip(body, |painter| {
            for (rect, y0, column) in self.session.visible() {
                for piece in &column.pieces {
                    let band = Rect::new(rect.x0 - size::SPACE_24, body.y0 + y0 + piece.y, rect.x1 + size::SPACE_24, body.y0 + y0 + piece.y + (piece.to - piece.from));
                    if band.y1 < body.y0 || band.y0 > body.y1 {
                        continue;
                    }
                    let origin = Vec2::new(rect.x0, band.y0 - piece.from);
                    match &items[piece.item] {
                        Item::Text { x, .. } => {
                            let Some(layout) = self.session.layout(piece.item) else { continue };
                            let at = Affine::translate(origin + Vec2::new(*x, 0.0));
                            let whole = piece.from <= 0.5 && piece.to >= f64::from(layout.height()) - 0.5;
                            if whole {
                                render_text(painter, at, layout, &palette, true);
                            } else {
                                painter.with_fill_clip(band, |painter| render_text(painter, at, layout, &palette, true));
                            }
                        }
                        Item::Image { size: shown, .. } => {
                            let Some(image) = self.session.image(piece.item) else { continue };
                            let k = shown.width / f64::from(image.image.width);
                            let x = rect.x0 + ((rect.width() - shown.width) / 2.0).max(0.0);
                            painter.with_fill_clip(band, |painter| painter.draw_image(image, Affine::translate((x, band.y0 - piece.from)) * Affine::scale(k)));
                        }
                        Item::Rule => {
                            let y = band.y0 + 0.5;
                            painter.stroke(&Segment::new((rect.x0, y), (rect.x1, y)), &Stroke::new(size::BORDER_1), tm.border_subtle).draw();
                        }
                    }
                }
            }
        });

        self.labels.clear();
        self.mark_lines.resize_with(marks.len(), Line::default);
        let mark_size = f64::from(self.settings.size) * MARK_SCALE;
        for ((rects, cfi, label), line) in marks.iter().zip(&mut self.mark_lines) {
            let Some(last) = rects.last().map(|r| *r + offset) else { continue };
            let style = Style { size: mark_size as f32, weight: self.settings.weight, line_height: mark_size };
            let layout = line.layout(ctx, label, f64::MAX, style);
            let at = Point::new(last.x1 + f64::from(self.settings.size) * 0.15, last.y0 + (last.height() - f64::from(self.settings.size)) / 2.0);
            render_text(painter, Affine::translate(at.to_vec2()), layout, &[tm.text_accent.into()], true);
            self.labels.push((Rect::from_origin_size(at, Size::new(f64::from(layout.width()), mark_size)).inflate(4.0, 4.0), cfi.clone()));
        }

        if self.zones.iter().any(|z| z.value() > 0.0) {
            let hover = self.hover.flatten();
            let nav = nav_width(body.width());
            for (side, zone_alpha) in [(Side::Prev, self.zones[0].value()), (Side::Next, self.zones[1].value())] {
                let zone = match side {
                    Side::Prev => Rect::new(body.x0, body.y0, body.x0 + nav, body.y1),
                    Side::Next => Rect::new(body.x1 - nav, body.y0, body.x1, body.y1),
                };
                let active = hover == Some(side);
                let color = if active { tm.text_primary } else { tm.text_secondary }.multiply_alpha(zone_alpha as f32);
                let icon = if side == Side::Prev { icons::CHEVRON_LEFT } else { icons::CHEVRON_RIGHT };
                let glyph = size::SIZE_24;
                icon.draw(painter, zone.center() - Vec2::new(glyph / 2.0, glyph / 2.0), glyph, 2.0, color);
            }
        }

        let footer = window.height() - PROGRESS - size::BORDER_1;
        painter.fill(Rect::new(0.0, footer, window.width(), window.height()), tm.bg_surface).draw();
        painter.stroke(&Segment::new((0.0, footer + 0.5), (window.width(), footer + 0.5)), &Stroke::new(size::BORDER_1), tm.border_subtle).draw();
        let bar = Rect::new(0.0, window.height() - PROGRESS, window.width(), window.height());
        painter.fill(bar, tm.bg_surface_subtle).draw();
        painter.fill(Rect::new(0.0, bar.y0, bar.width() * self.bar.value(), bar.y1), tm.text_primary.multiply_alpha(0.45)).draw();

        if self.quote_button.is_some() && self.ordered().is_some() && !self.selecting {
            let shape = RoundedRect::from_rect(self.toolbar, size::RADIUS_XL);
            box_shadow(painter, shape, tm.elevation_md);
            painter.fill(shape, tm.bg_surface).draw();
            painter.stroke(&RoundedRect::from_rect(self.toolbar.inflate(-0.5, -0.5), size::RADIUS_XL - 0.5), &Stroke::new(size::BORDER_1), tm.border_subtle).draw();
            let (from, to) = self.ordered().expect("выделение проверено выше");
            let text = self.session.text(from, to);
            let line = Self::line_height();
            let layout = self.preview_line.layout(ctx, &text, self.preview.width().max(1.0), Style::ui(line));
            render_text(painter, Affine::translate((self.preview.x0, self.preview.y0 + ((self.preview.height() - line) / 2.0).round())), layout, &[tm.text_secondary.into()], true);
        }

        if self.popover.is_some() {
            let shape = RoundedRect::from_rect(self.popover_rect, size::RADIUS_XL);
            box_shadow(painter, shape, tm.elevation_float);
            box_shadow(painter, shape, tm.shadow_ring);
            painter.fill(shape, tm.dialog_bg).draw();
        }
    }
}
