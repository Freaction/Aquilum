use std::rc::Rc;

use masonry::kurbo::Shape;
use masonry::parley::Layout;

use super::super::drawing::Item;
use super::super::tone::Paint;
use super::super::panel::PAD;
use super::{laid, mermaid};
use crate::ui::editor::embed::Drawing;

const NOTE: &str = include_str!("../../../../../testdata/markdown-test.md");

fn blocks() -> Vec<String> {
    let mut out = Vec::new();
    let mut body: Option<String> = None;
    for line in NOTE.lines() {
        match (body.as_mut(), line.trim()) {
            (None, "```mermaid") => body = Some(String::new()),
            (Some(_), "```") => out.extend(body.take()),
            (Some(text), _) => {
                text.push_str(line);
                text.push('\n');
            }
            _ => {}
        }
    }
    out
}

fn draw(source: &str, available: f64) -> Option<Drawing> {
    mermaid(source, available, &mut |_| Layout::default())
}

fn chain() -> String {
    blocks().into_iter().find(|b| b.starts_with("flowchart LR")).expect("в заметке есть широкая цепочка")
}

#[test]
fn every_example_in_the_spec_note_is_drawn_or_left_as_code() {
    let blocks = blocks();
    assert!(blocks.len() >= 15, "{}", blocks.len());
    for body in &blocks {
        let head = body.lines().next().unwrap_or_default();
        let kept_as_code = head.starts_with("this is not") || body.trim().lines().count() < 2;
        match draw(body, 700.0) {
            Some(drawing) => {
                assert!(!kept_as_code, "{head}: этот тип пока показывается кодом");
                assert!(drawing.height > PAD * 2.0, "{head}: {}", drawing.height);
                assert!(drawing.items.iter().any(|i| matches!(i, Item::Text(..))), "{head}: без подписей");
                assert!(drawing.items.iter().any(|i| matches!(i, Item::Path(..))), "{head}: без фигур");
            }
            None => assert!(kept_as_code, "{head}: не нарисовано"),
        }
    }
}

#[test]
fn colors_come_from_the_theme_at_paint_time() {
    for body in blocks() {
        let Some(drawing) = draw(&body, 700.0) else { continue };
        assert!(drawing.items.iter().all(|i| !matches!(i, Item::Path(_, Paint::Fixed(_), _))), "{body}");
    }
}

#[test]
fn a_wide_diagram_shrinks_to_the_column_before_it_scrolls() {
    let narrow = draw(&chain(), 100.0).unwrap();
    let natural = (narrow.width - PAD * 2.0) / 0.85 + PAD * 2.0;
    assert!(natural > 1000.0, "{natural}");
    assert_eq!(draw(&chain(), natural + 50.0).unwrap().max_scroll(), 0.0, "помещается без ужатия");
    assert_eq!(draw(&chain(), natural * 0.9).unwrap().max_scroll(), 0.0, "ужимается до колонки");
    assert!(draw(&chain(), 360.0).unwrap().max_scroll() > 0.0, "на телефоне прокручивается вбок");
    let phone = draw(&chain(), 360.0).unwrap();
    assert!(phone.height < draw(&chain(), natural + 50.0).unwrap().height, "ужатая диаграмма ниже");
}

#[test]
fn a_small_diagram_stays_centered_and_unscaled() {
    let body = blocks().into_iter().find(|b| b.contains("Очень длинная подпись")).unwrap();
    let wide = draw(&body, 2000.0).unwrap();
    let left = wide.items.iter().filter_map(|i| if let Item::Path(path, ..) = i { Some(path.bounding_box().x0) } else { None }).fold(f64::MAX, f64::min);
    assert!(left > 600.0, "диаграмма по центру колонки: {left}");
    assert_eq!(wide.max_scroll(), 0.0);
}

#[test]
fn formulas_in_labels_are_drawn_as_math() {
    let drawing = draw("flowchart LR\n    A[\"$$\\frac{1}{2}$$\"] -->|\"$$x^2$$\"| B[Площадь $$\\pi r^2$$]", 700.0).unwrap();
    let glyphs = drawing.items.iter().filter(|i| matches!(i, Item::Glyphs(..))).count();
    assert!(glyphs >= 3, "формулы в узлах и на стрелке нарисованы модулем формул: {glyphs}");
    assert!(drawing.items.iter().all(|i| !matches!(i, Item::Text(_, _, t) if t.contains("$$"))), "исходник формулы не выводится текстом");
}

#[test]
fn layouts_are_cached_by_source_text() {
    let body = chain();
    let first = laid(&body).unwrap();
    assert!(Rc::ptr_eq(&first, &laid(&body).unwrap()), "раскладка не пересчитывается при смене ширины");
    assert!(laid("this is not valid mermaid syntax").is_none());
}

#[test]
fn label_plates_keep_their_lines_whatever_the_letter_case() {
    let mut wraps = Vec::new();
    let source = "flowchart TD\n    A -->|да| B\n    A -->|ДА| C\n    A -->|Подпись ПРОПИСНЫМИ| D";
    mermaid(source, 700.0, &mut |spec| {
        if ["да", "ДА", "Подпись ПРОПИСНЫМИ"].contains(&spec.text) {
            wraps.push(spec.wrap);
        }
        Layout::default()
    })
    .unwrap();
    assert_eq!(wraps, vec![None; 3], "строки плашки не переносятся по оценке ширины, высота одна при любом регистре");
}
