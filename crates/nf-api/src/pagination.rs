use serde::{Deserialize, Serialize};

use nf_store::repository::Page;

use crate::error::{ApiError, ApiResult};

/// Common pagination query parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct Pagination {
    /// Page number, 1-indexed. Defaults to 1.
    #[serde(default = "default_page")]
    pub page: u32,
    /// Items per page. Defaults to 20, max 100.
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

impl Pagination {
    /// Convert to zero-indexed page number for the repository.
    pub fn page(&self) -> u32 {
        self.page.saturating_sub(1)
    }

    /// Clamp per_page to a maximum of 100.
    pub fn per_page(&self) -> u32 {
        self.per_page.clamp(1, 100)
    }

    /// Validate that page >= 1 and per_page is between 1 and 100.
    pub fn validate(&self) -> ApiResult<()> {
        if self.page < 1 {
            return Err(ApiError::InvalidPageNumber(
                "page must be >= 1".to_string(),
            ));
        }
        if self.per_page < 1 || self.per_page > 100 {
            return Err(ApiError::InvalidPageSize(
                "page_size must be between 1 and 100".to_string(),
            ));
        }
        Ok(())
    }
}

/// Strip HTML tags from a string using a simple state machine.
fn strip_html_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}

/// Sanitize a search query string.
///
/// - Strips HTML tags
/// - Trims leading/trailing whitespace
/// - Limits length to 500 characters
/// - Rejects queries that contain no alphanumeric characters
///
/// Returns the sanitized query or an `InvalidSearchQuery` error.
pub fn sanitize_search_query(query: &str) -> ApiResult<String> {
    let stripped = strip_html_tags(query);
    let trimmed = stripped.trim();

    // Enforce length limit before further checks.
    let limited: &str = if trimmed.len() > 500 {
        &trimmed[..500]
    } else {
        trimmed
    };

    // Reject empty queries.
    if limited.is_empty() {
        return Err(ApiError::InvalidSearchQuery(
            "search query must not be empty".to_string(),
        ));
    }

    // Reject queries with no alphanumeric characters.
    if !limited.chars().any(|c| c.is_alphanumeric()) {
        return Err(ApiError::InvalidSearchQuery(
            "search query must contain at least one alphanumeric character".to_string(),
        ));
    }

    Ok(limited.to_string())
}

/// Paginated API response wrapper.
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total_in_page: usize,
    pub total_count: i64,
    pub total_pages: u32,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn from_page(page: Page<T>, pagination: Pagination) -> Self {
        let total_in_page = page.items.len();
        let total_count = page.total_count;
        let total_pages = page.total_pages();
        Self {
            items: page.items,
            page: pagination.page,
            per_page: pagination.per_page(),
            total_in_page,
            total_count,
            total_pages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_page_zero_indexed() {
        let p = Pagination {
            page: 1,
            per_page: 20,
        };
        assert_eq!(p.page(), 0);

        let p = Pagination {
            page: 3,
            per_page: 10,
        };
        assert_eq!(p.page(), 2);
    }

    #[test]
    fn test_pagination_per_page_clamped() {
        let p = Pagination {
            page: 1,
            per_page: 200,
        };
        assert_eq!(p.per_page(), 100);

        let p = Pagination {
            page: 1,
            per_page: 0,
        };
        assert_eq!(p.per_page(), 1);
    }

    #[test]
    fn test_paginated_response_from_page() {
        let page: Page<i32> = Page::with_total(vec![1, 2, 3], 0, 20, 50);
        let pagination = Pagination {
            page: 1,
            per_page: 20,
        };
        let response = PaginatedResponse::from_page(page, pagination);
        assert_eq!(response.items.len(), 3);
        assert_eq!(response.total_in_page, 3);
        assert_eq!(response.page, 1);
        assert_eq!(response.total_count, 50);
        assert_eq!(response.total_pages, 3);
    }

    // ─── Pagination::validate() tests ─────────────────────────────────────────

    #[test]
    fn test_validate_accepts_valid_pagination() {
        let p = Pagination { page: 1, per_page: 20 };
        assert!(p.validate().is_ok());

        let p = Pagination { page: 5, per_page: 100 };
        assert!(p.validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_page_zero() {
        let p = Pagination { page: 0, per_page: 20 };
        let err = p.validate().unwrap_err();
        assert!(matches!(err, ApiError::InvalidPageNumber(_)));
    }

    #[test]
    fn test_validate_rejects_per_page_zero() {
        let p = Pagination { page: 1, per_page: 0 };
        let err = p.validate().unwrap_err();
        assert!(matches!(err, ApiError::InvalidPageSize(_)));
    }

    #[test]
    fn test_validate_rejects_per_page_over_100() {
        let p = Pagination { page: 1, per_page: 101 };
        let err = p.validate().unwrap_err();
        assert!(matches!(err, ApiError::InvalidPageSize(_)));
    }

    #[test]
    fn test_validate_boundary_per_page_1_and_100() {
        assert!(Pagination { page: 1, per_page: 1 }.validate().is_ok());
        assert!(Pagination { page: 1, per_page: 100 }.validate().is_ok());
    }

    // ─── sanitize_search_query() tests ────────────────────────────────────────

    #[test]
    fn test_sanitize_strips_html_tags() {
        let result = sanitize_search_query("<b>Jane Doe</b>").unwrap();
        assert_eq!(result, "Jane Doe");
    }

    #[test]
    fn test_sanitize_trims_whitespace() {
        let result = sanitize_search_query("  hello world  ").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_sanitize_limits_to_500_chars() {
        let long_query = "a".repeat(600);
        let result = sanitize_search_query(&long_query).unwrap();
        assert_eq!(result.len(), 500);
    }

    #[test]
    fn test_sanitize_rejects_empty_query() {
        assert!(sanitize_search_query("").is_err());
        assert!(sanitize_search_query("   ").is_err());
    }

    #[test]
    fn test_sanitize_rejects_whitespace_only_after_strip() {
        // HTML tags only, no text content
        assert!(sanitize_search_query("<b>   </b>").is_err());
    }

    #[test]
    fn test_sanitize_rejects_special_chars_only() {
        let err = sanitize_search_query("!@#$%^&*()").unwrap_err();
        assert!(matches!(err, ApiError::InvalidSearchQuery(_)));
    }

    #[test]
    fn test_sanitize_accepts_valid_query() {
        let result = sanitize_search_query("official campaign donations").unwrap();
        assert_eq!(result, "official campaign donations");
    }

    #[test]
    fn test_sanitize_accepts_query_with_numbers() {
        let result = sanitize_search_query("2024 election").unwrap();
        assert_eq!(result, "2024 election");
    }

    #[test]
    fn test_strip_html_tags_nested() {
        assert_eq!(strip_html_tags("<div><span>text</span></div>"), "text");
    }

    #[test]
    fn test_strip_html_tags_no_tags() {
        assert_eq!(strip_html_tags("plain text"), "plain text");
    }
}
