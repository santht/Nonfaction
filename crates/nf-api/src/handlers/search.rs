use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use nf_search::{SearchOptions, SearchResult};

use crate::error::ApiResult;
use crate::state::AppState;

// ─── Search query params ──────────────────────────────────────────────────────

/// Query parameters for GET /search
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// The full-text search query string.
    #[serde(rename = "q")]
    pub query: String,
    /// Optional entity type filter (Person, Organization, Payment, etc.)
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: u32,
    /// Number of results per page.
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    /// Optional date range filter: Unix timestamp lower bound.
    pub date_from: Option<i64>,
    /// Optional date range filter: Unix timestamp upper bound.
    pub date_to: Option<i64>,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

// ─── Search response ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub page: u32,
    pub per_page: u32,
    pub total_results: usize,
    pub results: Vec<SearchResultItem>,
    /// Facets: count by entity type.
    pub facets: Vec<FacetEntry>,
}

#[derive(Debug, Serialize)]
pub struct SearchResultItem {
    pub entity_id: String,
    pub entity_type: String,
    pub name: String,
    pub score: f32,
    pub snippets: Vec<String>,
    pub source_urls: Vec<String>,
    pub tags: Vec<String>,
}

impl From<SearchResult> for SearchResultItem {
    fn from(r: SearchResult) -> Self {
        Self {
            entity_id: r.entity_id,
            entity_type: r.entity_type,
            name: r.name,
            score: r.score,
            snippets: r.snippets,
            source_urls: r.source_urls,
            tags: r.tags,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FacetEntry {
    pub entity_type: String,
    pub count: usize,
}

// ─── Handler ─────────────────────────────────────────────────────────────────

/// GET /api/v1/search?q=query&type=Person&page=1&per_page=20
///
/// Full-text search with optional type filter, date range, and facets.
pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> ApiResult<Json<SearchResponse>> {
    let page = params.page.saturating_sub(1) as usize; // Convert 1-indexed to 0-indexed
    let per_page = params.per_page.min(100) as usize;
    let offset = page * per_page;

    let opts = SearchOptions::new()
        .with_limit(per_page)
        .with_offset(offset)
        .with_date_range(params.date_from, params.date_to);

    let opts = if let Some(ref et) = params.entity_type {
        opts.with_entity_type(et.as_str())
    } else {
        opts
    };

    let results: Vec<SearchResultItem> = state
        .searcher
        .search(&params.query, &opts)?
        .into_iter()
        .map(SearchResultItem::from)
        .collect();

    let total_results = results.len();

    // Fetch facets (type breakdown for the query).
    let facets: Vec<FacetEntry> = state
        .searcher
        .facet_by_type(&params.query)?
        .into_iter()
        .map(|(entity_type, count)| FacetEntry { entity_type, count })
        .collect();

    Ok(Json(SearchResponse {
        query: params.query,
        page: params.page,
        per_page: params.per_page,
        total_results,
        results,
        facets,
    }))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_page() {
        assert_eq!(default_page(), 1);
    }

    #[test]
    fn test_default_per_page() {
        assert_eq!(default_per_page(), 20);
    }

    #[test]
    fn test_search_result_item_from_search_result() {
        let sr = nf_search::SearchResult {
            entity_id: "abc-123".to_string(),
            entity_type: "Person".to_string(),
            name: "Jane Doe".to_string(),
            score: 1.5,
            snippets: vec!["<b>Jane</b> Doe".to_string()],
            source_urls: vec!["https://fec.gov/test".to_string()],
            tags: vec!["senator".to_string()],
        };

        let item = SearchResultItem::from(sr);
        assert_eq!(item.entity_id, "abc-123");
        assert_eq!(item.entity_type, "Person");
        assert_eq!(item.name, "Jane Doe");
        assert_eq!(item.score, 1.5);
        assert_eq!(item.snippets.len(), 1);
        assert_eq!(item.source_urls.len(), 1);
        assert_eq!(item.tags.len(), 1);
    }

    #[test]
    fn test_page_offset_calculation() {
        // Page 1, per_page 20 → offset 0
        let page: u32 = 1;
        let per_page: u32 = 20;
        let offset = (page.saturating_sub(1) as usize) * per_page as usize;
        assert_eq!(offset, 0);

        // Page 3, per_page 10 → offset 20
        let page: u32 = 3;
        let per_page: u32 = 10;
        let offset = (page.saturating_sub(1) as usize) * per_page as usize;
        assert_eq!(offset, 20);
    }
}
