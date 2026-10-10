pub struct Rows {
    nodes: Vec<u32>,
    node_y: Vec<f32>,
    edges: Vec<u32>,
    edge_y: Vec<f32>,
    pub reach: f32,
    pub long: Vec<u32>,
}

const LONG_SHARE: f64 = 0.01;

impl Rows {
    pub fn new(positions: &[[f32; 2]], edges: &[[u32; 2]]) -> Rows {
        let mut nodes: Vec<u32> = (0..positions.len() as u32).collect();
        nodes.sort_unstable_by(|&a, &b| positions[a as usize][1].total_cmp(&positions[b as usize][1]));
        let node_y = nodes.iter().map(|&n| positions[n as usize][1]).collect();
        let span = |&[a, b]: &[u32; 2]| (positions[a as usize][1] - positions[b as usize][1]).abs();
        let mut extents: Vec<f32> = edges.iter().map(span).collect();
        extents.sort_unstable_by(f32::total_cmp);
        let cut = (extents.len().saturating_sub(1) as f64 * (1.0 - LONG_SHARE)) as usize;
        let reach = extents.get(cut).copied().unwrap_or(0.0);
        let (mut short, mut long) = (Vec::new(), Vec::new());
        for (i, edge) in edges.iter().enumerate() {
            if span(edge) <= reach { short.push(i as u32) } else { long.push(i as u32) }
        }
        let low = |i: u32| {
            let [a, b] = edges[i as usize];
            positions[a as usize][1].min(positions[b as usize][1])
        };
        short.sort_unstable_by(|&a, &b| low(a).total_cmp(&low(b)));
        let edge_y = short.iter().map(|&i| low(i)).collect();
        Rows { nodes, node_y, edges: short, edge_y, reach, long }
    }

    pub fn nodes(&self, low: f32, high: f32) -> &[u32] {
        let from = self.node_y.partition_point(|&y| y < low);
        let to = self.node_y.partition_point(|&y| y <= high);
        &self.nodes[from..to.max(from)]
    }

    pub fn edges(&self, low: f32, high: f32) -> &[u32] {
        let from = self.edge_y.partition_point(|&y| y < low - self.reach);
        let to = self.edge_y.partition_point(|&y| y <= high);
        &self.edges[from..to.max(from)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_nodes_and_edges_by_height() {
        let positions = [[0.0, 0.0], [0.0, 1.0], [0.0, 2.0], [0.0, 10.0], [0.0, -10.0]];
        let edges = [[0, 1], [1, 2], [2, 0], [1, 0], [3, 4]];
        let rows = Rows::new(&positions, &edges);
        assert_eq!(rows.nodes(0.5, 2.0), [1, 2]);
        assert_eq!(rows.long, [4]);
        let found = rows.edges(1.5, 1.7);
        assert!(found.contains(&1) && found.contains(&2) && !found.contains(&4));
        assert!(rows.edges(5.0, 6.0).is_empty());
    }
}
