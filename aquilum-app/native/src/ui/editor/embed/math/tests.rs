use masonry::core::{BrushIndex, StyleSet};
use masonry::kurbo::{Point, Rect, Shape};
use masonry::peniko::Blob;
use masonry::parley::{FontContext, Layout, LayoutContext, LineMetrics, PositionedInlineBox, PositionedLayoutItem};

use super::super::View;
use super::super::drawing::Item;
use super::math;
use crate::ui::editor::document::Document;
use crate::ui::editor::markdown::{Boxed, Decor, Fonts};

fn formulas(decor: &Decor) -> Vec<&super::Formula> {
    decor.boxes.iter().filter_map(|b| if let Boxed::Formula(f) = b { Some(f) } else { None }).collect()
}

#[derive(Default)]
struct Marks {
    glyphs: Vec<(Point, bool, Blob<u8>)>,
    rules: Vec<(Rect, bool)>,
}

fn marks(source: &str, available: f64) -> Marks {
    let drawing = math(source, 16.0, available).expect("формула нарисована");
    assert!(drawing.height > 16.0, "{}", drawing.height);
    let mut marks = Marks::default();
    for item in &drawing.items {
        match item {
            Item::Glyphs(font, _, run, color) => marks.glyphs.extend(run.iter().map(|g| (Point::new(g.x.into(), g.y.into()), color.is_some(), font.data.clone()))),
            Item::Path(path, color, _) => marks.rules.push((path.bounding_box(), color.is_some())),
            _ => panic!("в формуле только глифы и линии"),
        }
    }
    marks
}

#[test]
fn a_fraction_draws_both_parts_around_a_centered_bar() {
    let m = marks(r"\frac{a}{b}", 600.0);
    let [(bar, _)] = m.rules[..] else { panic!("одна черта дроби: {:?}", m.rules) };
    assert!((bar.center().x - 300.0).abs() < 1.0, "формула не по центру: {bar:?}");
    let [(a, ..), (b, ..)] = &m.glyphs[..] else { panic!("два глифа") };
    assert!(a.y < bar.y0 && b.y > bar.y1, "числитель и знаменатель не по разные стороны черты: {a:?} {b:?} {bar:?}");
}

#[test]
fn broken_or_empty_source_draws_nothing() {
    assert!(math(r"\frac{a", 16.0, 600.0).is_none());
    assert!(math("", 16.0, 600.0).is_none());
}

#[test]
fn a_wide_formula_shrinks_to_the_column() {
    let m = marks("a+b+c+d+e+f+g+h+i+j+k+l+m+n", 60.0);
    assert_eq!(m.glyphs.len(), 27);
    assert!(m.glyphs.iter().all(|(p, ..)| p.x >= -0.5 && p.x < 60.0), "{:?}", m.glyphs.iter().map(|(p, ..)| p.x).collect::<Vec<_>>());
}

#[test]
fn only_explicit_colors_override_the_theme() {
    assert!(marks("x", 600.0).glyphs.iter().all(|(_, colored, _)| !colored));
    assert!(marks(r"\color{red}{x}", 600.0).glyphs.iter().all(|(_, colored, _)| *colored));
}

#[test]
fn text_outside_katex_fonts_uses_the_bundled_fonts() {
    let m = marks(r"\text{Привет}", 600.0);
    assert_eq!(m.glyphs.len(), 6);
    let bundled: Vec<u64> = crate::ui::theme::bundled_fonts().iter().map(|f| f.data.id()).collect();
    assert!(m.glyphs.iter().all(|(.., font)| bundled.contains(&font.id())));
    assert!(marks("x+y", 600.0).glyphs.iter().all(|(.., font)| !bundled.contains(&font.id())));
}

fn laid(text: &str, caret: Option<usize>) -> Document {
    let mut doc = Document::new(text);
    doc.set_markdown(Some(Fonts::new(16.0, true, true)));
    doc.keep = (0.0, 10_000.0);
    let mut styles = StyleSet::new(16.0);
    for property in crate::ui::style::editor_text() {
        styles.insert(property);
    }
    doc.layout(&mut FontContext::default(), &mut LayoutContext::new(), &styles, Some(600.0), None, caret);
    doc
}

fn boxes(layout: &Layout<BrushIndex>) -> Vec<(PositionedInlineBox, LineMetrics)> {
    layout
        .lines()
        .flat_map(|line| line.items().filter_map(move |item| if let PositionedLayoutItem::InlineBox(b) = item { Some((b, *line.metrics())) } else { None }))
        .collect()
}

#[test]
fn a_display_block_is_drawn_until_the_caret_enters_it() {
    let text = "до\n$$\n\\sqrt{x}\n$$\nпосле";
    let doc = laid(text, None);
    let drawn = doc.paragraphs().filter(|p| matches!(p.embed.map(|e| &e.view), Some(View::Drawing(_)))).count();
    assert_eq!(drawn, 1);
    assert_eq!(doc.bottom(2) - doc.top(2), 0.0, "исходник формулы виден вместе с рисунком");
    let doc = laid(text, Some(text.find("sqrt").expect("формула")));
    assert!(doc.bottom(2) - doc.top(2) > 0.0, "внутри формулы не видно исходника");
}

#[test]
fn inline_formulas_replace_their_source_on_the_text_baseline() {
    let text = r"Энергия $E = mc^2$, дробь $\frac{a}{b}$ и строка $$ x^2 + y^2 = z^2 $$ и высокая $\dfrac{\dfrac{a}{b}}{c}$ и $\frac{a$";
    let doc = laid(text, None);
    let p = doc.paragraph(0);
    assert_eq!(formulas(p.decor()).len(), 4, "незакрытая формула должна остаться исходником");
    let found = boxes(p.layout());
    assert_eq!(found.len(), 4);
    for ((b, line), formula) in found.iter().zip(formulas(p.decor())) {
        assert!((b.y + formula.ascent - line.baseline).abs() < 0.5, "формула не на базовой линии: {b:?} {line:?}");
        assert!(b.y >= line.block_min_coord - 0.5 && line.baseline + formula.depth <= line.block_max_coord + 0.5, "формула вылезла из строки: {b:?} {line:?}");
        assert!(b.width > 10.0);
    }
    assert!(found[2].0.height > found[0].0.height, "$$ в строке не в выключном стиле");
    let height = |text: &str| laid(text, None).paragraph(0).layout().height();
    assert_eq!(height(r"Буква $x$"), height("Буква x"), "низкая формула раздвинула строку");
    assert!(height(r"Дробь $\dfrac{\dfrac{a}{b}}{c}$") > height("Дробь"), "высокая формула не раздвинула строку");
    let inside = laid(text, Some(text.find("mc^2").expect("формула")));
    assert_eq!(formulas(inside.paragraph(0).decor()).len(), 3, "под кареткой должен быть исходник");
}
