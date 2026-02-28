use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nf_core::entities::Entity;
use nf_core::relationships::Relationship;
use nf_store::repository::Repository;

use crate::error::{ApiError, ApiResult};
use crate::pagination::{Pagination, PaginatedResponse};
use crate::state::AppState;

// ─── List entities ────────────────────────────────────────────────────────────

/// Query parameters for GET /entities
#[derive(Debug, Deserialize)]
pub struct ListEntitiesQuery {
    /// Filter by entity type (Person, Organization, Payment, etc.)
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
    #[serde(flatten)]
    pub pagination: Pagination,
}

/// GET /api/v1/entities
///
/// Returns a paginated list of all entities, optionally filtered by type.
pub async fn list_entities(
    State(state): State<AppState>,
    Query(params): Query<ListEntitiesQuery>,
) -> ApiResult<Json<PaginatedResponse<Entity>>> {
    let page = params.pagination.page();
    let per_page = params.pagination.per_page();

    let result = if let Some(ref entity_type) = params.entity_type {
        state
            .entity_repo
            .list_by_type(entity_type, page, per_page)
            .await?
    } else {
        state.entity_repo.list(page, per_page).await?
    };

    Ok(Json(PaginatedResponse::from_page(result, params.pagination)))
}

// ─── Get entity by ID ────────────────────────────────────────────────────────

/// GET /api/v1/entities/:id
pub async fn get_entity(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Entity>> {
    let entity = state
        .entity_repo
        .get(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("entity {id}")))?;

    Ok(Json(entity))
}

// ─── Get entity relationships ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RelationshipBundle {
    pub outgoing: Vec<Relationship>,
    pub incoming: Vec<Relationship>,
}

/// GET /api/v1/entities/:id/relationships
///
/// Returns all relationships (outgoing and incoming) for an entity.
pub async fn get_entity_relationships(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(pagination): Query<Pagination>,
) -> ApiResult<Json<RelationshipBundle>> {
    // Verify entity exists.
    state
        .entity_repo
        .get(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("entity {id}")))?;

    let page = pagination.page();
    let per_page = pagination.per_page();

    let outgoing = state
        .relationship_repo
        .list_from(id, page, per_page)
        .await?
        .items;

    let incoming = state
        .relationship_repo
        .list_to(id, page, per_page)
        .await?
        .items;

    Ok(Json(RelationshipBundle { outgoing, incoming }))
}

// ─── Entity timeline ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TimelineEntry {
    pub relationship: Relationship,
    /// ISO-8601 date string for the start date (if present), for sorting.
    pub date: Option<String>,
}

/// GET /api/v1/entities/:id/timeline
///
/// Returns all connections sorted by start_date ascending (null dates last).
pub async fn get_entity_timeline(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(pagination): Query<Pagination>,
) -> ApiResult<Json<Vec<TimelineEntry>>> {
    state
        .entity_repo
        .get(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("entity {id}")))?;

    let page = pagination.page();
    let per_page = pagination.per_page();

    let outgoing = state.relationship_repo.list_from(id, page, per_page).await?.items;
    let incoming = state.relationship_repo.list_to(id, page, per_page).await?.items;

    let mut entries: Vec<TimelineEntry> = outgoing
        .into_iter()
        .chain(incoming)
        .map(|rel| {
            let date = rel.start_date.map(|d| d.to_string());
            TimelineEntry { relationship: rel, date }
        })
        .collect();

    // Sort by date ascending; None dates go to the end.
    entries.sort_by(|a, b| match (&a.date, &b.date) {
        (Some(da), Some(db)) => da.cmp(db),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    Ok(Json(entries))
}

// ─── Entity sources ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SourcesResponse {
    pub entity_id: Uuid,
    pub source_count: usize,
    pub sources: serde_json::Value,
}

/// GET /api/v1/entities/:id/sources
///
/// Returns all source references for an entity.
pub async fn get_entity_sources(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SourcesResponse>> {
    let entity = state
        .entity_repo
        .get(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("entity {id}")))?;

    let sources_chain = entity.sources();
    let source_count = sources_chain.source_count();
    let sources = serde_json::to_value(sources_chain)?;

    Ok(Json(SourcesResponse { entity_id: id, source_count, sources }))
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
    fn test_timeline_entry_sorting() {
        // Test date sorting logic: entries with dates come before those without.
        let mut entries = vec![
            TimelineEntry {
                relationship: make_rel(),
                date: None,
            },
            TimelineEntry {
                relationship: make_rel(),
                date: Some("2020-01-01".to_string()),
            },
            TimelineEntry {
                relationship: make_rel(),
                date: Some("2019-06-15".to_string()),
            },
        ];

        entries.sort_by(|a, b| match (&a.date, &b.date) {
            (Some(da), Some(db)) => da.cmp(db),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        assert_eq!(entries[0].date.as_deref(), Some("2019-06-15"));
        assert_eq!(entries[1].date.as_deref(), Some("2020-01-01"));
        assert_eq!(entries[2].date, None);
    }

    fn make_rel() -> Relationship {
        use nf_core::{entities::EntityId, relationships::{Relationship, RelationshipType}};
        Relationship::new(
            EntityId::new(),
            EntityId::new(),
            RelationshipType::DonatedTo,
            test_source_chain(),
        )
    }

    #[test]
    fn test_sources_response_fields() {
        let person = Person::new("Test", test_source_chain());
        let entity = Entity::Person(person);
        let sources_chain = entity.sources();
        assert_eq!(sources_chain.source_count(), 1);
    }
}
