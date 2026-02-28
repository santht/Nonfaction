use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nf_core::entities::Entity;
use nf_core::relationships::Relationship;
use nf_store::repository::Repository;

use crate::error::{ApiError, ApiResult};
use crate::pagination::{PaginatedResponse, Pagination};
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

    Ok(Json(PaginatedResponse::from_page(
        result,
        params.pagination,
    )))
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

    let mut entries: Vec<TimelineEntry> = outgoing
        .into_iter()
        .chain(incoming)
        .map(|rel| {
            let date = rel.start_date.map(|d| d.to_string());
            TimelineEntry {
                relationship: rel,
                date,
            }
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

    Ok(Json(SourcesResponse {
        entity_id: id,
        source_count,
        sources,
    }))
}

// ─── Create entity ──────────────────────────────────────────────────────────

/// POST /api/v1/entities
///
/// Create a new entity from a fully-formed Entity JSON.
/// Validates that the entity has at least one source reference.
pub async fn create_entity(
    State(state): State<AppState>,
    Json(entity): Json<Entity>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    validate_entity_for_creation(&entity)?;

    let id = state.entity_repo.insert(&entity).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id,
            "entity_type": entity.type_name(),
        })),
    ))
}

fn validate_entity_for_creation(entity: &Entity) -> ApiResult<()> {
    // Validate: entity must have at least one source.
    if entity.sources().source_count() == 0 {
        return Err(ApiError::BadRequest(
            "entity must have at least one source reference".to_string(),
        ));
    }

    // Validate: entity names are not blank for name-bearing entity types.
    match entity {
        Entity::Person(p) if p.name.trim().is_empty() => {
            return Err(ApiError::BadRequest(
                "entity name must not be empty".to_string(),
            ));
        }
        Entity::Organization(o) if o.name.trim().is_empty() => {
            return Err(ApiError::BadRequest(
                "entity name must not be empty".to_string(),
            ));
        }
        _ => {}
    }

    // Validate: source URLs must be http(s) URLs with a host.
    let sources = entity.sources();
    validate_source_url(&sources.primary.source_url)?;
    if let Some(archive_url) = &sources.primary.archive_url {
        validate_source_url(archive_url)?;
    }

    for source in &sources.supporting {
        validate_source_url(&source.source_url)?;
        if let Some(archive_url) = &source.archive_url {
            validate_source_url(archive_url)?;
        }
    }

    Ok(())
}

fn validate_source_url(url: &url::Url) -> ApiResult<()> {
    let is_http = matches!(url.scheme(), "http" | "https");
    let has_host = url.host_str().is_some();

    if !is_http || !has_host {
        return Err(ApiError::BadRequest(
            "source URL must be a valid http(s) URL".to_string(),
        ));
    }

    Ok(())
}

// ─── Create relationship ────────────────────────────────────────────────────

/// POST /api/v1/entities/:id/relationships
///
/// Create a new relationship originating from the given entity.
pub async fn create_relationship(
    State(state): State<AppState>,
    Path(from_id): Path<Uuid>,
    Json(rel): Json<Relationship>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    // Verify the from entity exists
    state
        .entity_repo
        .get(from_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("entity {from_id}")))?;

    // Verify the from_id matches the relationship's from field
    if rel.from.0 != from_id {
        return Err(ApiError::BadRequest(
            "relationship 'from' field must match the entity ID in the URL".to_string(),
        ));
    }

    // Verify the 'to' entity exists
    state
        .entity_repo
        .get(rel.to.0)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("target entity {}", rel.to.0)))?;

    let id = state.relationship_repo.insert(&rel).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id,
            "from": from_id,
            "to": rel.to.0,
            "type": format!("{:?}", rel.rel_type),
        })),
    ))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nf_core::{
        entities::{Entity, Organization, OrganizationType, Person},
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
        use nf_core::{
            entities::EntityId,
            relationships::{Relationship, RelationshipType},
        };
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

    #[test]
    fn test_validate_entity_for_creation_rejects_blank_person_name() {
        let person = Person::new("   ", test_source_chain());
        let entity = Entity::Person(person);
        assert!(validate_entity_for_creation(&entity).is_err());
    }

    #[test]
    fn test_validate_entity_for_creation_rejects_blank_org_name() {
        let org = Organization::new("   ", OrganizationType::Other, test_source_chain());
        let entity = Entity::Organization(org);
        assert!(validate_entity_for_creation(&entity).is_err());
    }

    #[test]
    fn test_validate_entity_for_creation_rejects_non_http_source_url() {
        let source = SourceRef::new(
            Url::parse("file:///tmp/source.txt").unwrap(),
            ContentHash::compute(b"test"),
            SourceType::OtherGovernment,
            "test",
        );
        let entity = Entity::Person(Person::new("Valid Name", SourceChain::new(source)));
        assert!(validate_entity_for_creation(&entity).is_err());
    }

    #[test]
    fn test_validate_entity_for_creation_accepts_valid_person() {
        let entity = Entity::Person(Person::new("Valid Name", test_source_chain()));
        assert!(validate_entity_for_creation(&entity).is_ok());
    }
}
