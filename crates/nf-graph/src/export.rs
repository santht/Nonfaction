use petgraph::visit::EdgeRef;
use serde_json::{json, Value};

use crate::graph::NfGraph;

/// Serialize `graph` to Cytoscape.js JSON format:
/// `{ "nodes": [...], "edges": [...] }`
pub fn to_cytoscape_json(graph: &NfGraph) -> Value {
    let g = graph.inner();

    let nodes: Vec<Value> = g
        .node_indices()
        .map(|n| {
            let id = g.node_weight(n).unwrap();
            json!({ "data": { "id": id.0.to_string() } })
        })
        .collect();

    let edges: Vec<Value> = g
        .edge_references()
        .map(|e| {
            let src = g.node_weight(e.source()).unwrap();
            let tgt = g.node_weight(e.target()).unwrap();
            json!({
                "data": {
                    "source": src.0.to_string(),
                    "target": tgt.0.to_string(),
                    "relationship": format!("{:?}", e.weight())
                }
            })
        })
        .collect();

    json!({ "nodes": nodes, "edges": edges })
}

/// Serialize `graph` to Graphviz DOT format.
pub fn to_dot(graph: &NfGraph) -> String {
    let g = graph.inner();
    let mut dot = String::from("digraph {\n");

    for n in g.node_indices() {
        let id = g.node_weight(n).unwrap();
        dot.push_str(&format!("    \"{}\";\n", id.0));
    }

    for e in g.edge_references() {
        let src = g.node_weight(e.source()).unwrap();
        let tgt = g.node_weight(e.target()).unwrap();
        dot.push_str(&format!(
            "    \"{}\" -> \"{}\" [label=\"{:?}\"];\n",
            src.0,
            tgt.0,
            e.weight()
        ));
    }

    dot.push_str("}\n");
    dot
}

#[cfg(test)]
mod tests {
    use super::*;
    use nf_core::entities::EntityId;
    use nf_core::relationships::RelationshipType;

    #[test]
    fn test_cytoscape_empty_graph() {
        let g = NfGraph::new();
        let json = to_cytoscape_json(&g);
        assert_eq!(json["nodes"].as_array().unwrap().len(), 0);
        assert_eq!(json["edges"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_cytoscape_nodes_and_edges() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);

        let json = to_cytoscape_json(&g);
        assert_eq!(json["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(json["edges"].as_array().unwrap().len(), 1);

        let edge = &json["edges"][0]["data"];
        assert_eq!(edge["source"], a.0.to_string());
        assert_eq!(edge["target"], b.0.to_string());
        assert_eq!(edge["relationship"], "DonatedTo");
    }

    #[test]
    fn test_cytoscape_node_ids_present() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        g.add_node(a);

        let json = to_cytoscape_json(&g);
        let node_id = json["nodes"][0]["data"]["id"].as_str().unwrap();
        assert_eq!(node_id, a.0.to_string());
    }

    #[test]
    fn test_dot_empty_graph() {
        let g = NfGraph::new();
        let dot = to_dot(&g);
        assert_eq!(dot, "digraph {\n}\n");
    }

    #[test]
    fn test_dot_contains_nodes_and_edges() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_edge(a, b, RelationshipType::Pardoned);

        let dot = to_dot(&g);
        assert!(dot.starts_with("digraph {"));
        assert!(dot.contains(&a.0.to_string()));
        assert!(dot.contains(&b.0.to_string()));
        assert!(dot.contains("->"));
        assert!(dot.contains("Pardoned"));
    }

    #[test]
    fn test_dot_structure() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);

        let dot = to_dot(&g);
        // Should have an arrow between the two UUIDs
        let expected_edge = format!(
            "\"{}\" -> \"{}\" [label=\"DonatedTo\"]",
            a.0, b.0
        );
        assert!(dot.contains(&expected_edge));
    }
}
