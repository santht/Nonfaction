use axum::Json;
use serde::Serialize;

use crate::error::ApiResult;

/// API route documentation entry.
#[derive(Debug, Serialize)]
pub struct RouteDoc {
    pub method: &'static str,
    pub path: &'static str,
    pub description: &'static str,
    pub auth_required: bool,
}

/// GET /api/v1/docs — list all available API routes.
pub async fn api_docs() -> ApiResult<Json<ApiDocsResponse>> {
    Ok(Json(ApiDocsResponse {
        name: "Nonfaction API",
        version: "v1",
        description: "Transparent, open-source political accountability database API",
        license: "GPL-3.0-only (code), CC-BY-SA-4.0 (data)",
        routes: vec![
            // Entities
            RouteDoc {
                method: "GET",
                path: "/api/v1/entities",
                description: "List all entities with pagination and optional type filter",
                auth_required: false,
            },
            RouteDoc {
                method: "POST",
                path: "/api/v1/entities",
                description: "Create a new entity (requires valid source chain)",
                auth_required: false,
            },
            RouteDoc {
                method: "GET",
                path: "/api/v1/entities/:id",
                description: "Get a single entity by UUID",
                auth_required: false,
            },
            RouteDoc {
                method: "GET",
                path: "/api/v1/entities/:id/relationships",
                description: "Get all outgoing and incoming relationships for an entity",
                auth_required: false,
            },
            RouteDoc {
                method: "POST",
                path: "/api/v1/entities/:id/relationships",
                description: "Create a relationship originating from this entity",
                auth_required: false,
            },
            RouteDoc {
                method: "GET",
                path: "/api/v1/entities/:id/timeline",
                description: "Get entity relationships sorted chronologically",
                auth_required: false,
            },
            RouteDoc {
                method: "GET",
                path: "/api/v1/entities/:id/sources",
                description: "Get all source references for an entity",
                auth_required: false,
            },
            // Search
            RouteDoc {
                method: "GET",
                path: "/api/v1/search",
                description: "Full-text search across all entities with faceting",
                auth_required: false,
            },
            // Graph
            RouteDoc {
                method: "GET",
                path: "/api/v1/graph/:id/network",
                description: "BFS network traversal from an entity (Cytoscape.js format)",
                auth_required: false,
            },
            RouteDoc {
                method: "GET",
                path: "/api/v1/graph/path/:from/:to",
                description: "Shortest path between two entities",
                auth_required: false,
            },
            RouteDoc {
                method: "GET",
                path: "/api/v1/graph/correlations",
                description: "Timing correlations for an official",
                auth_required: false,
            },
            // Export
            RouteDoc {
                method: "GET",
                path: "/api/v1/export/story-package/:id",
                description: "Download story package ZIP (entity + sources + graph)",
                auth_required: false,
            },
            // Submissions
            RouteDoc {
                method: "POST",
                path: "/api/v1/submissions",
                description: "Submit a new crowdsourced record for review",
                auth_required: false,
            },
            RouteDoc {
                method: "GET",
                path: "/api/v1/submissions",
                description: "List submissions (optionally filtered by status)",
                auth_required: false,
            },
            RouteDoc {
                method: "GET",
                path: "/api/v1/submissions/:id",
                description: "Get a single submission by ID",
                auth_required: false,
            },
            RouteDoc {
                method: "POST",
                path: "/api/v1/submissions/:id/review",
                description: "Review a submission (claim/approve/reject)",
                auth_required: false,
            },
            // Watchlist
            RouteDoc {
                method: "POST",
                path: "/api/v1/watchlist/subscribe",
                description: "Subscribe to entity update notifications",
                auth_required: false,
            },
            RouteDoc {
                method: "GET",
                path: "/api/v1/watchlist",
                description: "List all active watchlist subscriptions",
                auth_required: false,
            },
            RouteDoc {
                method: "DELETE",
                path: "/api/v1/watchlist/:id",
                description: "Remove a watchlist subscription",
                auth_required: false,
            },
            // Stats / Audit
            RouteDoc {
                method: "GET",
                path: "/api/v1/stats",
                description: "Database statistics (entity/relationship/submission counts)",
                auth_required: false,
            },
            RouteDoc {
                method: "GET",
                path: "/api/v1/audit/verify",
                description: "Verify hash-chained audit log integrity",
                auth_required: false,
            },
            RouteDoc {
                method: "GET",
                path: "/api/v1/docs",
                description: "This endpoint — API documentation",
                auth_required: false,
            },
            // Health/Readiness (not under /api/v1 but documented for completeness)
            RouteDoc {
                method: "GET",
                path: "/health",
                description: "Liveness probe — always returns 200",
                auth_required: false,
            },
            RouteDoc {
                method: "GET",
                path: "/ready",
                description: "Readiness probe — verifies database connectivity",
                auth_required: false,
            },
        ],
    }))
}

#[derive(Debug, Serialize)]
pub struct ApiDocsResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub license: &'static str,
    pub routes: Vec<RouteDoc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_docs_response() {
        let Json(docs) = api_docs().await.unwrap();
        assert_eq!(docs.name, "Nonfaction API");
        assert_eq!(docs.version, "v1");
        assert!(!docs.routes.is_empty());
        // Verify all routes have non-empty fields
        for route in &docs.routes {
            assert!(!route.method.is_empty());
            assert!(
                route.path.starts_with("/api/v1/")
                    || route.path == "/health"
                    || route.path == "/ready",
                "unexpected route path: {}",
                route.path
            );
            assert!(!route.description.is_empty());
        }
    }

    #[tokio::test]
    async fn test_api_docs_route_count() {
        let Json(docs) = api_docs().await.unwrap();
        // We should have 22 documented routes
        assert!(
            docs.routes.len() >= 20,
            "expected at least 20 routes, got {}",
            docs.routes.len()
        );
    }
}
