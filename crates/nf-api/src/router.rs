use axum::{
    Json, Router,
    extract::State,
    http::{HeaderName, StatusCode},
    routing::{delete, get, post},
};
use serde_json::json;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::handlers::{docs, entities, export, graph, search, submissions, watchlist};
use crate::state::AppState;

/// Build the full Axum router for the Nonfaction API.
///
/// Routes:
///   GET  /health
///   GET  /api/v1/entities
///   GET  /api/v1/entities/:id
///   GET  /api/v1/entities/:id/relationships
///   GET  /api/v1/entities/:id/timeline
///   GET  /api/v1/entities/:id/sources
///   GET  /api/v1/search
///   GET  /api/v1/graph/:id/network
///   GET  /api/v1/graph/path/:from/:to
///   GET  /api/v1/graph/correlations
///   GET  /api/v1/export/story-package/:id
///   POST /api/v1/submissions
///   GET  /api/v1/submissions
///   GET  /api/v1/submissions/:id
///   POST /api/v1/submissions/:id/review
///   POST /api/v1/watchlist/subscribe
///   GET  /api/v1/watchlist
///   DELETE /api/v1/watchlist/:id
///   GET  /api/v1/stats
///   GET  /api/v1/audit/verify
pub fn build_router(state: AppState) -> Router {
    let v1 = Router::new()
        // ── Entities ─────────────────────────────────────────────────────────
        .route(
            "/entities",
            get(entities::list_entities).post(entities::create_entity),
        )
        .route("/entities/{id}", get(entities::get_entity))
        .route(
            "/entities/{id}/relationships",
            get(entities::get_entity_relationships).post(entities::create_relationship),
        )
        .route(
            "/entities/{id}/timeline",
            get(entities::get_entity_timeline),
        )
        .route("/entities/{id}/sources", get(entities::get_entity_sources))
        // ── Search ────────────────────────────────────────────────────────────
        .route("/search", get(search::search))
        // ── Graph — NOTE: static segments must be registered before :id ──────
        .route("/graph/path/{from}/{to}", get(graph::get_shortest_path))
        .route("/graph/correlations", get(graph::get_correlations))
        .route("/graph/{id}/network", get(graph::get_network))
        // ── Export ────────────────────────────────────────────────────────────
        .route(
            "/export/story-package/{id}",
            get(export::export_story_package),
        )
        // ── Submissions ─────────────────────────────────────────────────────
        .route(
            "/submissions",
            post(submissions::create_submission).get(submissions::list_submissions),
        )
        .route("/submissions/{id}", get(submissions::get_submission))
        .route(
            "/submissions/{id}/review",
            post(submissions::review_submission),
        )
        // ── Watchlist ─────────────────────────────────────────────────────────
        .route("/watchlist/subscribe", post(watchlist::subscribe))
        .route("/watchlist", get(watchlist::list_subscriptions))
        .route("/watchlist/{id}", delete(watchlist::delete_subscription))
        // ── Stats / Audit / Docs ─────────────────────────────────────────────
        .route("/stats", get(stats))
        .route("/audit/verify", get(verify_audit))
        .route("/docs", get(docs::api_docs));

    let x_request_id = HeaderName::from_static("x-request-id");

    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .nest("/api/v1", v1)
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(CompressionLayer::new())
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        .layer(SetRequestIdLayer::new(x_request_id, MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
}

/// GET /health — liveness probe (always 200).
async fn health_check() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "service": "nf-api",
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}

/// GET /ready — readiness probe that verifies database connectivity.
async fn readiness_check(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    use nf_store::repository::Repository;

    // Attempt a minimal DB query — list 0 entities just to check connectivity.
    match state.entity_repo.list(0, 1).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "status": "ready",
                "database": "connected",
            })),
        ),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not ready",
                "database": format!("error: {e}"),
            })),
        ),
    }
}

/// GET /api/v1/stats — database statistics.
async fn stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, crate::error::ApiError> {
    use nf_store::repository::Repository;

    let entities = state.entity_repo.list(0, 1).await?;
    let relationships = state.relationship_repo.list(0, 1).await?;

    let pending_submissions = {
        let queue = state.submission_queue.lock().unwrap();
        queue.pending().len()
    };

    let watchlist_count = state.watchlist.list().len();

    Ok(Json(json!({
        "entities": entities.total_count,
        "relationships": relationships.total_count,
        "pending_submissions": pending_submissions,
        "watchlist_subscriptions": watchlist_count,
    })))
}

/// GET /api/v1/audit/verify — verify audit chain integrity.
async fn verify_audit(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, crate::error::ApiError> {
    let pool = state.entity_repo.pool();
    let audit = nf_store::audit::AuditLog::new(pool.clone());

    let entries = audit
        .all_entries()
        .await
        .map_err(|e| crate::error::ApiError::Internal(e.to_string()))?;

    let valid = audit
        .verify_chain()
        .await
        .map_err(|e| crate::error::ApiError::Internal(e.to_string()))?;

    Ok(Json(json!({
        "chain_valid": valid,
        "total_entries": entries.len(),
        "status": if valid { "intact" } else { "COMPROMISED" },
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_health_check_response() {
        let (status, Json(body)) = health_check().await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["service"], "nf-api");
        assert!(body["version"].is_string());
    }
}
