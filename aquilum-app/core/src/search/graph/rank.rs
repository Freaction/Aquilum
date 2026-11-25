use super::adjacency::Adjacency;

const ITERATIONS: usize = 20;
const DAMPING: f64 = 0.85;

pub fn pagerank(node_count: usize, links: &Adjacency) -> Vec<f32> {
    if node_count == 0 {
        return Vec::new();
    }
    let count = node_count as f64;
    let mut rank = vec![1.0 / count; node_count];
    let mut received = vec![0.0; node_count];
    for _ in 0..ITERATIONS {
        received.fill(0.0);
        let mut dangling = 0.0;
        for (node, current) in rank.iter().enumerate() {
            let targets = links.neighbours(node as u32);
            if targets.is_empty() {
                dangling += current;
                continue;
            }
            let share = current / targets.len() as f64;
            for target in targets {
                received[*target as usize] += share;
            }
        }
        let base = (1.0 - DAMPING) / count + DAMPING * dangling / count;
        for (value, incoming) in rank.iter_mut().zip(&received) {
            *value = base + DAMPING * incoming;
        }
    }
    rank.into_iter().map(|value| value as f32).collect()
}

#[cfg(test)]
mod tests {
    use super::{pagerank, Adjacency};

    #[test]
    fn a_linked_note_outranks_the_notes_linking_to_it() {
        let directed = [(1, 0), (2, 0), (3, 0)];
        let links = Adjacency::directed(4, directed.into_iter());

        let rank = pagerank(4, &links);

        assert!(rank[0] > rank[1]);
        assert!((rank[1] - rank[3]).abs() < 1e-9);
    }

    #[test]
    fn ranks_sum_to_one_even_with_dangling_notes() {
        let links = Adjacency::directed(3, [(0, 1)].into_iter());

        let total: f32 = pagerank(3, &links).iter().sum();

        assert!((total - 1.0).abs() < 1e-4);
    }

    #[test]
    fn an_empty_graph_yields_no_ranks() {
        assert!(pagerank(0, &Adjacency::directed(0, std::iter::empty())).is_empty());
    }
}
