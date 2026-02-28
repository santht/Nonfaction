// nf-scrape: Async scraper framework with rate limiting, dedup, retry

pub mod config;
pub mod framework;
pub mod sources;

pub use config::{FecConfig, CongressConfig, RecapConfig, ScraperConfig, SourceConfig};
pub use framework::{ScrapeError, ScrapeResult, ScraperRuntime, ScrapeSource};
