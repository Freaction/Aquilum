use super::*;

const READABLE_SCALE: f64 = 50.0;
const FULLY_READABLE_SCALE: f64 = 100.0;
const LABEL_PADDING: f64 = 2.0;
const LABEL_CANDIDATES: usize = 20_000;
const LABEL_LIMIT: usize = 400;
const NEW_SPRITES: usize = 32;
const LABEL_CELL: f64 = 48.0;
const SPRITE_LIMIT: usize = 1200;
const LAYOUT_LIMIT: usize = 1200;

pub(super) struct Label {
    pub(super) layout: Layout<BrushIndex>,
    pub(super) width: f64,
    pub(super) height: f64,
}

pub(super) fn readable(scale: f64) -> f64 {
    ((scale - READABLE_SCALE) / (FULLY_READABLE_SCALE - READABLE_SCALE)).clamp(0.0, 1.0)
}

pub(super) fn label_layout(fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, title: &str) -> Label {
    let font = theme::ui_font_settings();
    let mut builder = lcx.ranged_builder(fcx, title, 1.0, true);
    builder.push_default(StyleProperty::FontFamily(theme::ui_font(&font.family)));
    builder.push_default(StyleProperty::FontSize(size::FONT_SIZE_UI_XS as f32));
    builder.push_default(StyleProperty::FontWeight(FontWeight::new(number::FONT_WEIGHT_UI_STRONG as f32)));
    builder.push_default(StyleProperty::Brush(BrushIndex(0)));
    let mut layout = builder.build(title);
    layout.break_all_lines(None);
    let (width, height) = (f64::from(layout.width()), f64::from(layout.height()));
    Label { layout, width, height }
}

fn rasterize(labels: &[(&Label, usize)], scale: f64) -> Vec<(usize, Sprite)> {
    let pad = 1usize;
    let boxes: Vec<(usize, usize)> = labels.iter().map(|(l, _)| (((l.width * scale).ceil() as usize + pad * 2).max(1), ((l.height * scale).ceil() as usize + pad * 2).max(1))).collect();
    let width = boxes.iter().map(|b| b.0).max().unwrap_or(1).min(4096);
    let height: usize = boxes.iter().map(|b| b.1).sum();
    if height == 0 || height > u16::MAX as usize {
        return Vec::new();
    }
    let Some(image) = crate::render::offscreen(width as u16, height as u16, |painter| {
        let mut y = 0.0;
        for ((label, _), (_, h)) in labels.iter().zip(&boxes) {
            let at = Affine::translate((pad as f64, y + pad as f64)) * Affine::scale(scale);
            render_text(painter, at, &label.layout, &[Color::WHITE.into()], true);
            y += *h as f64;
        }
    }) else {
        return Vec::new();
    };
    let data = image.image.data.data();
    let mut out = Vec::new();
    let mut top = 0;
    for ((_, node), (w, h)) in labels.iter().zip(&boxes) {
        let (w, h) = ((*w).min(width), *h);
        let mut fill = vec![0u8; w * h];
        for y in 0..h {
            for x in 0..w {
                fill[y * w + x] = data[((top + y) * width + x) * 4 + 3];
            }
        }
        let halo = raster::dilate(&fill, w, h);
        out.push((*node, Sprite { width: w, height: h, fill, halo }));
        top += h;
    }
    out
}

impl GraphView {
    pub(super) fn step_labels(&mut self, seconds: f64, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>) -> bool {
        let _zone = aq_trace::zone("граф: подписи");
        let Some(graph) = self.graph.clone() else { return false };
        let read = readable(self.camera.scale * self.display.spread);
        if !self.display.labels || (read == 0.0 && self.hovered.is_none() && self.fades.is_empty()) {
            self.fades.clear();
            self.shown_labels.clear();
            return false;
        }
        let view = Rect::from_origin_size(Point::ORIGIN, self.size);
        let mut order: Vec<usize> = Vec::new();
        if read > 0.0 {
            let scale = self.camera.scale * self.display.spread;
            let (reach_x, reach_y) = (self.size.width / 2.0 / scale, self.size.height / 2.0 / scale);
            let (cx, cy) = (self.camera.center_x / self.display.spread, self.camera.center_y / self.display.spread);
            graph.visit_area(cx - reach_x, cy - reach_y, cx + reach_x, cy + reach_y, |node| {
                if Some(node) != self.hovered && self.reveal[node] > 0.0 && view.contains(self.screen(node)) {
                    order.push(node);
                }
            });
            order.sort_unstable_by(|&a, &b| graph.degrees[b].cmp(&graph.degrees[a]).then(a.cmp(&b)));
            order.truncate(LABEL_CANDIDATES);
        }
        if let Some(hovered) = self.hovered {
            order.insert(0, hovered);
        }
        let font = size::FONT_SIZE_UI_XS;
        let mut cells: HashMap<(i64, i64), Vec<Rect>> = HashMap::new();
        let mut placed: HashSet<usize> = HashSet::new();
        for &node in &order {
            if placed.len() >= LABEL_LIMIT {
                break;
            }
            let p = self.screen(node);
            let top = p.y + self.radius(&graph, node) + LABEL_GAP;
            let (width, height) = self.labels.get(&node).map_or_else(|| (graph.titles[node].chars().count() as f64 * font * 0.6, font * 1.3), |l| (l.width, l.height));
            let rect = Rect::new(p.x - width / 2.0, top, p.x + width / 2.0, top + height);
            if !rect.overlaps(view) {
                continue;
            }
            let padded = rect.inflate(LABEL_PADDING, LABEL_PADDING);
            let (c0, c1) = ((padded.x0 / LABEL_CELL).floor() as i64, (padded.x1 / LABEL_CELL).floor() as i64);
            let (r0, r1) = ((padded.y0 / LABEL_CELL).floor() as i64, (padded.y1 / LABEL_CELL).floor() as i64);
            let hovered = Some(node) == self.hovered;
            let blocked = !hovered && (r0..=r1).any(|r| (c0..=c1).any(|c| cells.get(&(c, r)).is_some_and(|rs| rs.iter().any(|o| o.overlaps(padded)))));
            if blocked {
                continue;
            }
            for r in r0..=r1 {
                for c in c0..=c1 {
                    cells.entry((c, r)).or_default().push(padded);
                }
            }
            placed.insert(node);
        }
        let step = approach(seconds, FADE);
        let mut busy = false;
        let keys: Vec<usize> = self.fades.keys().copied().chain(placed.iter().copied()).collect::<HashSet<_>>().into_iter().collect();
        self.shown_labels.clear();
        for node in keys {
            let on_screen = view.contains(self.screen(node));
            let target = if placed.contains(&node) { 1.0 } else { 0.0 };
            let fade = self.fades.get(&node).copied().unwrap_or(0.0);
            let mut alpha = fade + (target - fade) * step;
            if target == 0.0 && (alpha <= SETTLED || !on_screen) {
                self.fades.remove(&node);
                busy |= fade > 0.0 && on_screen;
                continue;
            }
            if target == 1.0 && alpha >= 1.0 - SETTLED {
                alpha = 1.0;
            }
            busy |= alpha != target;
            self.fades.insert(node, alpha);
            self.shown_labels.push((node, alpha, Some(node) == self.hovered));
        }
        self.shown_labels.sort_unstable_by_key(|l| (l.2, std::cmp::Reverse(graph.degrees[l.0])));
        let wanted: HashSet<usize> = self.shown_labels.iter().map(|l| l.0).collect();
        if self.labels.len() > LAYOUT_LIMIT {
            self.labels.retain(|n, _| wanted.contains(n));
        }
        for &(node, ..) in &self.shown_labels {
            self.labels.entry(node).or_insert_with(|| label_layout(fcx, lcx, &graph.titles[node]));
        }
        if self.sprite_scale != self.scale_factor {
            self.sprites.clear();
            self.sprite_scale = self.scale_factor;
        }
        if self.sprites.len() > SPRITE_LIMIT {
            self.sprites.retain(|n, _| wanted.contains(n));
        }
        let missing: Vec<(&Label, usize)> = self.shown_labels.iter().filter(|l| !self.sprites.contains_key(&l.0)).filter_map(|l| self.labels.get(&l.0).map(|label| (label, l.0))).collect();
        busy |= missing.len() > NEW_SPRITES;
        let fresh = rasterize(&missing[..missing.len().min(NEW_SPRITES)], self.scale_factor);
        for (node, sprite) in fresh {
            self.sprites.insert(node, sprite);
        }
        busy
    }
}
