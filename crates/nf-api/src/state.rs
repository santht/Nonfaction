use std::sync::{Arc, Mutex};

use nf_crowd::submission::SubmissionQueue;
use nf_search::{NfSchema, NfSearcher};
use nf_store::DbPool;
use nf_store::repository::{EntityRepository, RelationshipRepository};

use crate::handlers::watchlist::WatchlistStore;

/// Shared application state injected into every handler via Axum's `State` extractor.
///
/// Wrapped in `Arc` so it can be cloned cheaply across requests.
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL entity + relationship repository.
    pub entity_repo: Arc<EntityRepository>,
    pub relationship_repo: Arc<RelationshipRepository>,
    /// Tantivy full-text searcher.
    pub searcher: Arc<NfSearcher>,
    /// Tantivy index schema (needed for query building).
    pub search_schema: Arc<NfSchema>,
    /// In-memory watchlist store.
    pub watchlist: WatchlistStore,
    /// Crowd-sourced submission queue.
    pub submission_queue: Arc<Mutex<SubmissionQueue>>,
}

impl AppState {
    /// Construct AppState from an existing DB pool and a Tantivy index.
    pub fn new(
        pool: DbPool,
        tantivy_index: tantivy::Index,
        search_schema: Arc<NfSchema>,
    ) -> Result<Self, nf_search::SearchError> {
        let searcher = NfSearcher::new(&tantivy_index, search_schema.clone())?;
        Ok(Self {
            entity_repo: Arc::new(EntityRepository::new(pool.clone())),
            relationship_repo: Arc::new(RelationshipRepository::new(pool)),
            searcher: Arc::new(searcher),
            search_schema,
            watchlist: WatchlistStore::new(),
            submission_queue: Arc::new(Mutex::new(SubmissionQueue::new())),
        })
    }
}
