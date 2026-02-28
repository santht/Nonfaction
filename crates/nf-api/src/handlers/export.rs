use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::io::Write;
use uuid::Uuid;
use zip::{ZipWriter, write::FileOptions};

use nf_core::entities::Entity;
use nf_store::repository::Repository;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// ─── Story Package Exporter ───────────────────────────────────────────────────
//
// This is a UNIQUE Nonfaction feature: given any entity ID, produce a
// self-contained ZIP archive containing:
//   1. entity.json      — full entity data with all fields
//   2. sources.json     — all SourceRef records in the entity's SourceChain
//   3. relationships.json — all incoming + outgoing relationships
//   4. timeline.json    — relationships sorted by date
//   5. graph.json       — Cytoscape.js compatible network (depth=2)
//   6. README.txt       — human-readable summary and citation instructions
//
// Journalists can download this package and have everything needed to
// publish a sourced story about the entity.

/// GET /api/v1/export/story-package/:id
///
/// Returns a `.zip` file containing all source documents, citations,
/// entity timeline, and relationship graph JSON.
pub async fn export_story_package(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    // 1. Fetch the root entity.
    let entity = require_entity(state.entity_repo.get(id).await?, id)?;

    // 2. Fetch all relationships.
    let outgoing = state.relationship_repo.list_from(id, 0, 500).await?.items;
    let incoming = state.relationship_repo.list_to(id, 0, 500).await?.items;

    // 3. Build the timeline (relationships sorted by start_date).
    let mut timeline: Vec<_> = outgoing
        .iter()
        .chain(incoming.iter())
        .map(|rel| {
            let date = rel.start_date.map(|d| d.to_string());
            serde_json::json!({
                "relationship_id": rel.id.0,
                "from": rel.from.0,
                "to": rel.to.0,
                "type": format!("{:?}", rel.rel_type),
                "date": date,
                "properties": &rel.properties,
            })
        })
        .collect();

    timeline.sort_by(|a, b| {
        let da = a["date"].as_str().map(|s| s.to_string());
        let db = b["date"].as_str().map(|s| s.to_string());
        match (da, db) {
            (Some(da), Some(db)) => da.cmp(&db),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    // 4. Build Cytoscape.js compatible graph (depth=1 from this entity).
    let mut cyto_nodes: Vec<serde_json::Value> = Vec::new();
    let mut cyto_edges: Vec<serde_json::Value> = Vec::new();

    // Root node.
    cyto_nodes.push(serde_json::json!({
        "data": {
            "id": id.to_string(),
            "label": entity_label_str(&entity),
            "entity_type": entity.type_name(),
        }
    }));

    // Neighbor nodes + edges.
    for rel in outgoing.iter().chain(incoming.iter()) {
        let neighbor_id = if rel.from.0 == id {
            rel.to.0
        } else {
            rel.from.0
        };

        cyto_edges.push(serde_json::json!({
            "data": {
                "id": rel.id.0.to_string(),
                "source": rel.from.0.to_string(),
                "target": rel.to.0.to_string(),
                "label": format!("{:?}", rel.rel_type),
            }
        }));

        // Fetch neighbor entity for label.
        if let Ok(Some(neighbor)) = state.entity_repo.get(neighbor_id).await {
            // Avoid duplicates.
            let already = cyto_nodes
                .iter()
                .any(|n| n["data"]["id"] == neighbor_id.to_string());
            if !already {
                cyto_nodes.push(serde_json::json!({
                    "data": {
                        "id": neighbor_id.to_string(),
                        "label": entity_label_str(&neighbor),
                        "entity_type": neighbor.type_name(),
                    }
                }));
            }
        }
    }

    let graph_json = serde_json::json!({ "nodes": cyto_nodes, "edges": cyto_edges });

    // 5. Serialize everything to JSON bytes.
    let entity_json = serde_json::to_vec_pretty(&entity).map_err(ApiError::Serialization)?;

    let sources_json =
        serde_json::to_vec_pretty(entity.sources()).map_err(ApiError::Serialization)?;

    let all_rels: Vec<_> = outgoing.iter().chain(incoming.iter()).collect();
    let relationships_json =
        serde_json::to_vec_pretty(&all_rels).map_err(ApiError::Serialization)?;

    let timeline_json = serde_json::to_vec_pretty(&timeline).map_err(ApiError::Serialization)?;

    let graph_bytes = serde_json::to_vec_pretty(&graph_json).map_err(ApiError::Serialization)?;

    let readme = build_readme(&entity, id, outgoing.len() + incoming.len());

    // 6. Build ZIP archive in memory.
    let zip_bytes = build_zip(
        &entity_json,
        &sources_json,
        &relationships_json,
        &timeline_json,
        &graph_bytes,
        readme.as_bytes(),
    )
    .map_err(|e| ApiError::Internal(format!("zip error: {e}")))?;

    let filename = format!("story-package-{}.zip", id);

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip"),
            (
                header::CONTENT_DISPOSITION,
                Box::leak(format!("attachment; filename=\"{filename}\"").into_boxed_str()),
            ),
        ],
        Bytes::from(zip_bytes),
    )
        .into_response())
}

fn require_entity(entity: Option<Entity>, id: Uuid) -> ApiResult<Entity> {
    entity.ok_or_else(|| ApiError::NotFound(format!("entity {id}")))
}

// ─── ZIP builder ─────────────────────────────────────────────────────────────

fn build_zip(
    entity_json: &[u8],
    sources_json: &[u8],
    relationships_json: &[u8],
    timeline_json: &[u8],
    graph_json: &[u8],
    readme: &[u8],
) -> Result<Vec<u8>, std::io::Error> {
    let buf = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(buf));
    let options = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("entity.json", options)?;
    zip.write_all(entity_json)?;

    zip.start_file("sources.json", options)?;
    zip.write_all(sources_json)?;

    zip.start_file("relationships.json", options)?;
    zip.write_all(relationships_json)?;

    zip.start_file("timeline.json", options)?;
    zip.write_all(timeline_json)?;

    zip.start_file("graph.json", options)?;
    zip.write_all(graph_json)?;

    zip.start_file("README.txt", options)?;
    zip.write_all(readme)?;

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

// ─── README builder ───────────────────────────────────────────────────────────

fn build_readme(entity: &Entity, id: Uuid, relationship_count: usize) -> String {
    format!(
        "NONFACTION STORY PACKAGE\n\
         ========================\n\
         \n\
         Entity ID:    {id}\n\
         Entity Type:  {}\n\
         Entity Name:  {}\n\
         Relationships: {relationship_count}\n\
         \n\
         CONTENTS\n\
         --------\n\
         entity.json        - Full entity data (sourced, versioned)\n\
         sources.json       - All source references (URLs, hashes, filing IDs)\n\
         relationships.json - All known connections to other entities\n\
         timeline.json      - Connections sorted chronologically\n\
         graph.json         - Cytoscape.js compatible network graph\n\
         README.txt         - This file\n\
         \n\
         CITATION\n\
         --------\n\
         All data in this package is sourced from public government records.\n\
         Each entity and relationship includes a SourceChain with:\n\
         - Primary source URL\n\
         - SHA-256 content hash (proof of what the source said)\n\
         - Filing ID (FEC, PACER, SEC accession, etc.)\n\
         - Archive URL (Wayback Machine or internal archive)\n\
         \n\
         Generated by Nonfaction (https://nonfaction.org)\n\
         License: GPL-3.0-only\n",
        entity.type_name(),
        entity_label_str(entity),
    )
}

fn entity_label_str(entity: &Entity) -> String {
    match entity {
        Entity::Person(p) => p.name.clone(),
        Entity::Organization(o) => o.name.clone(),
        Entity::Document(d) => d.title.clone(),
        Entity::Payment(p) => format!("${:.0} payment", p.amount),
        Entity::CourtCase(c) => c.case_id.clone(),
        Entity::Pardon(p) => format!("Pardon: {}", p.offense),
        Entity::FlightLogEntry(f) => format!("Flight {}", f.aircraft_tail_number),
        Entity::TimingCorrelation(t) => {
            format!("{}→{}", t.event_a_description, t.event_b_description)
        }
        Entity::ConductComparison(c) => c.official_action.clone(),
        Entity::PublicStatement(s) => s.content_summary.clone(),
        Entity::PolicyDecision(p) => p.description.clone(),
        Entity::LobbyingActivity(la) => format!("{} for {}", la.registrant_name, la.client_name),
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
    fn test_entity_label_str_person() {
        let person = Person::new("Jane Doe", test_source_chain());
        let entity = Entity::Person(person);
        assert_eq!(entity_label_str(&entity), "Jane Doe");
    }

    #[test]
    fn test_build_readme_contains_entity_info() {
        let person = Person::new("Test Official", test_source_chain());
        let entity = Entity::Person(person);
        let id = Uuid::new_v4();
        let readme = build_readme(&entity, id, 5);
        assert!(readme.contains("Person"));
        assert!(readme.contains("Test Official"));
        assert!(readme.contains("5"));
        assert!(readme.contains("Nonfaction"));
    }

    #[test]
    fn test_build_zip_produces_valid_archive() {
        let zip_bytes = build_zip(
            b"{\"test\": true}",
            b"{\"sources\": []}",
            b"[]",
            b"[]",
            b"{\"nodes\": [], \"edges\": []}",
            b"README content",
        )
        .unwrap();

        // ZIP files start with PK magic bytes.
        assert!(zip_bytes.len() > 4);
        assert_eq!(&zip_bytes[0..2], b"PK");
    }

    #[test]
    fn test_build_zip_all_files_present() {
        let zip_bytes = build_zip(
            b"entity",
            b"sources",
            b"relationships",
            b"timeline",
            b"graph",
            b"readme",
        )
        .unwrap();

        let cursor = std::io::Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();

        assert!(names.contains(&"entity.json".to_string()));
        assert!(names.contains(&"sources.json".to_string()));
        assert!(names.contains(&"relationships.json".to_string()));
        assert!(names.contains(&"timeline.json".to_string()));
        assert!(names.contains(&"graph.json".to_string()));
        assert!(names.contains(&"README.txt".to_string()));
    }

    #[test]
    fn test_require_entity_not_found() {
        let id = Uuid::new_v4();
        let err = require_entity(None, id).unwrap_err();
        match err {
            ApiError::NotFound(msg) => assert!(msg.contains(&id.to_string())),
            _ => panic!("expected not found error"),
        }
    }
}
