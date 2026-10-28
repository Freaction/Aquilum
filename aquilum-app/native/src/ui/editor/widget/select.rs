use super::*;

impl TextEditor {
    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.focus)..self.anchor.max(self.focus)
    }

    pub fn selected_text(&self) -> Option<String> {
        let range = self.selection();
        (!range.is_empty()).then(|| self.doc.copy(range))
    }

    pub(super) fn include_caret(&mut self) {
        self.doc.include(self.focus);
    }

    pub(super) fn move_to(&mut self, (offset, affinity): (usize, Affinity), extend: bool) {
        self.focus = offset;
        self.affinity = affinity;
        if !extend {
            self.anchor = offset;
        }
    }

    fn unit_at(&self, point: Point, count: u8) -> Range<usize> {
        let (offset, _) = self.hit(point);
        let i = self.doc.at_offset(offset);
        let p = self.doc.paragraph(i);
        let local = point - self.shift() - self.doc.origin(i);
        match count {
            2 => {
                let Some(layout) = p.laid() else { return offset..offset };
                let r = Selection::word_from_point(layout, local.x as f32, local.y as f32).text_range();
                p.start + r.start..p.start + r.end
            }
            3 => p.range(),
            _ => offset..offset,
        }
    }

    pub(super) fn select_at(&mut self, point: Point, count: u8, extend: bool) {
        let (offset, affinity) = self.hit(point);
        self.unit = None;
        if extend {
            self.move_to((offset, affinity), true);
            return;
        }
        let range = self.unit_at(point, count.min(3));
        if count >= 2 {
            self.unit = Some((count.min(3), range.clone()));
        }
        self.anchor = range.start;
        self.focus = range.end;
        self.affinity = affinity;
        self.column = None;
    }

    pub(super) fn drag_select(&mut self, point: Point) -> bool {
        let before = (self.anchor, self.focus);
        match self.unit.clone() {
            Some((count, first)) => {
                let under = self.unit_at(point, count);
                (self.anchor, self.focus) = if under.start < first.start { (first.end, under.start) } else { (first.start, first.end.max(under.end)) };
                self.column = None;
            }
            None => {
                let target = self.hit(point);
                self.move_to(target, true);
            }
        }
        (self.anchor, self.focus) != before
    }
}
