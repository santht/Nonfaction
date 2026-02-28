use axum::{
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::handlers::{entities, export, graph, search, watchlist};
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
///   POST /api/v1/watchlist/subscribe
///   GET  /api/v1/watchlist
///   DELETE /api/v1/watchlist/:id
pub fn build_router(state: AppState) -> Router {
    let v1 = Router::new()
        // ── Entities ─────────────────────────────────────────────────────────
        .route("/entities", get(entities::list_entities))
        .route("/entities/{id}", get(entities::get_entity))
        .route(
            "/entities/{id}/relationships",
            get(entities::get_entity_relationships),
        )
        .route("/entities/{id}/timeline", get(entities::get_entity_timeline))
        .route("/entities/{id}/sources", get(entities::get_entity_sources))
        // ── Search ────────────────────────────────────────────────────────────
        .route("/search", get(search::search))
        // ── Graph — NOTE: static segments must be registered before :id ──────
        .route(
            "/graph/path/{from}/{to}",
            get(graph::get_shortest_path),
        )
        .route(
            "/graph/correlations",
            get(graph::get_correlations),
        )
        .route("/graph/{id}/network", get(graph::get_network))
        // ── Export ────────────────────────────────────────────────────────────
        .route(
            "/export/story-package/{id}",
            get(export::export_story_package),
        )
        // ── Watchlist ─────────────────────────────────────────────────────────
        .route("/watchlist/subscribe", post(watchlist::subscribe))
        .route("/watchlist", get(watchlist::list_subscriptions))
        .route("/watchlist/{id}", delete(watchlist::delete_subscription));

    Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", v1)
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

/// GET /health — simple liveness probe.
async fn health_check() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "service": "nf-api",
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    // Note: full integration tests requiring AppState/DB are in each handler module.
    // Here we only test things that don't require live services.

    #[tokio::test]
    async fn test_health_check_response() {
        let (status, Json(body)) = health_check().await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["service"], "nf-api");
    }
}
