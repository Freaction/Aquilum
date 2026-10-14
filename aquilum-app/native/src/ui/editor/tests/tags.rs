use super::support::{click, markdown_root, text, typing};

#[test]
fn the_caret_lands_on_both_sides_of_the_space_between_two_tags() {
    let mut seen = std::collections::BTreeSet::new();
    for x in (0..220).step_by(2) {
        let mut root = markdown_root("#один #два");
        click(&mut root, f64::from(x), 12.0);
        typing(&mut root, "X");
        seen.insert(text(&root).find('X').expect("буква вставлена"));
    }
    let first_end = "#один".len();
    assert!(seen.contains(&first_end) && seen.contains(&(first_end + 1)), "каретку нельзя поставить рядом с пробелом между тегами: {seen:?}");
}
