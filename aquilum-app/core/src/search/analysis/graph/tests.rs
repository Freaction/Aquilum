use super::{identity, GraphSnapshot};
use std::collections::{HashMap, HashSet};
use std::path::Path;

fn node(name: &str) -> String {
    identity(Path::new(&format!("C:/vault/{name}.md")))
}

#[test]
fn returns_candidates_and_their_common_neighbours() {
    let mut neighbours = HashMap::<String, HashSet<String>>::new();
    let mut connect = |left: &str, right: &str| {
        neighbours
            .entry(node(left))
            .or_default()
            .insert(node(right));
        neighbours
            .entry(node(right))
            .or_default()
            .insert(node(left));
    };

    connect("A", "X");
    connect("A", "Y");
    connect("A", "B");
    connect("A", "C");
    connect("X", "C");
    connect("Y", "C");
    connect("X", "B");

    let paths = ["A", "B", "C", "X", "Y"]
        .into_iter()
        .map(|name| (node(name), format!("C:/vault/{name}.md")))
        .collect();
    let outgoing = HashMap::from([
        (node("X"), HashSet::from([node("A"), node("B"), node("C")])),
        (node("Y"), HashSet::from([node("A"), node("C")])),
    ]);
    let graph = GraphSnapshot {
        neighbours,
        outgoing,
        paths,
    };

    let results = graph.adamic_adar(Path::new("C:/vault/A.md"), 10);

    let result = results.iter().find(|result| result.title == "C").unwrap();
    assert_eq!(result.reasons, vec!["X".to_owned(), "Y".to_owned()]);
    let expected = 1.0 / 3_f64.ln() + 1.0 / 2_f64.ln();
    assert!((result.raw_score - expected).abs() < f64::EPSILON);
}

#[test]
fn keeps_candidates_sharing_a_neighbour_with_a_single_outgoing_link() {
    let a = node("A");
    let b = node("B");
    let x = node("X");
    let neighbours = HashMap::from([
        (a.clone(), HashSet::from([x.clone()])),
        (b.clone(), HashSet::from([x.clone()])),
        (x.clone(), HashSet::from([a.clone(), b.clone()])),
    ]);
    let outgoing = HashMap::from([(x, HashSet::from([b.clone()]))]);
    let paths = ["A", "B", "X"]
        .into_iter()
        .map(|name| (node(name), format!("C:/vault/{name}.md")))
        .collect();
    let graph = GraphSnapshot {
        neighbours,
        outgoing,
        paths,
    };

    let result = graph
        .adamic_adar(Path::new("C:/vault/A.md"), 10)
        .into_iter()
        .find(|result| result.title == "B")
        .expect("candidate behind a single-link neighbour must survive ranking");

    assert!((result.raw_score - 1.0 / 2_f64.ln()).abs() < f64::EPSILON);
}

#[test]
fn weights_common_neighbours_by_outgoing_links_like_obsidian_graph_analysis() {
    let a = node("A");
    let b = node("B");
    let x = node("X");
    let y = node("Y");
    let z = node("Z");
    let neighbours = HashMap::from([
        (a.clone(), HashSet::from([x.clone()])),
        (b.clone(), HashSet::from([x.clone()])),
        (
            x.clone(),
            HashSet::from([a.clone(), b.clone(), y.clone(), z.clone()]),
        ),
    ]);
    let outgoing = HashMap::from([(x, HashSet::from([node("A"), node("B")]))]);
    let paths = ["A", "B", "X", "Y", "Z"]
        .into_iter()
        .map(|name| (node(name), format!("C:/vault/{name}.md")))
        .collect();
    let graph = GraphSnapshot {
        neighbours,
        outgoing,
        paths,
    };

    let result = graph
        .adamic_adar(Path::new("C:/vault/A.md"), 10)
        .into_iter()
        .find(|result| result.title == "B")
        .unwrap();

    assert!((result.raw_score - 1.0 / 2_f64.ln()).abs() < f64::EPSILON);
}

#[test]
fn reasons_are_the_strongest_common_neighbours() {
    let hubs = ["H1", "H2", "H3", "H4", "H5"];
    let mut neighbours = HashMap::<String, HashSet<String>>::new();
    let mut outgoing = HashMap::<String, HashSet<String>>::new();
    for (degree, hub) in hubs.iter().enumerate() {
        let targets = HashSet::from_iter([node("A"), node("B")].into_iter().chain((0..degree).map(|i| node(&format!("{hub}-{i}")))));
        neighbours.insert(node(hub), targets.clone());
        outgoing.insert(node(hub), targets);
        for side in ["A", "B"] {
            neighbours.entry(node(side)).or_default().insert(node(hub));
        }
    }
    let paths = ["A", "B"].iter().chain(&hubs).map(|name| (node(name), format!("C:/vault/{name}.md"))).collect();
    let graph = GraphSnapshot { neighbours, outgoing, paths };
    let result = graph.adamic_adar(Path::new("C:/vault/A.md"), 10).into_iter().find(|result| result.title == "B").unwrap();
    assert_eq!(result.reasons, ["H1", "H2", "H3"]);
}

#[test]
fn counts_distinct_notes_linking_to_each_document() {
    let outgoing = HashMap::from([
        (node("X"), HashSet::from([node("A"), node("B")])),
        (node("Y"), HashSet::from([node("A")])),
    ]);
    let graph = GraphSnapshot {
        neighbours: HashMap::new(),
        outgoing,
        paths: HashMap::new(),
    };
    let documents = ["A", "B", "X"].map(|name| std::path::PathBuf::from(format!("C:/vault/{name}.md")));
    assert_eq!(graph.incoming_counts(&documents), vec![2, 1, 0]);
}
