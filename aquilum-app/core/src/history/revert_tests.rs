use super::{shift, Map};

fn moved(at: usize, map: &Map, past_insertions: bool) -> usize {
    shift(at, map.iter().map(|(old, new)| (old, new)), past_insertions)
}

#[test]
fn a_position_moves_by_the_changes_before_it() {
    let map = vec![(2..4, 2..9), (40..41, 45..45)];
    assert_eq!(moved(10, &map, true), 15);
    assert_eq!(moved(41, &map, false), 45);
}

#[test]
fn an_insertion_at_the_position_counts_only_for_a_start() {
    let map = vec![(10..10, 10..13)];
    assert_eq!(moved(10, &map, true), 13);
    assert_eq!(moved(10, &map, false), 10);
}
