use std::collections::{HashMap, HashSet, VecDeque};

use nf_core::entities::EntityId;
use nf_core::relationships::RelationshipType;
use petgraph::algo::connected_components;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;

/// Directed graph over EntityIds with typed RelationshipType edges.
pub struct NfGraph {
    pub(crate) graph: DiGraph<EntityId, RelationshipType>,
    pub(crate) node_map: HashMap<EntityId, NodeIndex>,
}

impl NfGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Insert a node for `id`, returning its index. No-op if already present.
    pub fn add_node(&mut self, id: EntityId) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(&id) {
            return idx;
        }
        let idx = self.graph.add_node(id);
        self.node_map.insert(id, idx);
        idx
    }

    /// Add a directed edge `from → to` with the given relationship type.
    /// Nodes are auto-inserted if not already present.
    pub fn add_edge(&mut self, from: EntityId, to: EntityId, rel: RelationshipType) {
        let from_idx = self.add_node(from);
        let to_idx = self.add_node(to);
        self.graph.add_edge(from_idx, to_idx, rel);
    }

    /// Return all outgoing neighbors of `id`.
    pub fn neighbors(&self, id: EntityId) -> Vec<EntityId> {
        match self.node_map.get(&id) {
            Some(&idx) => self
                .graph
                .neighbors(idx)
                .map(|n| *self.graph.node_weight(n).unwrap())
                .collect(),
            None => Vec::new(),
        }
    }

    /// Shortest directed path from `from` to `to` using BFS (equivalent to
    /// Dijkstra with uniform edge weights).  Returns `None` if no path exists.
    pub fn shortest_path(&self, from: EntityId, to: EntityId) -> Option<Vec<EntityId>> {
        let &from_idx = self.node_map.get(&from)?;
        let &to_idx = self.node_map.get(&to)?;

        if from_idx == to_idx {
            return Some(vec![from]);
        }

        let mut parent: HashMap<NodeIndex, NodeIndex> = HashMap::new();
        let mut visited: HashSet<NodeIndex> = HashSet::new();
        let mut queue: VecDeque<NodeIndex> = VecDeque::new();

        queue.push_back(from_idx);
        visited.insert(from_idx);

        let mut found = false;
        'outer: while let Some(current) = queue.pop_front() {
            for neighbor in self.graph.neighbors(current) {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent.insert(neighbor, current);
                    if neighbor == to_idx {
                        found = true;
                        break 'outer;
                    }
                    queue.push_back(neighbor);
                }
            }
        }

        if !found {
            return None;
        }

        let mut path = Vec::new();
        let mut current = to_idx;
        loop {
            path.push(*self.graph.node_weight(current).unwrap());
            if current == from_idx {
                break;
            }
            current = *parent.get(&current)?;
        }
        path.reverse();
        Some(path)
    }

    /// Number of weakly connected components.
    pub fn connected_components(&self) -> usize {
        connected_components(&self.graph)
    }

    /// Degree centrality = (in-degree + out-degree) / (N - 1).
    pub fn degree_centrality(&self, id: EntityId) -> f64 {
        let &idx = match self.node_map.get(&id) {
            Some(i) => i,
            None => return 0.0,
        };
        let n = self.graph.node_count();
        if n <= 1 {
            return 0.0;
        }
        let out = self.graph.edges_directed(idx, Direction::Outgoing).count();
        let in_ = self.graph.edges_directed(idx, Direction::Incoming).count();
        (out + in_) as f64 / (n - 1) as f64
    }

    /// Access the underlying petgraph DiGraph.
    pub fn inner(&self) -> &DiGraph<EntityId, RelationshipType> {
        &self.graph
    }

    /// Map from EntityId to NodeIndex for external use (e.g., community detection).
    pub fn entity_map(&self) -> &HashMap<EntityId, NodeIndex> {
        &self.node_map
    }
}

impl Default for NfGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nf_core::relationships::RelationshipType;

    #[test]
    fn test_add_node_idempotent() {
        let mut g = NfGraph::new();
        let id = EntityId::new();
        let idx1 = g.add_node(id);
        let idx2 = g.add_node(id);
        assert_eq!(idx1, idx2);
        assert_eq!(g.inner().node_count(), 1);
    }

    #[test]
    fn test_add_edge_auto_creates_nodes() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        assert_eq!(g.inner().node_count(), 2);
        assert_eq!(g.inner().edge_count(), 1);
    }

    #[test]
    fn test_neighbors() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(a, c, RelationshipType::Pardoned);

        let mut ns = g.neighbors(a);
        ns.sort_by_key(|e| e.0);
        let mut expected = vec![b, c];
        expected.sort_by_key(|e| e.0);
        assert_eq!(ns, expected);

        // Node with no outgoing edges returns empty
        assert!(g.neighbors(b).is_empty());
    }

    #[test]
    fn test_shortest_path_direct() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        let path = g.shortest_path(a, b).unwrap();
        assert_eq!(path, vec![a, b]);
    }

    #[test]
    fn test_shortest_path_indirect() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(b, c, RelationshipType::Pardoned);
        let path = g.shortest_path(a, c).unwrap();
        assert_eq!(path, vec![a, b, c]);
    }

    #[test]
    fn test_shortest_path_same_node() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        g.add_node(a);
        let path = g.shortest_path(a, a).unwrap();
        assert_eq!(path, vec![a]);
    }

    #[test]
    fn test_shortest_path_no_path() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_node(a);
        g.add_node(b);
        // No edge between a and b
        assert!(g.shortest_path(a, b).is_none());
    }

    #[test]
    fn test_shortest_path_unknown_node() {
        let g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        assert!(g.shortest_path(a, b).is_none());
    }

    #[test]
    fn test_connected_components_two_clusters() {
        let mut g = NfGraph::new();
        // Cluster 1
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        // Cluster 2 (disconnected)
        let c = EntityId::new();
        let d = EntityId::new();
        g.add_edge(c, d, RelationshipType::Pardoned);

        assert_eq!(g.connected_components(), 2);
    }

    #[test]
    fn test_connected_components_single() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(b, c, RelationshipType::Pardoned);
        assert_eq!(g.connected_components(), 1);
    }

    #[test]
    fn test_degree_centrality() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(a, c, RelationshipType::Pardoned);
        // a has out=2, in=0 → (2+0)/(3-1) = 1.0
        assert!((g.degree_centrality(a) - 1.0).abs() < 1e-9);
        // b has out=0, in=1 → (0+1)/(3-1) = 0.5
        assert!((g.degree_centrality(b) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_degree_centrality_unknown_node() {
        let g = NfGraph::new();
        assert_eq!(g.degree_centrality(EntityId::new()), 0.0);
    }

    #[test]
    fn test_degree_centrality_single_node() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        g.add_node(a);
        assert_eq!(g.degree_centrality(a), 0.0);
    }
}
