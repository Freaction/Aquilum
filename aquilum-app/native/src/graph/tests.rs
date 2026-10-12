use super::snapshot::decode;
use super::{Graph, Range};

fn sample() -> Graph {
    Graph::new(
        vec![[0.0, 0.0], [1.0, 0.0], [5.0, 5.0]],
        vec![1.0, 2.0, 3.0],
        vec![4.0, 5.0, f32::NAN],
        vec![1, 2, 1],
        vec![[0, 1], [1, 2]],
        vec!["a/Первая.md".into(), "b.md".into(), "c.md".into()],
    )
}

#[test]
fn decodes_the_core_snapshot() {
    let mut bytes = Vec::new();
    for v in [2u32, 1, 7, 0] {
        bytes.extend(v.to_le_bytes());
    }
    for v in [1.0f32, 2.0, 3.0, 4.0, 10.0, 11.0, 12.0, 13.0] {
        bytes.extend(v.to_le_bytes());
    }
    for v in [1u32, 1, 0, 1] {
        bytes.extend(v.to_le_bytes());
    }
    let (positions, created, modified, degrees, edges, epoch) = decode(&bytes).unwrap();
    assert_eq!(positions, [[1.0, 2.0], [3.0, 4.0]]);
    assert_eq!((created, modified, degrees, edges, epoch), (vec![10.0, 11.0], vec![12.0, 13.0], vec![1, 1], vec![[0, 1]], 7));
}

#[test]
fn finds_neighbours_levels_and_nodes() {
    let graph = sample();
    assert_eq!(graph.titles[0], "Первая");
    assert_eq!(graph.neighbours(1), [0, 2]);
    let mut seen = Vec::new();
    graph.levels(0, 2, |node, level| seen.push((node, level)));
    assert_eq!(seen, [(0, 0), (1, 1), (2, 2)]);
    assert_eq!(graph.pick(1.0, 0.0, 100.0, 1.0, 1.0, |_| true), Some(1));
    assert_eq!(graph.pick(3.0, 3.0, 100.0, 1.0, 1.0, |_| true), None);
    assert_eq!(graph.modified_range, Range { oldest: 4.0, newest: 5.0 });
}
