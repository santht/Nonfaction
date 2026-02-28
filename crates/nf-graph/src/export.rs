use petgraph::visit::EdgeRef;
use serde_json::{Value, json};

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

/// Serialize `graph` to GraphML format (used by Gephi, yEd, and other tools).
pub fn to_graphml(graph: &NfGraph) -> String {
    let g = graph.inner();
    let mut out = String::new();

    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<graphml xmlns=\"http://graphml.graphdrawing.org/graphml\"\n");
    out.push_str("         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n");
    out.push_str("         xsi:schemaLocation=\"http://graphml.graphdrawing.org/graphml http://graphml.graphdrawing.org/graphml/graphml.xsd\">\n");
    out.push_str("  <key id=\"relationship\" for=\"edge\" attr.name=\"relationship\" attr.type=\"string\"/>\n");
    out.push_str("  <graph id=\"G\" edgedefault=\"directed\">\n");

    for n in g.node_indices() {
        let id = g.node_weight(n).unwrap();
        out.push_str(&format!("    <node id=\"{}\"/>\n", id.0));
    }

    for (i, e) in g.edge_references().enumerate() {
        let src = g.node_weight(e.source()).unwrap();
        let tgt = g.node_weight(e.target()).unwrap();
        out.push_str(&format!(
            "    <edge id=\"e{}\" source=\"{}\" target=\"{}\">\n",
            i, src.0, tgt.0
        ));
        out.push_str(&format!(
            "      <data key=\"relationship\">{:?}</data>\n",
            e.weight()
        ));
        out.push_str("    </edge>\n");
    }

    out.push_str("  </graph>\n");
    out.push_str("</graphml>\n");
    out
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
        let expected_edge = format!("\"{}\" -> \"{}\" [label=\"DonatedTo\"]", a.0, b.0);
        assert!(dot.contains(&expected_edge));
    }

    // ── to_graphml tests ──────────────────────────────────────────────────────

    #[test]
    fn test_graphml_empty_graph() {
        let g = NfGraph::new();
        let xml = to_graphml(&g);
        assert!(xml.contains("<?xml version=\"1.0\""));
        assert!(xml.contains("<graphml"));
        assert!(xml.contains("edgedefault=\"directed\""));
        assert!(!xml.contains("<node"));
        assert!(!xml.contains("<edge"));
    }

    #[test]
    fn test_graphml_contains_node_ids() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_node(a);
        g.add_node(b);

        let xml = to_graphml(&g);
        assert!(xml.contains(&format!("<node id=\"{}\"/>", a.0)));
        assert!(xml.contains(&format!("<node id=\"{}\"/>", b.0)));
    }

    #[test]
    fn test_graphml_contains_edge_with_relationship() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);

        let xml = to_graphml(&g);
        assert!(xml.contains(&format!("source=\"{}\"", a.0)));
        assert!(xml.contains(&format!("target=\"{}\"", b.0)));
        assert!(xml.contains("DonatedTo"));
    }

    #[test]
    fn test_graphml_relationship_key_declared() {
        let g = NfGraph::new();
        let xml = to_graphml(&g);
        assert!(xml.contains("<key id=\"relationship\""));
        assert!(xml.contains("attr.name=\"relationship\""));
        assert!(xml.contains("attr.type=\"string\""));
    }

    #[test]
    fn test_graphml_multiple_edges_indexed() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);
        g.add_edge(b, c, RelationshipType::Pardoned);

        let xml = to_graphml(&g);
        assert!(xml.contains("id=\"e0\""));
        assert!(xml.contains("id=\"e1\""));
        assert!(xml.contains("DonatedTo"));
        assert!(xml.contains("Pardoned"));
    }

    #[test]
    fn test_graphml_graph_tag_closed() {
        let mut g = NfGraph::new();
        let a = EntityId::new();
        let b = EntityId::new();
        g.add_edge(a, b, RelationshipType::DonatedTo);

        let xml = to_graphml(&g);
        assert!(xml.contains("</graph>"));
        assert!(xml.contains("</graphml>"));
    }
}
