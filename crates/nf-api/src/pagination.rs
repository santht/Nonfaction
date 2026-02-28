use serde::{Deserialize, Serialize};

use nf_store::repository::Page;

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
}

/// Paginated API response wrapper.
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total_in_page: usize,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn from_page(page: Page<T>, pagination: Pagination) -> Self {
        let total_in_page = page.items.len();
        Self {
            items: page.items,
            page: pagination.page,
            per_page: pagination.per_page(),
            total_in_page,
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
        let page: Page<i32> = Page::new(vec![1, 2, 3], 0, 20);
        let pagination = Pagination {
            page: 1,
            per_page: 20,
        };
        let response = PaginatedResponse::from_page(page, pagination);
        assert_eq!(response.items.len(), 3);
        assert_eq!(response.total_in_page, 3);
        assert_eq!(response.page, 1);
    }
}
