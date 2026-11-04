use super::*;

pub(super) fn tab_body(bounds: Rect) -> Rect {
    Rect::new(bounds.x0, bounds.y0 + size::PADDING_2XS, bounds.x1, bounds.y1)
}

pub(super) fn content_y(bounds: Rect) -> f64 {
    bounds.center().y + size::TAB_CONTENT_SHIFT
}

pub(super) fn hover(body: Rect) -> Rect {
    Rect { y1: body.y1 - size::PADDING_2XS, ..body } + Vec2::new(0.0, size::TAB_CONTENT_SHIFT)
}

pub(super) fn paint_tab(painter: &mut Painter<'_>, body: Rect, active: bool, hovered: bool) {
    let (t, r) = (theme::current(), size::TAB_RADIUS);
    let (shape, bg) = match (active, hovered) {
        (true, _) => (RoundedRect::from_rect(body, (r, r, 0.0, 0.0)), t.tab_bg_active),
        (false, true) => (RoundedRect::from_rect(hover(body), r), t.tab_bg_hover),
        (false, false) => return,
    };
    painter.fill(shape, bg).draw();
}

pub(super) fn paint_frame(painter: &mut Painter<'_>, tab: Rect, floor: f64) {
    let t = theme::current();
    let (r, w, body) = (size::TAB_RADIUS, size::BORDER_1, tab_body(tab));
    let (left, right, top, line) = (body.x0 + w / 2.0, body.x1 - w / 2.0, body.y0 + w / 2.0, body.y1 + w / 2.0);
    let quarter = std::f64::consts::FRAC_PI_2;
    let arc = |path: &mut BezPath, center: (f64, f64), from: f64, sweep: f64| path.extend(Arc::new(center, (r, r), from, sweep, 0.0).append_iter(0.1));
    let mut edge = BezPath::new();
    edge.move_to((left - r, line));
    arc(&mut edge, (left - r, line - r), quarter, -quarter);
    edge.line_to((left, top + r));
    arc(&mut edge, (left + r, top + r), 2.0 * quarter, quarter);
    edge.line_to((right - r, top));
    arc(&mut edge, (right - r, top + r), 3.0 * quarter, quarter);
    edge.line_to((right, line - r));
    arc(&mut edge, (right + r, line - r), 2.0 * quarter, -quarter);
    let mut skirt = BezPath::new();
    skirt.move_to((left - r, line));
    arc(&mut skirt, (left - r, line - r), quarter, -quarter);
    skirt.line_to((left, body.y1));
    skirt.line_to((right, body.y1));
    skirt.line_to((right, line - r));
    arc(&mut skirt, (right + r, line - r), 2.0 * quarter, -quarter);
    skirt.line_to((right + r, floor));
    skirt.line_to((left - r, floor));
    skirt.close_path();
    painter.fill(&skirt, t.tab_bg_active).draw();
    for color in frame_line() {
        painter.stroke(&edge, &Stroke::new(w), color).draw();
    }
}
