use super::*;

impl TextEditor {
    pub fn selected(&self) -> String {
        self.selected_text().unwrap_or_default()
    }

    pub fn table_marks(&self) -> Option<(bool, usize)> {
        self.table.as_ref().map(|s| (s.range.is_some(), s.caret.abs_diff(s.anchor)))
    }

    pub fn image_source_shown(&self) -> bool {
        self.doc.source.is_some()
    }

    pub fn inline_boxes(&self, line: usize) -> usize {
        self.doc.paragraph(line).decor().boxes.len()
    }

    pub fn brush_at(&self, offset: usize) -> Option<BrushIndex> {
        let (_, local, layout) = self.local(offset);
        layout?.lines().flat_map(|line| line.items().collect::<Vec<_>>()).find_map(|item| match item {
            masonry::parley::PositionedLayoutItem::GlyphRun(run) if run.run().text_range().contains(&local) => Some(run.style().brush.clone()),
            _ => None,
        })
    }
}

impl TextEditor {
    pub fn card_scrollbar(&self) -> Option<crate::ui::scroll::bar::Span> {
        self.doc.paragraphs().filter(|p| p.fold == Fold::Head).find_map(|p| self.block_span(p.start))
    }
}
