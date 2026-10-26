use super::*;
use crate::ui::editor::embed::panel::{self, Header};
use markdown::{Item, LIST_LEVEL, LIST_START, Part, QUOTE_BAR};
use masonry::peniko::Brush;

const BULLET: f64 = 5.0;
const CALLOUT_RADIUS: f64 = 4.0;
const CALLOUT_ALPHA: f32 = 0.12;
const CHECKBOX: f64 = markdown::CHECKBOX as f64;
const CHECKBOX_RADIUS: f64 = 6.0;
const RULE: f64 = 2.0;
const QUOTE_INSET: f64 = 2.0;

fn line_middle(layout: &Layout<BrushIndex>) -> f64 {
    layout.lines().next().map_or(0.0, |l| f64::from(l.metrics().block_min_coord + l.metrics().block_max_coord) / 2.0)
}

fn x_at(layout: &Layout<BrushIndex>, offset: usize) -> f64 {
    Cursor::from_byte_index(layout, offset, Affinity::Downstream).geometry(layout, 0.0).x0
}

impl TextEditor {
    fn runs(&self, first: usize, last: usize, same: impl Fn(usize) -> bool, starts: impl Fn(usize) -> bool) -> Vec<(usize, usize)> {
        let mut runs = Vec::new();
        let mut i = first;
        while i <= last {
            if same(i) {
                let run = self.doc.around(i, &same, &starts);
                runs.push(run);
                i = run.1;
            }
            i += 1;
        }
        runs
    }

    pub(super) fn paint_blocks(&self, painter: &mut Painter<'_>, first: usize, last: usize, palette: &[Brush]) -> Vec<(std::ops::RangeInclusive<usize>, Rect)> {
        let Some(fonts) = self.doc.markdown.as_ref() else { return Vec::new() };
        let tm = theme::current();
        let x = self.shift().x;
        let right = x + self.inner_width();
        let block = |j: usize| self.doc.block(j);
        let view = |j: usize| self.doc.view(j);
        let clips = self.paint_codes(painter, first, last, fonts, palette);
        for k in 1..=(first..=last).map(|j| view(j).panels).max().unwrap_or(0) {
            let header = |j: usize| view(j).panels == k && matches!(block(j), Block::Callout { kind: Callout::Typed(_), .. });
            for (top, end) in self.runs(first, last, |j| view(j).panels >= k, header) {
                let index = match block(top) {
                    Block::Callout { kind: Callout::Typed(index), .. } => usize::from(*index),
                    _ => 0,
                };
                let kind = &panel::KINDS[index];
                let close = view(end);
                let lift = f64::from((k - 1 + close.closes).saturating_sub(close.panels)) * panel::PAD;
                let rect = Rect::new(x + view(top).shell(self.doc.line(top), k, fonts), self.doc.top(top), right - f64::from(k - 1) * panel::PAD, self.doc.bottom(end) - lift);
                panel::paint(painter, rect, kind.tone, Some(Header { height: panel::header(fonts.line), icon: Some(kind.icon), button: None }));
            }
        }
        for level in 1..=(first..=last).map(|j| view(j).quote).max().unwrap_or(0) {
            let bare = |j: usize| view(j).quote >= level && level > view(j).panel;
            let book = |j: usize| matches!(block(j), Block::Callout { kind: Callout::Book, .. });
            for (from, to) in self.runs(first, last, bare, book).into_iter().filter(|&(from, _)| !(book(from) && view(from).quote == level)) {
                let dx = view(from).bar(self.doc.line(from), level, fonts);
                painter.fill(RoundedRect::new(x + dx, self.doc.top(from) + QUOTE_INSET, x + dx + QUOTE_BAR, self.doc.bottom(to) - QUOTE_INSET, QUOTE_BAR / 2.0), tm.bg_accent).draw();
            }
        }
        self.paint_outline_guides(painter, first, last, fonts);
        clips
    }

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

    pub(super) fn paint_outline_guides(&self, painter: &mut Painter<'_>, first: usize, last: usize, fonts: &Fonts) {
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

    pub(super) fn paint_backdrop(&self, painter: &mut Painter<'_>, i: usize) {
        let p = self.doc.paragraph(i);
        if let Some(item) = p.block.item()
            && let Some((_, tone)) = item.callout().filter(|_| self.doc.markdown.as_ref().is_some_and(|fonts| fonts.callouts))
        {
            let x = self.shift().x + self.doc.origin(i).x + x_at(p.layout(), item.marker().start) - LIST_START;
            let shape = RoundedRect::new(x, self.doc.top(i), self.shift().x + self.inner_width() - p.frame.right(), self.doc.bottom(i), CALLOUT_RADIUS);
            painter.fill(shape, markdown::tone(&theme::current(), tone).multiply_alpha(CALLOUT_ALPHA)).draw();
        }
        markdown::paint_backgrounds(painter, p.layout(), p.decor(), self.shift() + self.doc.origin(i));
    }

    pub(super) fn paint_marks(&self, painter: &mut Painter<'_>, i: usize, palette: &[Brush]) {
        let tm = theme::current();
        let p = self.doc.paragraph(i);
        let layout = p.layout();
        let origin = self.shift() + self.doc.origin(i);
        let selection = p.decor().selection.as_ref();
        if matches!(p.block, Block::Rule) && !Part::Rule.shown(selection, &(p.frame.lead..p.len as usize)) {
            let (x, y, view) = (self.shift().x, (self.doc.top(i) + self.doc.bottom(i)) / 2.0, self.doc.view(i));
            let left = self.doc.markdown.as_ref().map_or(0.0, |fonts| view.x(self.doc.line(i), fonts));
            painter.fill(Rect::new(x + left, y - RULE / 2.0, x + self.inner_width() - view.right(), y + RULE / 2.0), tm.icon_quiet).draw();
        }
        if let Some(item) = p.block.item() {
            let (marker, task) = (item.marker(), item.task());
            let middle = origin.y + line_middle(layout);
            if !item.ordered && !Part::Bullet.shown(selection, &item.symbol()) {
                let (x0, x1) = (x_at(layout, marker.start), x_at(layout, marker.start + 1));
                painter.fill(Circle::new((origin.x + (x0 + x1) / 2.0, middle), BULLET / 2.0), tm.editor_list_marker_color).draw();
            }
            if let Some((range, checked)) = task
                && !Part::Task.shown(selection, &range)
            {
                let bounds = self.checkbox_rect(i, range.start);
                let shape = RoundedRect::from_rect(bounds, CHECKBOX_RADIUS);
                if checked {
                    painter.fill(shape, tm.bg_accent).draw();
                    let c = bounds.center();
                    let mut check = BezPath::new();
                    check.move_to((c.x - 4.5, c.y + 0.5));
                    check.line_to((c.x - 1.5, c.y + 3.5));
                    check.line_to((c.x + 4.5, c.y - 3.0));
                    painter.stroke(&check, &Stroke::new(2.0).with_caps(Cap::Round).with_join(Join::Round), tm.icon_on_accent).draw();
                } else {
                    painter.fill(shape, tm.bg_surface_overlay).draw();
                }
            }
        }
        markdown::paint_boxes(painter, layout, p.decor(), origin, palette);
    }

    pub(super) fn checkbox_rect(&self, i: usize, at: usize) -> Rect {
        let layout = self.doc.paragraph(i).layout();
        let origin = self.shift() + self.doc.origin(i);
        let middle = origin.y + line_middle(layout);
        let x = origin.x + x_at(layout, at);
        Rect::new(x, middle - CHECKBOX / 2.0, x + CHECKBOX, middle + CHECKBOX / 2.0)
    }
}
