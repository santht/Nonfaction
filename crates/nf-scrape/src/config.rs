//! Scraper configuration — API keys, rate limits, base URLs, scrape intervals.

use serde::{Deserialize, Serialize};

/// Global configuration for the scraper framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperConfig {
    pub fec: FecConfig,
    pub congress: CongressConfig,
    pub recap: RecapConfig,
    pub opensecrets_fec_bulk: OpenSecretsFecBulkConfig,
    pub pacer: PacerConfig,
}

impl Default for ScraperConfig {
    fn default() -> Self {
        Self {
            fec: FecConfig::default(),
            congress: CongressConfig::default(),
            recap: RecapConfig::default(),
            opensecrets_fec_bulk: OpenSecretsFecBulkConfig::default(),
            pacer: PacerConfig::default(),
        }
    }
}

/// Configuration shared by every source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Maximum requests per second (token-bucket refill rate).
    pub requests_per_second: f64,
    /// Initial burst capacity in tokens.
    pub burst_size: f64,
    /// Maximum number of retry attempts on transient failures.
    pub max_retries: u32,
    /// Base delay in milliseconds for exponential back-off.
    pub retry_base_delay_ms: u64,
    /// How often (in seconds) to run a full scrape of this source.
    pub scrape_interval_secs: u64,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 2.0,
            burst_size: 5.0,
            max_retries: 3,
            retry_base_delay_ms: 500,
            scrape_interval_secs: 3600,
        }
    }
}

// ── FEC ──────────────────────────────────────────────────────────────────────

/// Configuration for the FEC Open Data API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FecConfig {
    /// FEC API key — get one free at https://api.data.gov/signup/
    pub api_key: String,
    /// Base URL — override in tests to point at a mock server.
    pub base_url: String,
    /// Per-source rate-limit / retry settings.
    pub source: SourceConfig,
    /// Election cycle to scrape (e.g. "2024").
    pub election_cycle: String,
    /// Maximum number of pages to fetch per endpoint.
    pub max_pages: u32,
    /// Results per page requested from the API.
    pub per_page: u32,
}

impl Default for FecConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.open.fec.gov/v1".to_string(),
            source: SourceConfig {
                requests_per_second: 1.0, // FEC allows ~1 req/s on free tier
                burst_size: 3.0,
                ..Default::default()
            },
            election_cycle: "2024".to_string(),
            max_pages: 100,
            per_page: 100,
        }
    }
}

// ── Congress.gov ─────────────────────────────────────────────────────────────

/// Configuration for the Congress.gov API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CongressConfig {
    /// Congress.gov API key — sign up at https://api.congress.gov/sign-up/
    pub api_key: String,
    /// Base URL — override in tests.
    pub base_url: String,
    /// Per-source rate-limit / retry settings.
    pub source: SourceConfig,
    /// Congress number to scrape (e.g. 118 for the 118th Congress).
    pub congress_number: u32,
    /// Maximum pages per endpoint.
    pub max_pages: u32,
    /// Results per page.
    pub per_page: u32,
}

impl Default for CongressConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.congress.gov/v3".to_string(),
            source: SourceConfig {
                requests_per_second: 5.0,
                burst_size: 10.0,
                ..Default::default()
            },
            congress_number: 118,
            max_pages: 50,
            per_page: 250,
        }
    }
}

// ── RECAP / CourtListener ────────────────────────────────────────────────────

/// Configuration for the CourtListener / RECAP REST API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecapConfig {
    /// CourtListener API token (optional — unauthenticated requests are rate-limited).
    pub api_token: Option<String>,
    /// Base URL — override in tests.
    pub base_url: String,
    /// Per-source rate-limit / retry settings.
    pub source: SourceConfig,
    /// Maximum pages per endpoint.
    pub max_pages: u32,
    /// Results per page.
    pub per_page: u32,
}

impl Default for RecapConfig {
    fn default() -> Self {
        Self {
            api_token: None,
            base_url: "https://www.courtlistener.com/api/rest/v3".to_string(),
            source: SourceConfig {
                requests_per_second: 2.0,
                burst_size: 4.0,
                ..Default::default()
            },
            max_pages: 50,
            per_page: 100,
        }
    }
}

// ── OpenSecrets/FEC bulk ────────────────────────────────────────────────────

/// Configuration for OpenSecrets/FEC-style bulk campaign finance ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSecretsFecBulkConfig {
    /// Optional API token for OpenSecrets bulk endpoints.
    pub api_token: Option<String>,
    /// Base URL for bulk endpoint API.
    pub base_url: String,
    /// Per-source rate-limit / retry settings.
    pub source: SourceConfig,
    /// Maximum pages to ingest per run.
    pub max_pages: u32,
    /// Rows per page.
    pub per_page: u32,
    /// Election cycle to filter bulk rows.
    pub election_cycle: String,
}

impl Default for OpenSecretsFecBulkConfig {
    fn default() -> Self {
        Self {
            api_token: None,
            base_url: "https://www.opensecrets.org/api/v1".to_string(),
            source: SourceConfig {
                requests_per_second: 1.0,
                burst_size: 2.0,
                ..Default::default()
            },
            max_pages: 50,
            per_page: 500,
            election_cycle: "2024".to_string(),
        }
    }
}

// ── PACER ────────────────────────────────────────────────────────────────────

/// Configuration for PACER-compatible case APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacerConfig {
    /// PACER API token, if using an authenticated gateway.
    pub api_token: Option<String>,
    /// Base URL for PACER case API.
    pub base_url: String,
    /// Per-source rate-limit / retry settings.
    pub source: SourceConfig,
    /// Maximum pages to scrape.
    pub max_pages: u32,
    /// Results per page.
    pub per_page: u32,
}

impl Default for PacerConfig {
    fn default() -> Self {
        Self {
            api_token: None,
            base_url: "https://api.pacer.uscourts.gov/v1".to_string(),
            source: SourceConfig {
                requests_per_second: 1.0,
                burst_size: 2.0,
                ..Default::default()
            },
            max_pages: 50,
            per_page: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configs_are_valid() {
        let cfg = ScraperConfig::default();
        assert!(!cfg.fec.base_url.is_empty());
        assert!(!cfg.congress.base_url.is_empty());
        assert!(!cfg.recap.base_url.is_empty());
        assert!(!cfg.opensecrets_fec_bulk.base_url.is_empty());
        assert!(!cfg.pacer.base_url.is_empty());
        assert!(cfg.fec.source.requests_per_second > 0.0);
        assert!(cfg.congress.source.burst_size > 0.0);
    }

    #[test]
    fn fec_config_defaults() {
        let cfg = FecConfig::default();
        assert_eq!(cfg.base_url, "https://api.open.fec.gov/v1");
        assert_eq!(cfg.election_cycle, "2024");
        assert_eq!(cfg.per_page, 100);
    }

    #[test]
    fn recap_config_optional_token() {
        let cfg = RecapConfig::default();
        assert!(cfg.api_token.is_none());
    }

    #[test]
    fn opensecrets_bulk_defaults() {
        let cfg = OpenSecretsFecBulkConfig::default();
        assert!(cfg.base_url.contains("opensecrets"));
        assert_eq!(cfg.election_cycle, "2024");
    }

    #[test]
    fn pacer_config_defaults() {
        let cfg = PacerConfig::default();
        assert!(cfg.base_url.contains("pacer"));
        assert!(cfg.api_token.is_none());
    }
}
