use std::collections::HashMap;
use std::sync::Arc;

use super::support::unfocused;
use crate::ui::editor::resolve::{Book, Fetch, Grid, Media, Request, Resolver};
use crate::ui::editor::TextEditor;

const WIDTH: u32 = 600;
const HEIGHT: u32 = 300;
const PROBE_X: usize = 400;

struct Pending;

impl Resolver for Pending {
    fn dataview(&self, _query: &str) -> (Fetch<Grid>, u64) {
        (Fetch::Pending, 0)
    }
    fn media(&self, _source: &str, _wiki: bool) -> (Fetch<Media>, u64) {
        (Fetch::Pending, 0)
    }
    fn book(&self, _target: &str) -> (Fetch<Book>, u64) {
        (Fetch::Pending, 0)
    }
    fn request(&self, _request: Request) {}
}

fn block_fill(source: &str, name: &str) -> u32 {
    let editor = TextEditor::new(source).markdown().with_resolver(Some(Arc::new(Pending)));
    let mut root = unfocused(editor, WIDTH, HEIGHT);
    let (layers, _) = root.redraw();
    let mut renderer = crate::render::Renderer::new(2);
    let page = crate::ui::theme::background();
    renderer.render(&layers, WIDTH as u16, HEIGHT as u16, 1.0, page).unwrap();
    if let Ok(out) = std::env::var("AQ_OUT") {
        std::fs::write(std::path::Path::new(&out).join(name), renderer.png()).unwrap();
    }
    let page = renderer.frame_pixels()[(HEIGHT as usize - 1) * WIDTH as usize + PROBE_X];
    let mut seen: HashMap<u32, usize> = HashMap::new();
    for row in renderer.frame_pixels().chunks(WIDTH as usize) {
        if row[PROBE_X] != page {
            *seen.entry(row[PROBE_X]).or_default() += 1;
        }
    }
    seen.into_iter().max_by_key(|(_, count)| *count).map(|(color, _)| color).expect("блок не нарисован")
}

#[test]
fn a_dataview_card_has_the_same_fill_as_a_code_block_in_both_themes() {
    let _lock = crate::ui::theme::test_lock();
    let before = crate::ui::theme::is_dark();
    for dark in [false, true] {
        crate::ui::theme::apply_setting(if dark { "dark" } else { "light" }, dark, "#1471eb");
        let code = block_fill("```js\nx\n```", "card-code.png");
        let card = block_fill("```dataview\nTABLE file.name\n```", "card-dataview.png");
        assert_eq!(format!("{card:08x}"), format!("{code:08x}"), "фон dataview отличается от блока кода (тёмная тема: {dark})");
    }
    crate::ui::theme::apply_setting(if before { "dark" } else { "light" }, before, "#1471eb");
}
