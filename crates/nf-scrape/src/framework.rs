//! Core scraper framework: traits, rate limiter, retry, deduplication.

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::header::RETRY_AFTER;
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

    #[error("HTTP status {status} for {url}: {body}")]
    HttpStatus {
        url: String,
        status: u16,
        body: String,
    },

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("URL already seen (dedup): {0}")]
    Duplicate(String),

    #[error("max retries exceeded for {url} after {attempts} attempts: {cause}")]
    MaxRetriesExceeded {
        url: String,
        attempts: u32,
        cause: String,
    },

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
        self.fetch_json_with(url, |request| request).await
    }

    /// Same as `fetch_json`, but lets callers modify the request builder
    /// (e.g. inject auth headers) before each attempt.
    pub async fn fetch_json_with<F>(
        &self,
        url: &Url,
        configure_request: F,
    ) -> ScrapeResult<Option<serde_json::Value>>
    where
        F: Fn(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        let url_str = url.to_string();

        if !self.mark_seen(&url_str).await {
            warn!("skipping already-seen URL: {}", url_str);
            return Ok(None);
        }

        let mut last_error = String::new();

        for attempt in 0..=self.retry_config.max_retries {
            self.wait_for_token().await;

            let request = configure_request(self.client.get(url.clone()));
            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        match response.json::<serde_json::Value>().await {
                            Ok(json) => return Ok(Some(json)),
                            Err(err) => {
                                last_error = err.to_string();
                                if attempt < self.retry_config.max_retries
                                    && is_retryable_reqwest_error(&err)
                                {
                                    let delay = self.retry_config.backoff_delay(attempt);
                                    warn!(
                                        "JSON parse request error (attempt {}), retrying in {}ms: {}",
                                        attempt + 1,
                                        delay.as_millis(),
                                        last_error
                                    );
                                    sleep(delay).await;
                                    continue;
                                }
                                return Err(ScrapeError::MaxRetriesExceeded {
                                    url: url_str,
                                    attempts: attempt + 1,
                                    cause: last_error,
                                });
                            }
                        }
                    }

                    // Extract retry-after header before consuming response with .text()
                    let retry_after = response.headers().get(RETRY_AFTER).cloned();

                    let response_body = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read response body>".to_string());
                    let body_excerpt = truncate_for_error(&response_body, 256);
                    last_error = format!("status {}: {}", status.as_u16(), body_excerpt);

                    if attempt < self.retry_config.max_retries && is_retryable_status(status) {
                        let delay = retry_delay(&self.retry_config, attempt, retry_after.as_ref());
                        warn!(
                            "HTTP {} (attempt {}), retrying in {}ms: {}",
                            status.as_u16(),
                            attempt + 1,
                            delay.as_millis(),
                            url,
                        );
                        sleep(delay).await;
                        continue;
                    }

                    return Err(ScrapeError::HttpStatus {
                        url: url_str,
                        status: status.as_u16(),
                        body: body_excerpt,
                    });
                }
                Err(err) => {
                    last_error = err.to_string();
                    if attempt < self.retry_config.max_retries && is_retryable_reqwest_error(&err) {
                        let delay = self.retry_config.backoff_delay(attempt);
                        warn!(
                            "HTTP error (attempt {}), retrying in {}ms: {}",
                            attempt + 1,
                            delay.as_millis(),
                            last_error
                        );
                        sleep(delay).await;
                        continue;
                    }

                    return Err(ScrapeError::MaxRetriesExceeded {
                        url: url_str,
                        attempts: attempt + 1,
                        cause: last_error,
                    });
                }
            }
        }

        Err(ScrapeError::MaxRetriesExceeded {
            url: url_str,
            attempts: self.retry_config.max_retries + 1,
            cause: last_error,
        })
    }
}

fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::REQUEST_TIMEOUT
        || status.is_server_error()
}

fn is_retryable_reqwest_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request()
}

fn retry_delay(
    retry: &RetryConfig,
    attempt: u32,
    retry_after_header: Option<&reqwest::header::HeaderValue>,
) -> Duration {
    let backoff = retry.backoff_delay(attempt);
    let retry_after = retry_after_header
        .and_then(|h| h.to_str().ok())
        .and_then(parse_retry_after_seconds)
        .map(Duration::from_secs);

    retry_after.unwrap_or(backoff).min(retry.max_delay)
}

fn parse_retry_after_seconds(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok()
}

fn truncate_for_error(message: &str, max_chars: usize) -> String {
    if message.chars().count() <= max_chars {
        return message.to_string();
    }
    let mut out = String::with_capacity(max_chars + 3);
    for ch in message.chars().take(max_chars) {
        out.push(ch);
    }
    out.push_str("...");
    out
}

// ── Scraper traits ───────────────────────────────────────────────────────────

/// Per-source scraper helper trait with built-in runtime-backed
/// rate limiting and retry-aware fetch helpers.
pub trait Scraper: Send + Sync {
    /// Unique source identifier (e.g. `fec`, `congress`).
    fn source_id(&self) -> &str;

    /// Source-level runtime settings (rate/retry).
    fn source_config(&self) -> &SourceConfig;

    /// Optional request customization hook (e.g. auth headers).
    fn prepare_request(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
    }

    /// Fetch JSON using shared runtime controls.
    fn fetch_json<'a>(
        &'a self,
        runtime: &'a ScraperRuntime,
        url: &'a Url,
    ) -> Pin<Box<dyn Future<Output = ScrapeResult<Option<serde_json::Value>>> + Send + 'a>> {
        Box::pin(async move {
            runtime
                .fetch_json_with(url, |request| self.prepare_request(request))
                .await
        })
    }
}

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
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
        assert_eq!(cfg.backoff_delay(20), Duration::from_secs(10));
    }

    #[test]
    fn parse_retry_after_header() {
        assert_eq!(parse_retry_after_seconds("5"), Some(5));
        assert_eq!(parse_retry_after_seconds(" 12 "), Some(12));
        assert_eq!(parse_retry_after_seconds("not-a-number"), None);
    }

    #[tokio::test]
    async fn runtime_dedup_blocks_second_fetch() {
        let rt = ScraperRuntime::new_unlimited();
        assert!(rt.mark_seen("https://example.com/api").await);
        assert!(!rt.mark_seen("https://example.com/api").await);
    }

    #[tokio::test]
    async fn fetch_json_retries_retryable_status_then_fails() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/always-500"))
            .respond_with(
                ResponseTemplate::new(500)
                    .insert_header("Retry-After", "0")
                    .set_body_string("temporary outage"),
            )
            .mount(&server)
            .await;

        let cfg = SourceConfig {
            requests_per_second: 1000.0,
            burst_size: 1000.0,
            max_retries: 2,
            retry_base_delay_ms: 0,
            scrape_interval_secs: 0,
        };
        let runtime = ScraperRuntime::new(&cfg);
        let url = Url::parse(&format!("{}/always-500", server.uri())).unwrap();

        let err = runtime.fetch_json(&url).await.unwrap_err();
        match err {
            ScrapeError::HttpStatus { status, .. } => assert_eq!(status, 500),
            other => panic!("unexpected error variant: {other:?}"),
        }
    }
}
