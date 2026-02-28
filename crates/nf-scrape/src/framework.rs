//! Core scraper framework: trait, rate limiter, retry, deduplication.

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, warn};
use url::Url;

use nf_core::Entity;

use crate::config::SourceConfig;

// ── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ScrapeError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("URL already seen (dedup): {0}")]
    Duplicate(String),

    #[error("max retries exceeded for {url}: {cause}")]
    MaxRetriesExceeded { url: String, cause: String },

    #[error("configuration error: {0}")]
    Config(String),

    #[error("transform error: {0}")]
    Transform(String),
}

pub type ScrapeResult<T> = Result<T, ScrapeError>;

// ── Rate limiter (token bucket) ───────────────────────────────────────────────

/// Thread-safe token-bucket rate limiter.
pub struct RateLimiter {
    tokens: f64,
    max_tokens: f64,
    /// Tokens added per second.
    refill_rate: f64,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(rate_per_second: f64, burst: f64) -> Self {
        Self {
            tokens: burst,
            max_tokens: burst,
            refill_rate: rate_per_second,
            last_refill: Instant::now(),
        }
    }

    /// Unlimited limiter — always grants tokens immediately (for testing).
    pub fn unlimited() -> Self {
        Self::new(f64::MAX / 2.0, f64::MAX / 2.0)
    }

    /// Try to consume one token. Returns `true` if the token was granted.
    pub fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Duration to wait before the next token becomes available.
    pub fn wait_time(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64((1.0 - self.tokens) / self.refill_rate.max(1e-9))
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }
}

// ── Retry configuration ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl RetryConfig {
    pub fn from_source_config(cfg: &SourceConfig) -> Self {
        Self {
            max_retries: cfg.max_retries,
            base_delay: Duration::from_millis(cfg.retry_base_delay_ms),
            max_delay: Duration::from_secs(60),
        }
    }

    /// Instant retry — zero delay, zero retries (for testing).
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            base_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
        }
    }

    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        let exp = self.base_delay.saturating_mul(2u32.saturating_pow(attempt));
        exp.min(self.max_delay)
    }
}

// ── Scraper runtime ───────────────────────────────────────────────────────────

/// Shared runtime used by all scrapers: HTTP client, rate limiter,
/// seen-URL dedup set, and retry policy.
pub struct ScraperRuntime {
    pub client: reqwest::Client,
    limiter: Arc<Mutex<RateLimiter>>,
    seen_urls: Arc<Mutex<HashSet<String>>>,
    retry_config: RetryConfig,
}

impl ScraperRuntime {
    pub fn new(cfg: &SourceConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            limiter: Arc::new(Mutex::new(RateLimiter::new(
                cfg.requests_per_second,
                cfg.burst_size,
            ))),
            seen_urls: Arc::new(Mutex::new(HashSet::new())),
            retry_config: RetryConfig::from_source_config(cfg),
        }
    }

    /// Create a runtime that imposes no rate limits or retries — useful for tests.
    pub fn new_unlimited() -> Self {
        let cfg = SourceConfig {
            requests_per_second: f64::MAX / 2.0,
            burst_size: f64::MAX / 2.0,
            max_retries: 0,
            retry_base_delay_ms: 0,
            scrape_interval_secs: 0,
        };
        Self::new(&cfg)
    }

    /// Check and mark a URL as seen. Returns `true` if this is the first time.
    pub async fn mark_seen(&self, url: &str) -> bool {
        let mut seen = self.seen_urls.lock().await;
        seen.insert(url.to_string())
    }

    /// Wait for a rate-limit token, then return.
    pub async fn wait_for_token(&self) {
        loop {
            let wait = {
                let mut limiter = self.limiter.lock().await;
                if limiter.try_acquire() {
                    Duration::ZERO
                } else {
                    limiter.wait_time()
                }
            };
            if wait.is_zero() {
                break;
            }
            debug!("rate limiter: sleeping {}ms", wait.as_millis());
            sleep(wait).await;
        }
    }

    /// Fetch a URL as parsed JSON with rate limiting, deduplication, and retry.
    ///
    /// Returns `None` if the URL was already fetched (dedup suppression).
    pub async fn fetch_json(&self, url: &Url) -> ScrapeResult<Option<serde_json::Value>> {
        let url_str = url.to_string();

        if !self.mark_seen(&url_str).await {
            warn!("skipping already-seen URL: {}", url_str);
            return Ok(None);
        }

        self.wait_for_token().await;

        let mut attempt = 0u32;
        loop {
            match self.client.get(url.clone()).send().await {
                Ok(resp) => match resp.json::<serde_json::Value>().await {
                    Ok(json) => return Ok(Some(json)),
                    Err(e) if attempt < self.retry_config.max_retries => {
                        attempt += 1;
                        let delay = self.retry_config.backoff_delay(attempt - 1);
                        warn!("JSON parse failed (attempt {}), retrying in {}ms: {}", attempt, delay.as_millis(), e);
                        sleep(delay).await;
                    }
                    Err(e) => {
                        return Err(ScrapeError::MaxRetriesExceeded {
                            url: url_str,
                            cause: e.to_string(),
                        });
                    }
                },
                Err(e) if attempt < self.retry_config.max_retries => {
                    attempt += 1;
                    let delay = self.retry_config.backoff_delay(attempt - 1);
                    warn!("HTTP error (attempt {}), retrying in {}ms: {}", attempt, delay.as_millis(), e);
                    sleep(delay).await;
                }
                Err(e) => {
                    return Err(ScrapeError::MaxRetriesExceeded {
                        url: url_str,
                        cause: e.to_string(),
                    });
                }
            }
        }
    }
}

// ── ScrapeSource trait ────────────────────────────────────────────────────────

/// Core trait implemented by every source-specific scraper.
///
/// The trait is intentionally object-safe: `scrape_all` returns a boxed future
/// so that scrapers can be stored as `Box<dyn ScrapeSource>` in a registry.
pub trait ScrapeSource: Send + Sync {
    /// Unique stable identifier for this source (e.g. "fec", "congress").
    fn source_id(&self) -> &str;

    /// Run a complete scrape of all available data for this source.
    ///
    /// Implementations should page through all available results and transform
    /// every record into one or more nf-core entities, each with a valid SourceRef.
    fn scrape_all<'a>(
        &'a self,
        runtime: &'a ScraperRuntime,
    ) -> Pin<Box<dyn Future<Output = ScrapeResult<Vec<Entity>>> + Send + 'a>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_grants_burst() {
        let mut rl = RateLimiter::new(1.0, 3.0);
        assert!(rl.try_acquire());
        assert!(rl.try_acquire());
        assert!(rl.try_acquire());
        assert!(!rl.try_acquire()); // burst exhausted
    }

    #[test]
    fn rate_limiter_unlimited_always_grants() {
        let mut rl = RateLimiter::unlimited();
        for _ in 0..1000 {
            assert!(rl.try_acquire());
        }
    }

    #[test]
    fn retry_config_backoff() {
        let cfg = RetryConfig {
            max_retries: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
        };
        assert_eq!(cfg.backoff_delay(0), Duration::from_millis(100));
        assert_eq!(cfg.backoff_delay(1), Duration::from_millis(200));
        assert_eq!(cfg.backoff_delay(2), Duration::from_millis(400));
        // capped at max_delay
        assert_eq!(cfg.backoff_delay(20), Duration::from_secs(10));
    }

    #[tokio::test]
    async fn runtime_dedup_blocks_second_fetch() {
        let rt = ScraperRuntime::new_unlimited();
        assert!(rt.mark_seen("https://example.com/api").await);
        assert!(!rt.mark_seen("https://example.com/api").await);
    }
}
