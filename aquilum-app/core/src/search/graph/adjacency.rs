pub struct Adjacency {
    offsets: Vec<u32>,
    targets: Vec<u32>,
}

impl Adjacency {
    pub fn directed(node_count: usize, pairs: impl Iterator<Item = (u32, u32)> + Clone) -> Self {
        let mut offsets = vec![0u32; node_count + 1];
        for (source, _) in pairs.clone() {
            offsets[source as usize + 1] += 1;
        }
        for node in 0..node_count {
            offsets[node + 1] += offsets[node];
        }
        let mut cursor = offsets.clone();
        let mut targets = vec![0u32; offsets[node_count] as usize];
        for (source, target) in pairs {
            let slot = &mut cursor[source as usize];
            targets[*slot as usize] = target;
            *slot += 1;
        }
        Self { offsets, targets }
    }

    pub fn build(node_count: usize, edges: &[u32]) -> Self {
        Self::directed(node_count, edges.chunks_exact(2).flat_map(|pair| [(pair[0], pair[1]), (pair[1], pair[0])]))
    }

    pub fn neighbours(&self, node: u32) -> &[u32] {
        let start = self.offsets[node as usize] as usize;
        let end = self.offsets[node as usize + 1] as usize;
        &self.targets[start..end]
    }
}

pub fn components(node_count: usize, edges: &[u32]) -> Vec<u32> {
    let mut parent: Vec<u32> = (0..node_count as u32).collect();
    for pair in edges.chunks_exact(2) {
        let left = find(&mut parent, pair[0]);
        let right = find(&mut parent, pair[1]);
        if left != right {
            parent[left.max(right) as usize] = left.min(right);
        }
    }
    (0..node_count as u32)
        .map(|node| find(&mut parent, node))
        .collect()
}

fn find(parent: &mut [u32], node: u32) -> u32 {
    let mut node = node;
    while parent[node as usize] != node {
        let grandparent = parent[parent[node as usize] as usize];
        parent[node as usize] = grandparent;
        node = grandparent;
    }
    node
}

#[cfg(test)]
mod tests {
    use super::{components, Adjacency};

    #[test]
    fn neighbours_are_symmetric() {
        let adjacency = Adjacency::build(3, &[0, 1, 1, 2]);

        assert_eq!(adjacency.neighbours(0), [1]);
        assert_eq!(adjacency.neighbours(1).len(), 2);
        assert_eq!(adjacency.neighbours(2), [1]);
    }

    #[test]
    fn isolated_notes_keep_their_own_component() {
        let labels = components(5, &[0, 1, 1, 2]);

        assert_eq!(labels[0], labels[2]);
        assert_ne!(labels[0], labels[3]);
        assert_ne!(labels[3], labels[4]);
    }

    #[test]
    fn component_label_is_the_smallest_member() {
        let labels = components(3, &[2, 1, 1, 0]);

        assert_eq!(labels, vec![0, 0, 0]);
    }
}
