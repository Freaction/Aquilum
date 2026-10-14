use masonry::core::BrushIndex;
use masonry::parley::{Alignment, Layout};

use super::markdown::parse;
use super::model::{Align, DEFAULT_COL, Range2, Table};
use super::view::{TableView, letters};

pub(super) fn table(text: &str) -> Table {
    parse(&text.split('\n').collect::<Vec<_>>()).unwrap()
}

pub(super) fn blank_view(table: &Table) -> TableView {
    table.layout(500.0, None, |_: &str, _: Option<f64>, _: Alignment, _: bool| (Layout::<BrushIndex>::default(), crate::ui::editor::markdown::Decor::default()))
}

#[test]
fn round_trips_merges_and_metadata() {
    let source = "| Этап | <!--q-empty--> | Срок |\n| --- | :---: | ---: |\n| Backend |  | 12.08 |\n| ^^ | ^^ | 14.08 |\n<!--q-table:{\"merges\":[[1,0,2,1]],\"rows\":[40,52,52],\"cols\":[80,120,60]}-->";
    let t = table(source);
    assert_eq!(t.value(2, 1), "Backend");
    assert_eq!(t.aligns, [None, Some(Align::Center), Some(Align::Right)]);
    assert!(t.explicit);
    assert_eq!(t.serialize(), source);
}

#[test]
fn legacy_spans_are_inferred() {
    let t = table("| a | b |\n| --- | --- |\n| x |  |\n| ^^ | ^^ |");
    assert_eq!(t.merges, [Range2 { top: 1, left: 0, bottom: 2, right: 1 }]);
}

#[test]
fn edits_sanitize_and_structure_ops_keep_merges() {
    let mut t = table("| a | b | c |\n| --- | --- | --- |\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |");
    t.set(1, 1, "x|y\n");
    assert_eq!(t.value(1, 1), "x|y ");
    assert!(t.merge(Range2::span((1, 0), (2, 1))));
    assert_eq!(t.value(2, 1), "1");
    t.add_col();
    assert_eq!(t.cols(), 4);
    assert!(t.move_cols(3, 3, 0));
    assert_eq!(t.merges[0].left, 1);
    assert!(t.delete_row(2));
    assert_eq!(t.merges, [Range2 { top: 1, left: 1, bottom: 1, right: 2 }]);
    assert!(t.unmerge(1, 1));
    assert_eq!(letters(27), "AB");
}

#[test]
fn a_table_without_saved_widths_hugs_its_content() {
    let view = blank_view(&table("| a | b |\n| --- | --- |\n| 1 | 2 |"));
    assert_eq!(view.widths(), [DEFAULT_COL, DEFAULT_COL]);
    assert!(view.layout_w < view.scroll_w);
    assert_eq!(view.edge(), view.layout_w);
}

#[test]
fn the_last_border_stays_visible_when_a_wide_table_scrolls_to_its_end() {
    let view = blank_view(&table(&format!("|{}\n|{}\n|{}", " a |".repeat(10), " --- |".repeat(10), " 1 |".repeat(10))));
    let end = view.max_scroll();
    assert!(end > 0.0);
    assert!(view.range_rect(Range2::span((0, 9), (0, 9)), end).x1 < view.scroll_w);
}

#[test]
fn borders_are_found_where_they_are_painted_after_the_table_narrows() {
    use super::view::{Hit, Line};
    use masonry::kurbo::Point;
    let make = |_: &str, _: Option<f64>, _: Alignment, _: bool| (Layout::<BrushIndex>::default(), crate::ui::editor::markdown::Decor::default());
    let wide = table("| a | b | c | d |\n| --- | --- | --- | --- |\n| 1 | 2 | 3 | 4 |\n<!--q-table:{\"merges\":[],\"cols\":[300,300,300,300]}-->");
    let scroll = wide.layout(500.0, None, make).max_scroll();
    let mut narrow = wide.clone();
    narrow.widths = vec![300.0, 300.0, 36.0, 36.0];
    let view = narrow.layout(500.0, None, make);
    assert!(view.max_scroll() < scroll);
    let painted = view.border_line(Hit::ColumnBorder(0), view.max_scroll());
    assert_eq!(view.border_line(Hit::ColumnBorder(0), scroll), painted);
    let Line::Col(x, _) = painted else { panic!("нет линии границы") };
    assert_eq!(view.hit(Point::new(x, 60.0), scroll), Hit::ColumnBorder(0));
}
