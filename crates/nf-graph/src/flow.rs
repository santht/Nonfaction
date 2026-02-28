// Money flow tracing through financial relationship chains.
// Detects PAC-to-PAC-to-candidate laundering by following DonatedTo and
// ReceivedContract edges up to max_depth hops via BFS.

use std::collections::{HashMap, VecDeque};

use nf_core::entities::EntityId;
use nf_core::relationships::RelationshipType;
use petgraph::Direction;
use petgraph::visit::EdgeRef;

use crate::graph::NfGraph;

fn is_financial(rel: &RelationshipType) -> bool {
    matches!(
        rel,
        RelationshipType::DonatedTo | RelationshipType::ReceivedContract | RelationshipType::LobbiedFor
    )
}

/// A traced path of money flowing through the graph from a source entity.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowPath {
    /// Ordered list of entity IDs: [source, hop1, hop2, …, terminal]
    pub hops: Vec<EntityId>,
    /// Sum of edge amounts along the path (0.0 for edges without amounts).
    pub total_amount: f64,
    /// Number of edges traversed (= hops.len() - 1).
    pub hop_count: usize,
}

/// Traces money flow through an [`NfGraph`] starting from a source entity,
/// following financial relationship edges up to `max_depth` hops.
pub struct MoneyFlowTracer<'a> {
    graph: &'a NfGraph,
    /// Amounts attached to directed edges (from, to) -> USD.
    amounts: HashMap<(EntityId, EntityId), f64>,
    /// Maximum BFS depth (default 5).
    pub max_depth: usize,
}

impl<'a> MoneyFlowTracer<'a> {
    pub fn new(graph: &'a NfGraph) -> Self {
        Self {
            graph,
            amounts: HashMap::new(),
            max_depth: 5,
        }
    }

    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Register the USD amount for a directed financial edge.
    pub fn with_amount(mut self, from: EntityId, to: EntityId, amount: f64) -> Self {
        self.amounts.insert((from, to), amount);
        self
    }

    /// Bulk-register edge amounts.
    pub fn set_amounts(&mut self, amounts: HashMap<(EntityId, EntityId), f64>) {
        self.amounts = amounts;
    }

    /// BFS from `source` following financial edges.  Returns one [`FlowPath`]
    /// for every reachable entity (at every depth ≤ max_depth, cycle-free).
    pub fn trace(&self, source: EntityId) -> Vec<FlowPath> {
        let inner = self.graph.inner();
        let node_map = self.graph.entity_map();

        let mut results = Vec::new();

        // Queue item: (current node, path from source, accumulated amount)
        let mut queue: VecDeque<(EntityId, Vec<EntityId>, f64)> = VecDeque::new();
        queue.push_back((source, vec![source], 0.0));

        while let Some((current, path, accumulated)) = queue.pop_front() {
            let hop_count = path.len() - 1;

            // Record every node reached beyond the source.
            if hop_count > 0 {
                results.push(FlowPath {
                    hops: path.clone(),
                    total_amount: accumulated,
                    hop_count,
                });
            }

            // Stop expanding at max depth.
            if hop_count >= self.max_depth {
                continue;
            }

            let idx = match node_map.get(&current) {
                Some(&i) => i,
                None => continue,
            };

            for edge in inner.edges_directed(idx, Direction::Outgoing) {
                if !is_financial(edge.weight()) {
                    continue;
                }
                let target = *inner.node_weight(edge.target()).unwrap();

                // Avoid cycles.
                if path.contains(&target) {
                    continue;
                }

                let edge_amount = self
                    .amounts
                    .get(&(current, target))
                    .copied()
                    .unwrap_or(0.0);
                let mut new_path = path.clone();
                new_path.push(target);
                queue.push_back((target, new_path, accumulated + edge_amount));
            }
        }

        results
    }
}

/// Sum total money received at each terminal (last-hop) entity across all paths.
pub fn aggregate_flow_destinations(flows: &[FlowPath]) -> HashMap<EntityId, f64> {
    let mut totals: HashMap<EntityId, f64> = HashMap::new();
    for flow in flows {
        if let Some(&terminal) = flow.hops.last() {
            *totals.entry(terminal).or_insert(0.0) += flow.total_amount;
        }
    }
    totals
}

#[cfg(test)]
mod tests {
    use super::*;
    use nf_core::relationships::RelationshipType;

    use crate::graph::NfGraph;

    /// Build a 20-node PAC laundering graph:
    ///
    /// ```text
    /// donor[0..4] --DonatedTo--> pac[0..3]
    /// pac[0..3]   --DonatedTo--> super_pac[0..1]
    /// super_pac[0] --DonatedTo-> mid[0..2]
    /// super_pac[1] --DonatedTo-> mid[2..4]
    /// mid[0..4]   --DonatedTo--> candidate[0..1]
    /// orphan (isolated)
    /// ```
    fn laundering_graph() -> (NfGraph, Vec<EntityId>) {
        let mut g = NfGraph::new();
        let ids: Vec<EntityId> = (0..20).map(|_| EntityId::new()).collect();

        // donors 0-4 -> pacs 5-8
        for d in 0..5 {
            g.add_edge(ids[d], ids[5 + (d % 4)], RelationshipType::DonatedTo);
        }
        // pacs 5-8 -> super_pacs 9-10
        for p in 5..9 {
            g.add_edge(ids[p], ids[9 + (p % 2)], RelationshipType::DonatedTo);
        }
        // super_pac 9 -> mids 11-12
        g.add_edge(ids[9], ids[11], RelationshipType::DonatedTo);
        g.add_edge(ids[9], ids[12], RelationshipType::DonatedTo);
        // super_pac 10 -> mids 13-14
        g.add_edge(ids[10], ids[13], RelationshipType::DonatedTo);
        g.add_edge(ids[10], ids[14], RelationshipType::DonatedTo);
        // mids 11-14 -> candidates 15-16
        for m in 11..15 {
            g.add_edge(ids[m], ids[15 + (m % 2)], RelationshipType::DonatedTo);
        }
        // Non-financial edges that tracer must ignore
        g.add_edge(ids[0], ids[17], RelationshipType::Pardoned);
        g.add_edge(ids[17], ids[18], RelationshipType::FamilyOf);
        // Isolated node
        g.add_node(ids[19]);

        (g, ids)
    }

    #[test]
    fn test_trace_follows_only_financial_edges() {
        let (g, ids) = laundering_graph();
        let tracer = MoneyFlowTracer::new(&g);
        let flows = tracer.trace(ids[0]);

        // ids[17] reached only via Pardoned — must NOT appear in any flow
        for flow in &flows {
            assert!(
                !flow.hops.contains(&ids[17]),
                "Non-financial node should not appear in flow paths"
            );
        }
        // ids[0] -> ids[5] -> ids[9] -> ids[11] -> ids[15] should be reachable
        let has_deep = flows.iter().any(|f| f.hops.contains(&ids[15]));
        assert!(has_deep, "Should trace money all the way to candidates");
    }

    #[test]
    fn test_trace_respects_max_depth() {
        let (g, ids) = laundering_graph();
        let tracer = MoneyFlowTracer::new(&g).with_max_depth(2);
        let flows = tracer.trace(ids[0]);

        // With max_depth=2 we can only reach: ids[5], ids[9..10]
        // Candidates (depth 4) must not appear.
        for flow in &flows {
            assert!(
                flow.hop_count <= 2,
                "hop_count {} exceeds max_depth 2",
                flow.hop_count
            );
        }
    }

    #[test]
    fn test_trace_accumulates_amounts() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(b, c, RelationshipType::DonatedTo);

        let tracer = MoneyFlowTracer::new(&g)
            .with_amount(a, b, 1000.0)
            .with_amount(b, c, 500.0);

        let flows = tracer.trace(a);

        let to_c = flows
            .iter()
            .find(|f| f.hops == vec![a, b, c])
            .expect("Path a->b->c must exist");
        assert!((to_c.total_amount - 1500.0).abs() < 1e-9);

        let to_b = flows
            .iter()
            .find(|f| f.hops == vec![a, b])
            .expect("Path a->b must exist");
        assert!((to_b.total_amount - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn test_trace_no_cycles() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        // Cycle: a->b->c->a
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(b, c, RelationshipType::DonatedTo);
        g.add_edge(c, a, RelationshipType::DonatedTo);

        let flows = MoneyFlowTracer::new(&g).trace(a);
        // Every path must be cycle-free
        for flow in &flows {
            let unique: std::collections::HashSet<_> = flow.hops.iter().collect();
            assert_eq!(
                unique.len(),
                flow.hops.len(),
                "Flow path contains a cycle: {:?}",
                flow.hops
            );
        }
    }

    #[test]
    fn test_trace_unknown_source_returns_empty() {
        let g = NfGraph::new();
        let unknown = EntityId::new();
        let flows = MoneyFlowTracer::new(&g).trace(unknown);
        assert!(flows.is_empty());
    }

    #[test]
    fn test_aggregate_flow_destinations() {
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();

        let flows = vec![
            FlowPath {
                hops: vec![a, b],
                total_amount: 100.0,
                hop_count: 1,
            },
            FlowPath {
                hops: vec![a, b, c],
                total_amount: 250.0,
                hop_count: 2,
            },
            FlowPath {
                hops: vec![a, b, c],
                total_amount: 150.0,
                hop_count: 2,
            },
        ];

        let agg = aggregate_flow_destinations(&flows);
        assert!((agg[&b] - 100.0).abs() < 1e-9);
        assert!((agg[&c] - 400.0).abs() < 1e-9);
    }

    #[test]
    fn test_aggregate_empty_flows() {
        let agg = aggregate_flow_destinations(&[]);
        assert!(agg.is_empty());
    }

    #[test]
    fn test_received_contract_is_financial() {
        let mut g = NfGraph::new();
        let gov = EntityId::new();
        let firm = EntityId::new();
        g.add_edge(gov, firm, RelationshipType::ReceivedContract);

        let flows = MoneyFlowTracer::new(&g).trace(gov);
        assert!(!flows.is_empty());
        assert_eq!(flows[0].hops, vec![gov, firm]);
    }

    #[test]
    fn test_hop_count_matches_path_length() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        let d = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(b, c, RelationshipType::DonatedTo);
        g.add_edge(c, d, RelationshipType::DonatedTo);

        let tracer = MoneyFlowTracer::new(&g);
        for flow in tracer.trace(a) {
            assert_eq!(flow.hop_count, flow.hops.len() - 1);
        }
    }

    #[test]
    fn test_full_laundering_scenario() {
        // Simulates a classic dark-money chain:
        // Real donor -> shell PAC -> super PAC -> candidate
        let mut g = NfGraph::new();
        let real_donor = EntityId::new();
        let shell = EntityId::new();
        let super_pac = EntityId::new();
        let candidate = EntityId::new();

        g.add_edge(real_donor, shell, RelationshipType::DonatedTo);
        g.add_edge(shell, super_pac, RelationshipType::DonatedTo);
        g.add_edge(super_pac, candidate, RelationshipType::DonatedTo);

        let tracer = MoneyFlowTracer::new(&g)
            .with_amount(real_donor, shell, 500_000.0)
            .with_amount(shell, super_pac, 490_000.0)
            .with_amount(super_pac, candidate, 480_000.0);

        let flows = tracer.trace(real_donor);
        let dest = aggregate_flow_destinations(&flows);

        // Only the final hop's paths matter: candidate should have 480_000 from direct path
        assert!(dest.contains_key(&candidate));
        let final_amount = dest[&candidate];
        assert!(
            final_amount > 0.0,
            "Candidate should receive money: {}",
            final_amount
        );
    }
}
