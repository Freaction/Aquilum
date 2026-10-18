use super::support::unfocused;
use crate::ui::editor::TextEditor;

const WIDTH: u32 = 600;
const HEIGHT: u32 = 300;

fn render(source: &str, name: &str) -> Vec<u32> {
    let mut root = unfocused(TextEditor::new(source).markdown(), WIDTH, HEIGHT);
    let (layers, _) = root.redraw();
    let mut renderer = crate::render::Renderer::new(2);
    renderer.render(&layers, WIDTH as u16, HEIGHT as u16, 1.0, crate::ui::theme::background()).unwrap();
    if let Ok(out) = std::env::var("AQ_OUT") {
        std::fs::write(std::path::Path::new(&out).join(name), renderer.png()).unwrap();
    }
    renderer.frame_pixels().to_vec()
}

fn most_bars_in_a_row(pixels: &[u32]) -> usize {
    let accent = crate::ui::theme::current().bg_accent.to_rgba8();
    let target = [accent.r, accent.g, accent.b];
    let near = |p: &u32| [(p >> 16) as u8, (p >> 8) as u8, *p as u8].iter().zip(target).all(|(a, b)| a.abs_diff(b) <= 24);
    pixels
        .chunks(WIDTH as usize)
        .map(|row| row.iter().map(near).fold((0, false), |(bars, inside), now| (bars + usize::from(now && !inside), now)).0)
        .max()
        .unwrap_or(0)
}

#[test]
fn a_nested_quote_gets_a_bar_for_every_level() {
    assert_eq!(most_bars_in_a_row(&render("> один\n> два", "quote-1.png")), 1);
    assert_eq!(most_bars_in_a_row(&render("> один\n> > два\n> три", "quote-2.png")), 2);
    assert_eq!(most_bars_in_a_row(&render("> один\n> > два\n> > > три\n\nтекст", "quote-3.png")), 3);
    assert_eq!(most_bars_in_a_row(&render(">один\n>>два", "quote-compact.png")), 2);
    assert_eq!(most_bars_in_a_row(&render("> один\n> ```rust\n> let x = 1;\n> ```\n> > ```\n> > код\n> > ```", "quote-code.png")), 2);
    assert_eq!(most_bars_in_a_row(&render("- пункт\n  ```rust\n  let x = 1;\n  ```\n```\nкод\n```", "list-code.png")), 0);
    assert_eq!(most_bars_in_a_row(&render("текст без цитаты", "quote-none.png")), 0);
}

#[test]
fn highlight_is_painted_softly() {
    let plain = render("слово", "plain.png");
    let marked = render("==слово==", "highlight.png");
    let changed = plain.iter().zip(&marked).filter(|(a, b)| a != b).count();
    assert!(changed > 0, "подсветка ==…== не нарисована");
}
