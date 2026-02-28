// nf-api: Axum REST API for the Nonfaction political accountability platform.
//
// Exposes versioned endpoints under /api/v1/ for entities, search, graph
// analysis, story package export, and watchlist subscriptions.

pub mod error;
pub mod handlers;
pub mod pagination;
pub mod rate_limit;
pub mod router;
pub mod state;

pub use error::{ApiError, ApiResult};
pub use router::build_router;
pub use state::AppState;
