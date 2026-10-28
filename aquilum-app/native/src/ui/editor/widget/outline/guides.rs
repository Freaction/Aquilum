use super::*;
use crate::ui::editor::embed::panel;
use markdown::{BULLET, Item, LIST_LEVEL, LIST_START, QUOTE_INSET, line_middle, x_at};

impl TextEditor {
    fn bullet_x(&self, j: usize, item: &Item, fonts: &Fonts) -> f64 {
        let p = self.doc.paragraph(j);
        if let Some(layout) = p.laid() {
            let marker = item.marker();
            let (x0, x1) = (x_at(layout, marker.start), x_at(layout, marker.start + 1));
            return self.shift().x + self.doc.origin(j).x + (x0 + x1) / 2.0;
        }
        let view = self.doc.view(j);
        let origin_x = view.origin(self.doc.line(j), fonts) + LIST_START + LIST_LEVEL * item.level() as f64;
        self.shift().x + origin_x - self.doc.scroll_of(p.start) + f64::from(fonts.ch) / 2.0
    }

    fn item_middle_y(&self, j: usize, fonts: &Fonts) -> f64 {
        let p = self.doc.paragraph(j);
        let mid = p.laid().map_or(f64::from(fonts.line) / 2.0, line_middle);
        self.shift().y + self.doc.origin(j).y + mid
    }

    fn outline_guides(&self, first: usize, last: usize, fonts: &Fonts) -> Vec<(f64, f64, f64)> {
        let count = self.doc.line_count();
        if count == 0 {
            return Vec::new();
        }

        let mut parents = Vec::new();
        let first_level = match self.doc.block(first).item() {
            Some(item) => item.level(),
            None => self.doc.view(first).list.map_or(0, |n| n.level + 1),
        };
        for l in 0..first_level {
            let mut j = first;
            while j > 0 {
                j -= 1;
                let block = self.doc.block(j);
                let view = self.doc.view(j);
                let line = self.doc.line(j);
                if let Some(item) = block.item() {
                    if item.level() == l {
                        if !item.ordered {
                            parents.push(j);
                        }
                        break;
                    } else if item.level() < l {
                        break;
                    }
                } else if view.list.is_none() && !line.trim().is_empty() {
                    break;
                }
            }
        }

        for j in first..=last.min(count - 1) {
            if let Some(item) = self.doc.block(j).item() {
                if !item.ordered {
                    parents.push(j);
                }
            }
        }

        parents.sort_unstable();
        parents.dedup();

        let mut guides = Vec::new();
        for p in parents {
            let Some(item) = self.doc.block(p).item() else { continue };
            let level = item.level();
            let in_branch = |idx: usize| {
                if let Some(child) = self.doc.block(idx).item() {
                    child.level() > level
                } else {
                    self.doc.view(idx).list.is_some_and(|nest| nest.level >= level)
                }
            };

            let mut last_child = None;
            let mut k = p + 1;
            while k < count {
                if in_branch(k) {
                    last_child = Some(k);
                    k += 1;
                } else if self.doc.line(k).trim().is_empty() {
                    let mut peek = k + 1;
                    while peek < count && self.doc.line(peek).trim().is_empty() {
                        peek += 1;
                    }
                    if peek < count && in_branch(peek) {
                        k = peek;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            let Some(end) = last_child else { continue };
            if p > last || end < first {
                continue;
            }

            let x = self.bullet_x(p, item, fonts);
            let y0 = self.item_middle_y(p, fonts) + BULLET / 2.0 + size::EDITOR_LIST_GUIDE_GAP;
            let closes = f64::from(self.doc.view(end).closes) * panel::PAD;
            let quote_inset = if self.doc.view(end).quote > 0 { QUOTE_INSET } else { 0.0 };
            let y1 = self.shift().y + self.doc.bottom(end) - closes - quote_inset;

            if y1 > y0 {
                guides.push((x, y0, y1));
            }
        }
        guides
    }

    pub(in super::super) fn paint_outline_guides(&self, painter: &mut Painter<'_>, first: usize, last: usize, fonts: &Fonts) {
        let color = theme::current().editor_list_guide_color;
        for (x, y0, y1) in self.outline_guides(first, last, fonts) {
            let rect = Rect::new(x - size::EDITOR_LIST_GUIDE_WIDTH / 2.0, y0, x + size::EDITOR_LIST_GUIDE_WIDTH / 2.0, y1);
            painter.fill(rect, color).draw();
        }
    }

    #[cfg(test)]
    pub(crate) fn outline_guides_all(&self) -> Vec<(f64, f64, f64)> {
        let Some(fonts) = self.doc.markdown.as_ref() else { return Vec::new() };
        self.outline_guides(0, self.doc.line_count(), fonts)
    }

    #[cfg(test)]
    pub(crate) fn paragraph_bottom(&self, line: usize) -> f64 {
        self.shift().y + self.doc.bottom(line)
    }
}
