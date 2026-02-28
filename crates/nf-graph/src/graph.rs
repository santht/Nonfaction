use std::collections::{HashMap, HashSet, VecDeque};

use nf_core::entities::EntityId;
use nf_core::relationships::RelationshipType;
use petgraph::Direction;
use petgraph::algo::connected_components;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;

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

    /// Out-degree (number of outgoing edges) for `id`.
    pub fn out_degree(&self, id: EntityId) -> usize {
        match self.node_map.get(&id) {
            Some(&idx) => self.graph.edges_directed(idx, Direction::Outgoing).count(),
            None => 0,
        }
    }

    /// In-degree (number of incoming edges) for `id`.
    pub fn in_degree(&self, id: EntityId) -> usize {
        match self.node_map.get(&id) {
            Some(&idx) => self.graph.edges_directed(idx, Direction::Incoming).count(),
            None => 0,
        }
    }

    /// Extract the induced directed subgraph reachable from `center_id` within
    /// `depth` outgoing BFS hops.
    pub fn subgraph(&self, center_id: EntityId, depth: usize) -> NfGraph {
        let &center_idx = match self.node_map.get(&center_id) {
            Some(idx) => idx,
            None => return NfGraph::new(),
        };

        let mut included: HashSet<NodeIndex> = HashSet::new();
        let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();
        included.insert(center_idx);
        queue.push_back((center_idx, 0));

        while let Some((current, dist)) = queue.pop_front() {
            if dist >= depth {
                continue;
            }
            for neighbor in self.graph.neighbors(current) {
                if included.insert(neighbor) {
                    queue.push_back((neighbor, dist + 1));
                }
            }
        }

        let mut result = NfGraph::new();
        for idx in &included {
            let id = *self.graph.node_weight(*idx).unwrap();
            result.add_node(id);
        }

        for edge in self.graph.edge_references() {
            let source = edge.source();
            let target = edge.target();
            if included.contains(&source) && included.contains(&target) {
                let source_id = *self.graph.node_weight(source).unwrap();
                let target_id = *self.graph.node_weight(target).unwrap();
                result.add_edge(source_id, target_id, *edge.weight());
            }
        }

        result
    }

    /// Find all simple directed paths from `from` to `to` with at most
    /// `max_length` edges.
    pub fn all_paths_up_to_length(
        &self,
        from: EntityId,
        to: EntityId,
        max_length: usize,
    ) -> Vec<Vec<EntityId>> {
        let &from_idx = match self.node_map.get(&from) {
            Some(idx) => idx,
            None => return Vec::new(),
        };
        let &to_idx = match self.node_map.get(&to) {
            Some(idx) => idx,
            None => return Vec::new(),
        };

        let mut results = Vec::new();
        let mut path = vec![from_idx];
        let mut visited = HashSet::new();
        visited.insert(from_idx);

        self.collect_paths(
            from_idx,
            to_idx,
            max_length,
            &mut path,
            &mut visited,
            &mut results,
        );

        results
            .into_iter()
            .map(|indices| {
                indices
                    .into_iter()
                    .map(|idx| *self.graph.node_weight(idx).unwrap())
                    .collect()
            })
            .collect()
    }

    fn collect_paths(
        &self,
        current: NodeIndex,
        target: NodeIndex,
        remaining: usize,
        path: &mut Vec<NodeIndex>,
        visited: &mut HashSet<NodeIndex>,
        results: &mut Vec<Vec<NodeIndex>>,
    ) {
        if current == target {
            results.push(path.clone());
            return;
        }
        if remaining == 0 {
            return;
        }

        for next in self.graph.neighbors(current) {
            if !visited.insert(next) {
                continue;
            }
            path.push(next);
            self.collect_paths(next, target, remaining - 1, path, visited, results);
            path.pop();
            visited.remove(&next);
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

    /// Degree centrality for every node in the graph.
    pub fn degree_centrality_all(&self) -> HashMap<EntityId, f64> {
        self.node_map
            .keys()
            .copied()
            .map(|id| (id, self.degree_centrality(id)))
            .collect()
    }

    /// Betweenness centrality over directed shortest paths (Brandes algorithm).
    ///
    /// Returned scores are normalized to `[0, 1]` by dividing by
    /// `(N - 1) * (N - 2)` for `N >= 3`.
    pub fn betweenness_centrality(&self) -> HashMap<EntityId, f64> {
        let n = self.graph.node_count();
        let mut cb: HashMap<NodeIndex, f64> =
            self.graph.node_indices().map(|idx| (idx, 0.0)).collect();

        if n < 3 {
            return self
                .graph
                .node_indices()
                .map(|idx| (*self.graph.node_weight(idx).unwrap(), 0.0))
                .collect();
        }

        for source in self.graph.node_indices() {
            let mut stack: Vec<NodeIndex> = Vec::new();
            let mut predecessors: HashMap<NodeIndex, Vec<NodeIndex>> = self
                .graph
                .node_indices()
                .map(|idx| (idx, Vec::new()))
                .collect();
            let mut sigma: HashMap<NodeIndex, f64> =
                self.graph.node_indices().map(|idx| (idx, 0.0)).collect();
            let mut distance: HashMap<NodeIndex, i64> =
                self.graph.node_indices().map(|idx| (idx, -1)).collect();

            sigma.insert(source, 1.0);
            distance.insert(source, 0);

            let mut queue: VecDeque<NodeIndex> = VecDeque::new();
            queue.push_back(source);

            while let Some(v) = queue.pop_front() {
                stack.push(v);
                let v_dist = *distance.get(&v).unwrap();

                for w in self.graph.neighbors(v) {
                    if *distance.get(&w).unwrap() < 0 {
                        queue.push_back(w);
                        distance.insert(w, v_dist + 1);
                    }
                    if *distance.get(&w).unwrap() == v_dist + 1 {
                        let sigma_w = sigma.get(&w).copied().unwrap_or(0.0);
                        let sigma_v = sigma.get(&v).copied().unwrap_or(0.0);
                        sigma.insert(w, sigma_w + sigma_v);
                        predecessors.get_mut(&w).unwrap().push(v);
                    }
                }
            }

            let mut dependency: HashMap<NodeIndex, f64> =
                self.graph.node_indices().map(|idx| (idx, 0.0)).collect();

            while let Some(w) = stack.pop() {
                let sigma_w = sigma.get(&w).copied().unwrap_or(0.0);
                if sigma_w == 0.0 {
                    continue;
                }

                for v in predecessors.get(&w).unwrap() {
                    let sigma_v = sigma.get(v).copied().unwrap_or(0.0);
                    let delta_w = dependency.get(&w).copied().unwrap_or(0.0);
                    let contribution = (sigma_v / sigma_w) * (1.0 + delta_w);
                    let delta_v = dependency.get(v).copied().unwrap_or(0.0);
                    dependency.insert(*v, delta_v + contribution);
                }

                if w != source {
                    let score = cb.get(&w).copied().unwrap_or(0.0);
                    cb.insert(w, score + dependency.get(&w).copied().unwrap_or(0.0));
                }
            }
        }

        let norm = ((n - 1) * (n - 2)) as f64;
        self.graph
            .node_indices()
            .map(|idx| {
                let id = *self.graph.node_weight(idx).unwrap();
                let normalized = cb.get(&idx).copied().unwrap_or(0.0) / norm;
                (id, normalized)
            })
            .collect()
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
    fn test_out_degree_and_in_degree() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(a, c, RelationshipType::DonatedTo);
        g.add_edge(c, a, RelationshipType::Pardoned);

        assert_eq!(g.out_degree(a), 2);
        assert_eq!(g.in_degree(a), 1);
        assert_eq!(g.out_degree(b), 0);
        assert_eq!(g.in_degree(b), 1);
    }

    #[test]
    fn test_degree_methods_unknown_node() {
        let g = NfGraph::new();
        let id = EntityId::new();
        assert_eq!(g.out_degree(id), 0);
        assert_eq!(g.in_degree(id), 0);
    }

    #[test]
    fn test_subgraph_depth_one() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        let d = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(b, c, RelationshipType::DonatedTo);
        g.add_edge(a, d, RelationshipType::Pardoned);

        let sub = g.subgraph(a, 1);
        assert_eq!(sub.inner().node_count(), 3);
        assert_eq!(sub.inner().edge_count(), 2);
        assert!(!sub.neighbors(a).is_empty());
        assert!(sub.neighbors(b).is_empty());
    }

    #[test]
    fn test_subgraph_unknown_center_returns_empty() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        g.add_node(a);
        let sub = g.subgraph(EntityId::new(), 2);
        assert_eq!(sub.inner().node_count(), 0);
        assert_eq!(sub.inner().edge_count(), 0);
    }

    #[test]
    fn test_all_paths_up_to_length_multiple_paths() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        let d = EntityId::new();

        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(b, d, RelationshipType::DonatedTo);
        g.add_edge(a, c, RelationshipType::DonatedTo);
        g.add_edge(c, d, RelationshipType::DonatedTo);

        let paths = g.all_paths_up_to_length(a, d, 2);
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&vec![a, b, d]));
        assert!(paths.contains(&vec![a, c, d]));
    }

    #[test]
    fn test_all_paths_up_to_length_respects_max_length() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();

        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(b, c, RelationshipType::DonatedTo);

        assert!(g.all_paths_up_to_length(a, c, 1).is_empty());
        let paths = g.all_paths_up_to_length(a, c, 2);
        assert_eq!(paths, vec![vec![a, b, c]]);
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

    #[test]
    fn test_degree_centrality_all() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(a, c, RelationshipType::DonatedTo);

        let scores = g.degree_centrality_all();
        assert_eq!(scores.len(), 3);
        assert!((scores.get(&a).copied().unwrap_or_default() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_betweenness_centrality_chain() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(b, c, RelationshipType::DonatedTo);

        let bc = g.betweenness_centrality();
        assert_eq!(bc.len(), 3);
        assert!(bc.get(&b).copied().unwrap_or_default() > 0.0);
        assert_eq!(bc.get(&a).copied().unwrap_or_default(), 0.0);
        assert_eq!(bc.get(&c).copied().unwrap_or_default(), 0.0);
    }

    #[test]
    fn test_betweenness_centrality_disconnected() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_node(a);
        g.add_node(b);

        let bc = g.betweenness_centrality();
        assert_eq!(bc.get(&a).copied().unwrap_or_default(), 0.0);
        assert_eq!(bc.get(&b).copied().unwrap_or_default(), 0.0);
    }
}
