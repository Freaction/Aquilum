use super::support::{TAG, unfocused};
use crate::ui::editor::TextEditor;

const WIDTH: u32 = 600;
const HEIGHT: u32 = 400;

fn editor_guides(source: &str) -> Vec<(f64, f64, f64)> {
    let mut root = unfocused(TextEditor::new(source).markdown(), WIDTH, HEIGHT);
    let _ = root.redraw();
    root.get_widget_with_tag(TAG).expect("редактор").inner().outline_guides_all()
}

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

#[test]
fn flat_list_has_no_vertical_guides() {
    let guides = editor_guides("- один\n- два\n- три");
    assert_eq!(guides.len(), 0, "в плоском списке без вложенности не должно быть направляющих линий");
}

#[test]
fn single_nested_sublist_has_one_guide_stopping_at_last_child() {
    let source = "- родитель\n  - потомок 1\n  - потомок 2\n- следующий";
    let guides = editor_guides(source);
    assert_eq!(guides.len(), 1, "должна быть ровно 1 линия от родителя до потомка 2");
    let (x, y0, y1) = guides[0];
    assert!(x > 10.0 && x < 25.0, "x координата должна соответствовать уровню 0: {x}");
    assert!(y1 > y0, "y1 ({y1}) должно быть больше y0 ({y0})");
    assert!(y0 >= 20.0, "линия должна начинаться с увеличенным по токену отступом (y0 = {y0})");
}

#[test]
fn guide_ends_at_paragraph_bottom_not_bullet_middle() {
    let source = "- родитель\n  - потомок 1\n  - потомок 2";
    let mut root = unfocused(TextEditor::new(source).markdown(), WIDTH, HEIGHT);
    let _ = root.redraw();
    let editor = root.get_widget_with_tag(TAG).expect("редактор").inner();
    let guides = editor.outline_guides_all();
    assert_eq!(guides.len(), 1);
    let (_, _, y1) = guides[0];
    let child_bottom = editor.paragraph_bottom(2);
    assert!((y1 - child_bottom).abs() < 1e-4, "линия должна заканчиваться у низа последнего абзаца ({child_bottom}), а не у буллита: {y1}");
}

#[test]
fn guide_metrics_use_design_tokens() {
    assert_eq!(crate::ui::tokens::size::EDITOR_LIST_GUIDE_GAP, 12.0);
    assert_eq!(crate::ui::tokens::size::EDITOR_LIST_GUIDE_WIDTH, 1.0);
    assert_eq!(crate::ui::tokens::DARK.editor_list_guide_color, crate::ui::tokens::DARK.border_subtle);
}

#[test]
fn deeply_nested_lists_have_hierarchical_guides() {
    let source = "- A\n  - B\n    - C\n  - D";
    let guides = editor_guides(source);
    assert_eq!(guides.len(), 2, "должно быть 2 направляющих (уровень 0 от A до D, уровень 1 от B до C)");
    let (x0, _, _) = guides[0];
    let (x1, _, _) = guides[1];
    assert!(x1 > x0, "направляющая уровня 1 должна быть правее уровня 0: {x1} > {x0}");
}

#[test]
fn nested_code_block_stays_in_scope_and_does_not_conflict() {
    let source = "- родитель\n  - пункт с кодом\n    ```rust\n    let x = 1;\n    ```\n  - второй пункт";
    let guides = editor_guides(source);
    assert!(guides.len() >= 1, "направляющие должны присутствовать при вложенном блоке кода");
    let (x_parent, _, y1_parent) = guides[0];
    assert!(x_parent < 30.0, "линия родителя идет слева от кода: {x_parent}");
    assert!(y1_parent > 0.0);
}

#[test]
fn unindented_horizontal_rule_breaks_list_and_is_not_crossed() {
    let source = "- пункт 1\n  - подпункт 1\n---\n- пункт 2";
    let guides = editor_guides(source);
    assert_eq!(guides.len(), 1, "линия должна остановиться на подпункте 1 перед разделителем");
}

#[test]
fn unindented_code_block_breaks_list() {
    let source = "- пункт 1\n```rust\nlet x = 1;\n```\n- пункт 2";
    let guides = editor_guides(source);
    assert_eq!(guides.len(), 0, "неотступный блок кода прерывает список, никаких висящих линий");
}

#[test]
fn loose_list_with_blank_lines_continues_guides() {
    let source = "- родитель\n\n  - потомок 1\n\n  - потомок 2";
    let guides = editor_guides(source);
    assert_eq!(guides.len(), 1, "пустые строки между пунктами не рвут направляющую");
}

#[test]
fn trailing_blank_line_stops_guide_at_last_item() {
    let source = "- родитель\n  - потомок\n\nобычный текст";
    let guides = editor_guides(source);
    assert_eq!(guides.len(), 1);
}

#[test]
fn rendered_pixels_show_guide_lines() {
    let flat_pixels = render("- один\n- два\n- три", "flat-list.png");
    let nested_pixels = render("- родитель\n  - потомок 1\n  - потомок 2", "nested-list.png");
    let diff = flat_pixels.iter().zip(&nested_pixels).filter(|(a, b)| a != b).count();
    assert!(diff > 0, "при наличии вложенных пунктов должны рисоваться направляющие линии");
}
