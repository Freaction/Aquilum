use super::markdown::parse;
use super::model::{Align, Range2, Table};
use super::view::letters;

fn table(text: &str) -> Table {
    parse(&text.split('\n').collect::<Vec<_>>()).unwrap()
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
    assert_eq!(t.value(1, 1), "x\u{2223}y");
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
fn borders_are_found_where_they_are_painted_after_the_table_narrows() {
    use super::view::{Hit, Line};
    use masonry::core::BrushIndex;
    use masonry::kurbo::Point;
    use masonry::parley::{Alignment, Layout};
    let make = |_: &str, _: Option<f64>, _: Alignment, _: bool| Layout::<BrushIndex>::default();
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
