// Dense cluster detection and bridge entity (articulation point) identification.
// Bridge entities are "kingpins" whose removal would fragment the network.

use std::collections::HashSet;

use nf_core::entities::EntityId;
use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;

use crate::graph::NfGraph;

/// A densely interconnected cluster of entities within the graph.
#[derive(Debug, Clone)]
pub struct EntityCluster {
    /// Members of this cluster.
    pub members: Vec<EntityId>,
    /// Number of directed edges whose both endpoints are within this cluster.
    pub internal_edges: usize,
    /// Graph density: `2 * internal_edges / (n * (n - 1))`.
    /// Ranges from 0 (no edges) to 1+ for directed graphs.
    pub density: f64,
}

/// Partition the graph into weakly-connected components and return those with
/// at least `min_size` members, annotated with internal edge count and density.
pub fn find_dense_clusters(graph: &NfGraph, min_size: usize) -> Vec<EntityCluster> {
    let inner = graph.inner();
    let components = graph.component_groups();

    components
        .into_iter()
        .filter(|members| members.len() >= min_size)
        .map(|members| {
            let member_set: HashSet<EntityId> = members.iter().copied().collect();

            let internal_edges = inner
                .edge_references()
                .filter(|e| {
                    let src = *inner.node_weight(e.source()).unwrap();
                    let tgt = *inner.node_weight(e.target()).unwrap();
                    member_set.contains(&src) && member_set.contains(&tgt)
                })
                .count();

            let n = members.len();
            let density = if n <= 1 {
                0.0
            } else {
                (2 * internal_edges) as f64 / (n * (n - 1)) as f64
            };

            EntityCluster {
                members,
                internal_edges,
                density,
            }
        })
        .collect()
}

/// Find all articulation points (bridge entities) using Tarjan's algorithm
/// applied to the undirected view of the graph.
///
/// An articulation point is a node whose removal increases the number of
/// connected components — these are the structural kingpins of the network.
pub fn find_bridge_entities(graph: &NfGraph) -> Vec<EntityId> {
    let inner = graph.inner();
    let n = inner.node_count();

    if n == 0 {
        return Vec::new();
    }

    let mut disc = vec![u32::MAX; n];
    let mut low = vec![u32::MAX; n];
    let mut is_ap = vec![false; n];
    let mut timer = 0u32;

    for start in inner.node_indices() {
        if disc[start.index()] == u32::MAX {
            dfs_articulation(inner, start, u32::MAX, &mut disc, &mut low, &mut is_ap, &mut timer);
        }
    }

    inner
        .node_indices()
        .filter(|idx| is_ap[idx.index()])
        .map(|idx| *inner.node_weight(idx).unwrap())
        .collect()
}

/// Recursive DFS for Tarjan's articulation-point algorithm on the undirected
/// projection of a directed graph (edges treated as bidirectional).
///
/// `parent_idx` is `u32::MAX` for the DFS tree root.
fn dfs_articulation(
    graph: &DiGraph<EntityId, nf_core::relationships::RelationshipType>,
    node: NodeIndex,
    parent_raw: u32,
    disc: &mut Vec<u32>,
    low: &mut Vec<u32>,
    is_ap: &mut Vec<bool>,
    timer: &mut u32,
) {
    disc[node.index()] = *timer;
    low[node.index()] = *timer;
    *timer += 1;

    let mut child_count = 0u32;

    // Treat the graph as undirected: consider both outgoing and incoming.
    let out_neighbors: Vec<NodeIndex> = graph
        .neighbors_directed(node, Direction::Outgoing)
        .collect();
    let in_neighbors: Vec<NodeIndex> = graph
        .neighbors_directed(node, Direction::Incoming)
        .collect();

    // Use a HashSet to deduplicate (handles multi-edges and bidirectional pairs).
    let mut seen_neighbors: HashSet<NodeIndex> = HashSet::new();
    let all_neighbors: Vec<NodeIndex> = out_neighbors
        .into_iter()
        .chain(in_neighbors)
        .filter(|&nb| seen_neighbors.insert(nb))
        .collect();

    for neighbor in all_neighbors {
        let nb_idx = neighbor.index();
        if disc[nb_idx] == u32::MAX {
            child_count += 1;
            dfs_articulation(graph, neighbor, node.index() as u32, disc, low, is_ap, timer);

            low[node.index()] = low[node.index()].min(low[nb_idx]);

            // Root with ≥2 children is an AP.
            if parent_raw == u32::MAX && child_count > 1 {
                is_ap[node.index()] = true;
            }
            // Non-root: if the subtree rooted at neighbor cannot reach
            // above node, then node is an AP.
            if parent_raw != u32::MAX && low[nb_idx] >= disc[node.index()] {
                is_ap[node.index()] = true;
            }
        } else if nb_idx as u32 != parent_raw {
            low[node.index()] = low[node.index()].min(disc[nb_idx]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nf_core::relationships::RelationshipType;

    use crate::graph::NfGraph;

    /// 22-node graph with three communities connected by bridge nodes.
    ///
    /// Community A (7 nodes, ids 0-6): fully meshed donations
    /// Community B (7 nodes, ids 7-13): ring + cross donations
    /// Community C (6 nodes, ids 14-19): star topology
    /// Bridge X (id 20): connects A↔B
    /// Bridge Y (id 21): connects B↔C
    fn complex_graph() -> (NfGraph, Vec<EntityId>) {
        let mut g = NfGraph::new();
        let ids: Vec<EntityId> = (0..22).map(|_| EntityId::new()).collect();

        // Community A: mesh (0-6) using DonatedTo
        for i in 0..7 {
            for j in (i + 1)..7 {
                g.add_edge(ids[i], ids[j], RelationshipType::DonatedTo);
            }
        }
        // Community B: ring 7->8->9->10->11->12->13->7
        for i in 7..14 {
            g.add_edge(ids[i], ids[7 + (i - 7 + 1) % 7], RelationshipType::DonatedTo);
        }
        // Community C: star around ids[14]
        for i in 15..20 {
            g.add_edge(ids[14], ids[i], RelationshipType::DonatedTo);
        }
        // Bridges
        g.add_edge(ids[20], ids[0], RelationshipType::DonatedTo);
        g.add_edge(ids[20], ids[7], RelationshipType::DonatedTo);
        g.add_edge(ids[21], ids[13], RelationshipType::DonatedTo);
        g.add_edge(ids[21], ids[14], RelationshipType::DonatedTo);

        (g, ids)
    }

    #[test]
    fn test_find_dense_clusters_min_size_filter() {
        let (g, _) = complex_graph();
        // The whole graph is one connected component with 22 nodes.
        let clusters = find_dense_clusters(&g, 10);
        assert_eq!(clusters.len(), 1);
        assert!(clusters[0].members.len() >= 10);
    }

    #[test]
    fn test_find_dense_clusters_two_components() {
        let mut g = NfGraph::new();
        let ids: Vec<EntityId> = (0..20).map(|_| EntityId::new()).collect();

        // Component 1: chain of 10 nodes
        for i in 0..9 {
            g.add_edge(ids[i], ids[i + 1], RelationshipType::DonatedTo);
        }
        // Component 2: chain of 10 nodes
        for i in 10..19 {
            g.add_edge(ids[i], ids[i + 1], RelationshipType::DonatedTo);
        }

        let clusters = find_dense_clusters(&g, 5);
        assert_eq!(clusters.len(), 2);
        for c in &clusters {
            assert!(c.members.len() >= 5);
        }
    }

    #[test]
    fn test_density_fully_connected() {
        let mut g = NfGraph::new();
        let ids: Vec<EntityId> = (0..5).map(|_| EntityId::new()).collect();
        // Complete directed graph: 5*4 = 20 edges
        for i in 0..5 {
            for j in 0..5 {
                if i != j {
                    g.add_edge(ids[i], ids[j], RelationshipType::DonatedTo);
                }
            }
        }
        let clusters = find_dense_clusters(&g, 2);
        assert_eq!(clusters.len(), 1);
        // density = 2*20 / (5*4) = 40/20 = 2.0 (directed overcounts)
        let d = clusters[0].density;
        assert!(d > 0.0, "Density should be positive, got {d}");
    }

    #[test]
    fn test_density_sparse_chain() {
        let mut g = NfGraph::new();
        let ids: Vec<EntityId> = (0..10).map(|_| EntityId::new()).collect();
        for i in 0..9 {
            g.add_edge(ids[i], ids[i + 1], RelationshipType::DonatedTo);
        }
        let clusters = find_dense_clusters(&g, 5);
        assert_eq!(clusters.len(), 1);
        let d = clusters[0].density;
        // 9 edges, n=10: 2*9/(10*9) = 18/90 = 0.2
        assert!((d - 0.2).abs() < 1e-9, "Expected density 0.2, got {d}");
    }

    #[test]
    fn test_find_dense_clusters_below_min_size_excluded() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);

        let clusters = find_dense_clusters(&g, 5);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_find_bridge_entities_simple_bridge() {
        // A -- bridge -- B
        //   (two separate cliques connected by one node)
        let mut g = NfGraph::new();
        let bridge = EntityId::new();
        let a1 = EntityId::new();
        let a2 = EntityId::new();
        let b1 = EntityId::new();
        let b2 = EntityId::new();

        // Left clique
        g.add_edge(a1, a2, RelationshipType::DonatedTo);
        g.add_edge(a2, a1, RelationshipType::DonatedTo);
        // Right clique
        g.add_edge(b1, b2, RelationshipType::DonatedTo);
        g.add_edge(b2, b1, RelationshipType::DonatedTo);
        // Bridge
        g.add_edge(a1, bridge, RelationshipType::DonatedTo);
        g.add_edge(bridge, b1, RelationshipType::DonatedTo);

        let bridges = find_bridge_entities(&g);
        assert!(
            bridges.contains(&bridge),
            "Bridge node must be identified as articulation point"
        );
    }

    #[test]
    fn test_find_bridge_entities_fully_connected_no_bridge() {
        let mut g = NfGraph::new();
        let ids: Vec<EntityId> = (0..5).map(|_| EntityId::new()).collect();
        // Complete bidirectional graph - no articulation points
        for i in 0..5 {
            for j in 0..5 {
                if i != j {
                    g.add_edge(ids[i], ids[j], RelationshipType::DonatedTo);
                }
            }
        }
        let bridges = find_bridge_entities(&g);
        assert!(
            bridges.is_empty(),
            "Complete graph should have no articulation points"
        );
    }

    #[test]
    fn test_find_bridge_entities_empty_graph() {
        let g = NfGraph::new();
        assert!(find_bridge_entities(&g).is_empty());
    }

    #[test]
    fn test_find_bridge_entities_single_node() {
        let mut g = NfGraph::new();
        g.add_node(EntityId::new());
        assert!(find_bridge_entities(&g).is_empty());
    }

    #[test]
    fn test_find_bridge_entities_chain() {
        // Linear chain: 0->1->2->3->4
        // Nodes 1,2,3 are articulation points in the undirected view.
        let mut g = NfGraph::new();
        let ids: Vec<EntityId> = (0..5).map(|_| EntityId::new()).collect();
        for i in 0..4 {
            g.add_edge(ids[i], ids[i + 1], RelationshipType::DonatedTo);
            g.add_edge(ids[i + 1], ids[i], RelationshipType::DonatedTo); // bidirectional
        }
        let bridges = find_bridge_entities(&g);
        // In a bidirectional chain, every interior node (1,2,3) is an AP
        assert!(
            bridges.contains(&ids[1]),
            "Node 1 should be an articulation point"
        );
        assert!(
            bridges.contains(&ids[2]),
            "Node 2 should be an articulation point"
        );
        assert!(
            bridges.contains(&ids[3]),
            "Node 3 should be an articulation point"
        );
    }

    #[test]
    fn test_cluster_internal_edges_count() {
        let mut g = NfGraph::new();
        let ids: Vec<EntityId> = (0..4).map(|_| EntityId::new()).collect();
        // 3 directed internal edges
        g.add_edge(ids[0], ids[1], RelationshipType::DonatedTo);
        g.add_edge(ids[1], ids[2], RelationshipType::DonatedTo);
        g.add_edge(ids[2], ids[3], RelationshipType::DonatedTo);

        let clusters = find_dense_clusters(&g, 2);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].internal_edges, 3);
    }

    #[test]
    fn test_cluster_members_include_all_nodes() {
        let mut g = NfGraph::new();
        let ids: Vec<EntityId> = (0..20).map(|_| EntityId::new()).collect();
        // Two disconnected components of 10 nodes each
        for i in 0..9 {
            g.add_edge(ids[i], ids[i + 1], RelationshipType::DonatedTo);
        }
        for i in 10..19 {
            g.add_edge(ids[i], ids[i + 1], RelationshipType::DonatedTo);
        }

        let clusters = find_dense_clusters(&g, 1);
        let total: usize = clusters.iter().map(|c| c.members.len()).sum();
        assert_eq!(total, 20);
    }
}
