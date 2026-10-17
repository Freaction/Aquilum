use super::layout;
use super::rank;
use crate::search::error::SearchError;
use crate::search::wiki;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;

const NANOS_PER_DAY: f64 = 86_400_000_000_000.0;
const LAYOUT_REPULSION: f64 = 1.0;

pub struct RenderSnapshot {
    pub paths: Vec<String>,
    pub positions: Vec<f32>,
    pub created_days: Vec<f32>,
    pub modified_days: Vec<f32>,
    pub degrees: Vec<u32>,
    pub edges: Vec<u32>,
}

struct NodeRow {
    path: String,
    relative_key: String,
    title_key: String,
    created_ns: i64,
    modified_ns: i64,
}

impl RenderSnapshot {
    pub fn build(connection: &Connection, root: &Path) -> Result<Self, SearchError> {
        let nodes = read_nodes(connection)?;
        let directed = read_directed_edges(connection, root, &nodes)?;
        let links = rank::out_links(nodes.len(), &directed);
        let ranks = rank::pagerank(nodes.len(), &links);
        let render_of_identity = render_order(&ranks);
        let edges = ordered_edges(&directed, &render_of_identity);
        let degrees = degrees(nodes.len(), &edges);
        let positions = layout::compute(nodes.len(), &edges, LAYOUT_REPULSION);

        let mut paths = vec![String::new(); nodes.len()];
        let mut created_days = vec![f32::NAN; nodes.len()];
        let mut modified_days = vec![f32::NAN; nodes.len()];
        for (identity, node) in nodes.into_iter().enumerate() {
            let slot = render_of_identity[identity] as usize;
            created_days[slot] = days(node.created_ns);
            modified_days[slot] = days(node.modified_ns);
            paths[slot] = node.path;
        }

        Ok(Self {
            paths,
            positions,
            created_days,
            modified_days,
            degrees,
            edges,
        })
    }

    pub fn node_count(&self) -> usize {
        self.paths.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len() / 2
    }
}

pub fn paths_at(paths: &[String], indices: &[u32]) -> Vec<String> {
    indices
        .iter()
        .map(|index| paths.get(*index as usize).cloned().unwrap_or_default())
        .collect()
}

fn read_nodes(connection: &Connection) -> Result<Vec<NodeRow>, SearchError> {
    let mut statement = connection.prepare(
        "SELECT wiki_documents.path, wiki_documents.relative_key, wiki_documents.title_key,
                COALESCE(documents.created_ns, 0), COALESCE(documents.modified_ns, 0)
         FROM wiki_documents LEFT JOIN documents ON documents.path = wiki_documents.path
         ORDER BY wiki_documents.path",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(NodeRow {
            path: row.get(0)?,
            relative_key: row.get(1)?,
            title_key: row.get(2)?,
            created_ns: row.get(3)?,
            modified_ns: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn read_directed_edges(
    connection: &Connection,
    root: &Path,
    nodes: &[NodeRow],
) -> Result<Vec<(u32, u32)>, SearchError> {
    let mut identity_of_path = HashMap::<&str, u32>::with_capacity(nodes.len());
    for (identity, node) in nodes.iter().enumerate() {
        identity_of_path.insert(node.path.as_str(), identity as u32);
    }
    let mut directed = Vec::new();
    wiki::for_each_link(
        connection,
        root,
        nodes.iter().map(|node| {
            (
                node.path.as_str(),
                node.relative_key.as_str(),
                node.title_key.as_str(),
            )
        }),
        |source, target| {
            let Some(from) = identity_of_path.get(source).copied() else {
                return;
            };
            let Some(to) = identity_of_path.get(target).copied() else {
                return;
            };
            if from != to {
                directed.push((from, to));
            }
        },
    )?;
    directed.sort_unstable();
    directed.dedup();
    Ok(directed)
}

fn render_order(ranks: &[f32]) -> Vec<u32> {
    let mut identities = (0..ranks.len() as u32).collect::<Vec<_>>();
    identities.sort_unstable_by(|left, right| {
        ranks[*right as usize]
            .total_cmp(&ranks[*left as usize])
            .then_with(|| left.cmp(right))
    });
    let mut render_of_identity = vec![0u32; ranks.len()];
    for (slot, identity) in identities.into_iter().enumerate() {
        render_of_identity[identity as usize] = slot as u32;
    }
    render_of_identity
}

fn ordered_edges(directed: &[(u32, u32)], render_of_identity: &[u32]) -> Vec<u32> {
    let mut pairs = directed
        .iter()
        .map(|(source, target)| {
            let source = render_of_identity[*source as usize];
            let target = render_of_identity[*target as usize];
            (source.min(target), source.max(target))
        })
        .collect::<Vec<_>>();
    pairs.sort_unstable();
    pairs.dedup();
    let mut edges = Vec::with_capacity(pairs.len() * 2);
    for (left, right) in pairs {
        edges.push(left);
        edges.push(right);
    }
    edges
}

fn degrees(node_count: usize, edges: &[u32]) -> Vec<u32> {
    let mut degrees = vec![0u32; node_count];
    for endpoint in edges {
        degrees[*endpoint as usize] += 1;
    }
    degrees
}

fn days(nanos: i64) -> f32 {
    if nanos <= 0 {
        return f32::NAN;
    }
    (nanos as f64 / NANOS_PER_DAY) as f32
}

#[cfg(test)]
mod tests {
    use super::{paths_at, RenderSnapshot};
    use rusqlite::Connection;
    use std::path::Path;

    fn vault() -> (Connection, &'static Path) {
        let root = Path::new("C:/vault");
        let mut connection = crate::search::metadata::memory();
        let transaction = connection.transaction().expect("transaction");
        for (name, body) in [
            ("Hub", "[[Leaf]] and [[Other]]"),
            ("Leaf", "[[Hub]]"),
            ("Other", ""),
            ("Lonely", "no links at all"),
        ] {
            let path = root.join(format!("{name}.md"));
            crate::search::index_document::index_text(&transaction, root, &path, body).expect("wiki row");
            transaction
                .execute(
                    "INSERT INTO documents(path, modified_ns, size, created_ns) VALUES(?1, 200, 10, 100)",
                    [path.to_string_lossy().as_ref()],
                )
                .expect("document row");
        }
        transaction.commit().expect("commit");
        (connection, root)
    }

    #[test]
    fn notes_without_links_stay_in_the_snapshot() {
        let (connection, root) = vault();

        let snapshot = RenderSnapshot::build(&connection, root).expect("snapshot");

        assert_eq!(snapshot.node_count(), 4);
        assert!(
            paths_at(&snapshot.paths, &(0..4).collect::<Vec<_>>())
                .iter()
                .any(|path| path.ends_with("Lonely.md"))
        );
    }

    #[test]
    fn mutual_links_collapse_into_a_single_edge() {
        let (connection, root) = vault();

        let snapshot = RenderSnapshot::build(&connection, root).expect("snapshot");

        assert_eq!(snapshot.edge_count(), 2);
    }

    #[test]
    fn the_most_linked_note_takes_the_first_slot() {
        let (connection, root) = vault();

        let snapshot = RenderSnapshot::build(&connection, root).expect("snapshot");

        assert!(paths_at(&snapshot.paths, &[0])[0].ends_with("Hub.md"));
    }

    #[test]
    fn edges_are_ordered_by_their_most_important_endpoint() {
        let (connection, root) = vault();

        let snapshot = RenderSnapshot::build(&connection, root).expect("snapshot");
        let leading = snapshot
            .edges
            .chunks_exact(2)
            .map(|pair| pair[0])
            .collect::<Vec<_>>();
        let mut sorted = leading.clone();
        sorted.sort_unstable();

        assert_eq!(leading, sorted);
        assert!(snapshot.edges.chunks_exact(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn every_node_gets_a_finite_position_and_a_degree() {
        let (connection, root) = vault();

        let snapshot = RenderSnapshot::build(&connection, root).expect("snapshot");

        assert_eq!(snapshot.positions.len(), snapshot.node_count() * 2);
        assert_eq!(snapshot.degrees.len(), snapshot.node_count());
        assert!(snapshot.positions.iter().all(|value| value.is_finite()));
        assert_eq!(snapshot.degrees.iter().sum::<u32>(), 4);
    }

    #[test]
    fn dates_reach_the_snapshot_as_days() {
        let (connection, root) = vault();

        let snapshot = RenderSnapshot::build(&connection, root).expect("snapshot");

        assert!(snapshot.created_days.iter().all(|value| *value > 0.0));
        assert!(snapshot.modified_days.iter().all(|value| *value > 0.0));
    }

    #[test]
    fn an_unindexed_workspace_yields_an_empty_snapshot() {
        let connection = crate::search::metadata::memory();

        let snapshot =
            RenderSnapshot::build(&connection, Path::new("C:/vault")).expect("snapshot");

        assert_eq!(snapshot.node_count(), 0);
        assert_eq!(snapshot.edge_count(), 0);
        assert!(snapshot.positions.is_empty());
    }
}
