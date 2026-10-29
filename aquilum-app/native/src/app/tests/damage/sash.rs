use super::*;

#[test]
fn dragging_the_sidebar_edge_repaints_the_edge_but_not_the_tree() {
    let spec = FIXED;
    let mut h = setup(spec, &plain_body(30));
    let sash = h.app.chrome.as_ref().unwrap().sidebar_sash;
    let at = rect_of(&h, sash, spec.scale).center();
    let mut frames = Frames::new(&mut h, spec, true);
    pointer(&mut h, at.x, at.y, spec.scale, false);
    let _ = frames.draw(&mut h);
    press(&mut h, at.x, at.y, spec.scale);
    h.deliver();
    let _ = frames.draw(&mut h);
    let field = rect_of(&h, h.app.tree.filter_field().unwrap(), spec.scale);
    let list_top = field.y1 + (crate::ui::tokens::size::INPUT_XS_PADDING_Y + crate::ui::tokens::size::SPACE_8) * spec.scale;
    let tree = Rect::new(0.0, list_top, at.x - 4.0 * spec.scale, f64::from(spec.height));
    for i in 1..=6 {
        pointer(&mut h, at.x + f64::from(i) * 4.0 * spec.scale, at.y, spec.scale, true);
        h.deliver();
        let report = frames.draw(&mut h);
        report.print(&format!("шаг {i}"));
        assert_eq!(report.differ, 0, "шаг {i}: частичный кадр расходится с полным");
        let inside: f64 = report.causes.iter().map(|r| r.intersect(tree)).map(|r| r.width().max(0.0) * r.height().max(0.0)).sum();
        assert!(inside < tree.area() * 0.05, "шаг {i}: дерево перерисовано на {inside:.0} px из {:.0}", tree.area());
    }
    release(&mut h, at.x, at.y, spec.scale);
}
