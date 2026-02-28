use std::collections::{HashMap, HashSet, VecDeque};

use nf_core::entities::EntityId;
use petgraph::Direction;
use petgraph::graph::NodeIndex;

use crate::graph::NfGraph;

/// Run label propagation community detection on `graph`.
///
/// Each node starts with its own unique label (its NodeIndex as `u32`).
/// On each iteration every node adopts the most common label among its
/// neighbors (both incoming and outgoing edges are considered). Iteration
/// stops when labels stabilise or after 100 rounds.
///
/// Returns a map from community label → member EntityIds.
pub fn label_propagation(nf_graph: &NfGraph) -> HashMap<u32, Vec<EntityId>> {
    let graph = nf_graph.inner();
    let node_count = graph.node_count();

    if node_count == 0 {
        return HashMap::new();
    }

    let node_indices: Vec<NodeIndex> = graph.node_indices().collect();

    // Initialise: each node has a unique label equal to its index.
    let mut labels: Vec<u32> = (0..node_count as u32).collect();

    for _ in 0..100 {
        let mut changed = false;

        for &node in &node_indices {
            let mut freq: HashMap<u32, usize> = HashMap::new();

            for nb in graph.neighbors_directed(node, Direction::Outgoing) {
                *freq.entry(labels[nb.index()]).or_insert(0) += 1;
            }
            for nb in graph.neighbors_directed(node, Direction::Incoming) {
                *freq.entry(labels[nb.index()]).or_insert(0) += 1;
            }

            if freq.is_empty() {
                continue; // isolated node keeps its own label
            }

            // Pick most frequent label; break ties by smallest label value for determinism.
            let best = freq
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
                .map(|(label, _)| label)
                .unwrap();

            if labels[node.index()] != best {
                labels[node.index()] = best;
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    // Group EntityIds by final label.
    let mut communities: HashMap<u32, Vec<EntityId>> = HashMap::new();
    for &node in &node_indices {
        let entity_id = *graph.node_weight(node).unwrap();
        let label = labels[node.index()];
        communities.entry(label).or_default().push(entity_id);
    }

    communities
}

/// Detect communities using weakly connected components.
///
/// Returns a map from component index to member EntityIds.
pub fn connected_component_communities(nf_graph: &NfGraph) -> HashMap<usize, Vec<EntityId>> {
    let graph = nf_graph.inner();
    let mut visited: HashSet<NodeIndex> = HashSet::new();
    let mut communities: HashMap<usize, Vec<EntityId>> = HashMap::new();
    let mut component_id = 0usize;

    for start in graph.node_indices() {
        if visited.contains(&start) {
            continue;
        }

        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited.insert(start);
        let mut members = Vec::new();

        while let Some(node) = queue.pop_front() {
            members.push(*graph.node_weight(node).unwrap());

            for nb in graph.neighbors_directed(node, Direction::Outgoing) {
                if visited.insert(nb) {
                    queue.push_back(nb);
                }
            }
            for nb in graph.neighbors_directed(node, Direction::Incoming) {
                if visited.insert(nb) {
                    queue.push_back(nb);
                }
            }
        }

        communities.insert(component_id, members);
        component_id += 1;
    }

    communities
}

#[cfg(test)]
mod tests {
    use super::*;
    use nf_core::entities::EntityId;
    use nf_core::relationships::RelationshipType;

    #[test]
    fn test_empty_graph() {
        let g = NfGraph::new();
        assert!(label_propagation(&g).is_empty());
    }

    #[test]
    fn test_single_node() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        g.add_node(a);
        let communities = label_propagation(&g);
        // One community containing just a
        assert_eq!(communities.values().map(|v| v.len()).sum::<usize>(), 1);
    }

    #[test]
    fn test_two_disconnected_clusters() {
        let mut g = NfGraph::new();
        // Cluster 1: a ↔ b
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(b, a, RelationshipType::DonatedTo);

        // Cluster 2: c ↔ d
        let c = EntityId::new();
        let d = EntityId::new();
        g.add_edge(c, d, RelationshipType::Pardoned);
        g.add_edge(d, c, RelationshipType::Pardoned);

        let communities = label_propagation(&g);
        // Should produce exactly 2 communities
        assert_eq!(communities.len(), 2);
        // Each community has 2 members
        for members in communities.values() {
            assert_eq!(members.len(), 2);
        }
    }

    #[test]
    fn test_single_connected_component() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(b, c, RelationshipType::DonatedTo);
        g.add_edge(c, a, RelationshipType::DonatedTo);

        let communities = label_propagation(&g);
        // All nodes end up in one community
        assert_eq!(communities.len(), 1);
        let members: &Vec<EntityId> = communities.values().next().unwrap();
        assert_eq!(members.len(), 3);
    }

    #[test]
    fn test_total_members_equals_node_count() {
        let mut g = NfGraph::new();
        let ids: Vec<EntityId> = (0..6).map(|_| EntityId::new()).collect();
        // Chain: 0→1→2, isolated: 3,4,5
        g.add_edge(ids[0], ids[1], RelationshipType::DonatedTo);
        g.add_edge(ids[1], ids[2], RelationshipType::DonatedTo);
        g.add_node(ids[3]);
        g.add_node(ids[4]);
        g.add_node(ids[5]);

        let communities = label_propagation(&g);
        let total: usize = communities.values().map(|v| v.len()).sum();
        assert_eq!(total, 6);
    }

    #[test]
    fn test_connected_component_communities() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_node(c);

        let communities = connected_component_communities(&g);
        assert_eq!(communities.len(), 2);
        let mut sizes: Vec<usize> = communities.values().map(|v| v.len()).collect();
        sizes.sort_unstable();
        assert_eq!(sizes, vec![1, 2]);
    }
}
