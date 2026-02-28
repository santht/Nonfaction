use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

use nf_core::entities::Entity;
use nf_core::relationships::Relationship;
use nf_store::repository::Repository;

use crate::error::{ApiError, ApiResult};
use crate::pagination::Pagination;
use crate::state::AppState;

// ─── Cytoscape.js compatible types ───────────────────────────────────────────

/// Cytoscape.js node element.
#[derive(Debug, Serialize)]
pub struct CytoNode {
    pub data: CytoNodeData,
}

#[derive(Debug, Serialize)]
pub struct CytoNodeData {
    pub id: String,
    pub label: String,
    pub entity_type: String,
}

/// Cytoscape.js edge element.
#[derive(Debug, Serialize)]
pub struct CytoEdge {
    pub data: CytoEdgeData,
}

#[derive(Debug, Serialize)]
pub struct CytoEdgeData {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: String,
    pub rel_type: String,
}

/// Full Cytoscape.js compatible graph payload.
#[derive(Debug, Serialize)]
pub struct CytoGraph {
    pub page: u32,
    pub per_page: u32,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub nodes: Vec<CytoNode>,
    pub edges: Vec<CytoEdge>,
}

// ─── GET /graph/:id/network ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct NetworkQuery {
    /// How many hops to traverse (default: 2, max: 3 to prevent huge graphs).
    #[serde(default = "default_depth")]
    pub depth: u32,
    #[serde(flatten)]
    pub pagination: Pagination,
}

fn default_depth() -> u32 {
    2
}

/// GET /api/v1/graph/:id/network
///
/// Returns a Cytoscape.js compatible JSON graph of the entity's network,
/// traversing up to `depth` hops from the root entity.
pub async fn get_network(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<NetworkQuery>,
) -> ApiResult<Json<CytoGraph>> {
    let depth = params.depth.min(3);
    let page = params.pagination.page() as usize;
    let per_page = params.pagination.per_page() as usize;
    let offset = page * per_page;

    // BFS traversal up to `depth` hops.
    let mut visited_entities: HashSet<Uuid> = HashSet::new();
    let mut visited_rels: HashSet<Uuid> = HashSet::new();
    let mut queue: VecDeque<(Uuid, u32)> = VecDeque::new();
    let mut nodes: Vec<CytoNode> = Vec::new();
    let mut edges: Vec<CytoEdge> = Vec::new();

    queue.push_back((id, 0));
    visited_entities.insert(id);

    while let Some((current_id, current_depth)) = queue.pop_front() {
        // Fetch entity and add as node.
        if let Ok(Some(entity)) = state.entity_repo.get(current_id).await {
            nodes.push(CytoNode {
                data: CytoNodeData {
                    id: current_id.to_string(),
                    label: entity_label(&entity),
                    entity_type: entity.type_name().to_string(),
                },
            });
        }

        if current_depth >= depth {
            continue;
        }

        // Fetch outgoing relationships.
        if let Ok(page) = state.relationship_repo.list_from(current_id, 0, 100).await {
            for rel in page.items {
                let rel_id = rel.id.0;
                let neighbor = rel.to.0;

                if !visited_rels.contains(&rel_id) {
                    visited_rels.insert(rel_id);
                    edges.push(rel_to_cyto_edge(&rel));
                }

                if !visited_entities.contains(&neighbor) {
                    visited_entities.insert(neighbor);
                    queue.push_back((neighbor, current_depth + 1));
                }
            }
        }

        // Fetch incoming relationships.
        if let Ok(page) = state.relationship_repo.list_to(current_id, 0, 100).await {
            for rel in page.items {
                let rel_id = rel.id.0;
                let neighbor = rel.from.0;

                if !visited_rels.contains(&rel_id) {
                    visited_rels.insert(rel_id);
                    edges.push(rel_to_cyto_edge(&rel));
                }

                if !visited_entities.contains(&neighbor) {
                    visited_entities.insert(neighbor);
                    queue.push_back((neighbor, current_depth + 1));
                }
            }
        }
    }

    let total_nodes = nodes.len();
    let total_edges = edges.len();
    let nodes = nodes.into_iter().skip(offset).take(per_page).collect();
    let edges = edges.into_iter().skip(offset).take(per_page).collect();

    Ok(Json(CytoGraph {
        page: params.pagination.page,
        per_page: params.pagination.per_page(),
        total_nodes,
        total_edges,
        nodes,
        edges,
    }))
}

// ─── GET /graph/path/:from/:to ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PathResponse {
    pub found: bool,
    pub path_length: usize,
    /// Ordered list of entity IDs along the shortest path.
    pub entity_ids: Vec<String>,
    /// Relationships traversed along the path.
    pub relationships: Vec<Relationship>,
}

/// GET /api/v1/graph/path/:from/:to
///
/// Finds the shortest path between two entities using BFS.
pub async fn get_shortest_path(
    State(state): State<AppState>,
    Path((from, to)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<PathResponse>> {
    if from == to {
        return Ok(Json(PathResponse {
            found: true,
            path_length: 0,
            entity_ids: vec![from.to_string()],
            relationships: vec![],
        }));
    }

    // BFS with parent tracking.
    let mut visited: HashSet<Uuid> = HashSet::new();
    let mut queue: VecDeque<Uuid> = VecDeque::new();
    // parent_map: child → (parent, relationship)
    let mut parent_map: HashMap<Uuid, (Uuid, Relationship)> = HashMap::new();

    queue.push_back(from);
    visited.insert(from);
    let mut found = false;

    'outer: while let Some(current) = queue.pop_front() {
        // Explore outgoing edges.
        if let Ok(page) = state.relationship_repo.list_from(current, 0, 200).await {
            for rel in page.items {
                let neighbor = rel.to.0;
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent_map.insert(neighbor, (current, rel));
                    if neighbor == to {
                        found = true;
                        break 'outer;
                    }
                    queue.push_back(neighbor);
                }
            }
        }

        // Explore incoming edges (bidirectional BFS).
        if let Ok(page) = state.relationship_repo.list_to(current, 0, 200).await {
            for rel in page.items {
                let neighbor = rel.from.0;
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent_map.insert(neighbor, (current, rel));
                    if neighbor == to {
                        found = true;
                        break 'outer;
                    }
                    queue.push_back(neighbor);
                }
            }
        }
    }

    if !found {
        return Ok(Json(PathResponse {
            found: false,
            path_length: 0,
            entity_ids: vec![],
            relationships: vec![],
        }));
    }

    // Reconstruct path from `to` back to `from`.
    let mut entity_ids = vec![to];
    let mut rels = Vec::new();
    let mut current = to;

    while current != from {
        let (parent, rel) = parent_map.remove(&current).unwrap();
        rels.push(rel);
        entity_ids.push(parent);
        current = parent;
    }

    entity_ids.reverse();
    rels.reverse();

    Ok(Json(PathResponse {
        found: true,
        path_length: entity_ids.len() - 1,
        entity_ids: entity_ids.iter().map(|id| id.to_string()).collect(),
        relationships: rels,
    }))
}

// ─── GET /graph/correlations ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CorrelationsQuery {
    /// The official (Person entity) UUID to find timing correlations for.
    pub official_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct CorrelationSummary {
    pub official_id: String,
    pub total_correlations: usize,
    /// Correlations grouped by type.
    pub by_type: HashMap<String, Vec<serde_json::Value>>,
}

/// GET /api/v1/graph/correlations?official_id=X
///
/// Returns timing correlations where the given official is involved.
/// Looks for `TimingCorrelation` entities linked to the official via relationships.
pub async fn get_correlations(
    State(state): State<AppState>,
    Query(params): Query<CorrelationsQuery>,
) -> ApiResult<Json<CorrelationSummary>> {
    let official_id = params.official_id;

    // Verify the official exists.
    state
        .entity_repo
        .get(official_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("official {official_id}")))?;

    // Fetch all TimingCorrelation entities — since nf-graph is still being built,
    // we query the store directly for entities connected to this official.
    let correlations_page = state
        .entity_repo
        .list_by_type("TimingCorrelation", 0, 500)
        .await?;

    let mut by_type: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    let mut total = 0;

    for entity in correlations_page.items {
        if let Entity::TimingCorrelation(ref tc) = entity {
            // Check if the official is involved (event_a or event_b points to them).
            let involves_official = tc.event_a.0 == official_id || tc.event_b.0 == official_id;

            if involves_official {
                total += 1;
                let type_key = format!("{:?}", tc.correlation_type);
                let val = serde_json::to_value(&entity).unwrap_or(serde_json::Value::Null);
                by_type.entry(type_key).or_default().push(val);
            }
        }
    }

    Ok(Json(CorrelationSummary {
        official_id: official_id.to_string(),
        total_correlations: total,
        by_type,
    }))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn entity_label(entity: &Entity) -> String {
    match entity {
        Entity::Person(p) => p.name.clone(),
        Entity::Organization(o) => o.name.clone(),
        Entity::Document(d) => d.title.clone(),
        Entity::Payment(p) => format!("${:.0} payment", p.amount),
        Entity::CourtCase(c) => c.case_id.clone(),
        Entity::Pardon(p) => format!("Pardon: {}", p.offense),
        Entity::FlightLogEntry(f) => format!("Flight {}", f.aircraft_tail_number),
        Entity::TimingCorrelation(t) => {
            format!(
                "{}→{} ({} days)",
                t.event_a_description, t.event_b_description, t.days_between
            )
        }
        Entity::ConductComparison(c) => c.official_action.clone(),
        Entity::PublicStatement(s) => s.content_summary.clone(),
        Entity::PolicyDecision(p) => p.description.clone(),
        Entity::LobbyingActivity(la) => format!("{} for {}", la.registrant_name, la.client_name),
    }
}

fn rel_to_cyto_edge(rel: &Relationship) -> CytoEdge {
    CytoEdge {
        data: CytoEdgeData {
            id: rel.id.0.to_string(),
            source: rel.from.0.to_string(),
            target: rel.to.0.to_string(),
            label: format!("{:?}", rel.rel_type),
            rel_type: format!("{:?}", rel.rel_type),
        },
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nf_core::{
        entities::{Entity, Person},
        source::{ContentHash, SourceChain, SourceRef, SourceType},
    };
    use url::Url;

    fn test_source_chain() -> SourceChain {
        let url = Url::parse("https://api.open.fec.gov/v1/test/").unwrap();
        let source = SourceRef::new(
            url,
            ContentHash::compute(b"test"),
            SourceType::FecFiling,
            "test",
        );
        SourceChain::new(source)
    }

    #[test]
    fn test_entity_label_person() {
        let person = Person::new("Jane Doe", test_source_chain());
        let entity = Entity::Person(person);
        assert_eq!(entity_label(&entity), "Jane Doe");
    }

    #[test]
    fn test_default_depth() {
        assert_eq!(default_depth(), 2);
    }

    #[test]
    fn test_path_response_same_entity() {
        let id = Uuid::new_v4();
        let response = PathResponse {
            found: true,
            path_length: 0,
            entity_ids: vec![id.to_string()],
            relationships: vec![],
        };
        assert!(response.found);
        assert_eq!(response.path_length, 0);
        assert_eq!(response.entity_ids.len(), 1);
    }

    #[test]
    fn test_cyto_edge_label_from_rel() {
        use nf_core::{
            entities::EntityId,
            relationships::{Relationship, RelationshipType},
        };
        let rel = Relationship::new(
            EntityId::new(),
            EntityId::new(),
            RelationshipType::DonatedTo,
            test_source_chain(),
        );
        let edge = rel_to_cyto_edge(&rel);
        assert_eq!(edge.data.rel_type, "DonatedTo");
    }

    #[test]
    fn test_network_query_pagination_defaults() {
        let query = NetworkQuery {
            depth: 2,
            pagination: Pagination {
                page: 1,
                per_page: 20,
            },
        };
        assert_eq!(query.depth, 2);
        assert_eq!(query.pagination.page, 1);
        assert_eq!(query.pagination.per_page(), 20);
    }

    #[test]
    fn test_network_query_pagination_clamped() {
        let query = NetworkQuery {
            depth: 1,
            pagination: Pagination {
                page: 2,
                per_page: 500,
            },
        };
        assert_eq!(query.pagination.page, 2);
        assert_eq!(query.pagination.per_page(), 100);
    }
}
