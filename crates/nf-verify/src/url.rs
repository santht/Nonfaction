use crate::error::{VerifyError, VerifyResult};
use std::time::Instant;
use url::Url;

/// HTTP status codes considered transient and eligible for retry.
const TRANSIENT_STATUS_CODES: &[u16] = &[429, 500, 502, 503, 504];
/// Exponential backoff delays (ms) for each retry attempt.
const RETRY_BACKOFF_MS: [u64; 3] = [100, 200, 400];

/// Result of checking a URL's liveness
#[derive(Debug, Clone)]
pub struct UrlVerifyResult {
    pub url: String,
    /// Whether the URL returned a 2xx response
    pub is_live: bool,
    /// HTTP status code returned
    pub status_code: Option<u16>,
    /// Whether an Internet Archive snapshot was found
    pub archive_available: bool,
    /// The Internet Archive snapshot URL, if found
    pub archive_url: Option<String>,
}

/// Result of archiving a URL via the Wayback Machine save API
#[derive(Debug, Clone)]
pub struct ArchiveSaveResult {
    /// The original URL that was submitted for archival
    pub original_url: String,
    /// The Wayback Machine URL of the saved snapshot, if available immediately
    pub snapshot_url: Option<String>,
    /// Whether the save request was accepted (202) or already cached (200)
    pub accepted: bool,
    /// Raw job-id returned by the API, for async polling
    pub job_id: Option<String>,
}

/// Result of checking a URL's freshness via response headers
#[derive(Debug, Clone)]
pub struct FreshnessResult {
    pub url: String,
    /// Whether the URL is currently accessible
    pub is_accessible: bool,
    /// `Last-Modified` header value, if returned
    pub last_modified: Option<String>,
    /// `ETag` header value, if returned
    pub etag: Option<String>,
    /// `Content-Type` header value
    pub content_type: Option<String>,
    /// Round-trip time in milliseconds for the HEAD request
    pub response_time_ms: u64,
    /// Final URL after any redirects
    pub redirected_to: Option<String>,
}

const WAYBACK_API_BASE: &str = "https://archive.org/wayback/available";
const WAYBACK_SAVE_BASE: &str = "https://web.archive.org/save";

/// Check if a URL is live by sending a HEAD request.
/// Also checks Internet Archive availability.
pub async fn verify_url(client: &reqwest::Client, url_str: &str) -> VerifyResult<UrlVerifyResult> {
    verify_url_with_wayback_base(client, url_str, WAYBACK_API_BASE).await
}

/// Same as `verify_url` but with a configurable Wayback API base URL (for testing).
pub async fn verify_url_with_wayback_base(
    client: &reqwest::Client,
    url_str: &str,
    wayback_base: &str,
) -> VerifyResult<UrlVerifyResult> {
    // Validate URL
    Url::parse(url_str).map_err(|e| VerifyError::InvalidUrl(e.to_string()))?;

    // HEAD request to check liveness
    let (is_live, status_code) = check_url_liveness(client, url_str).await?;

    // Check Internet Archive
    let (archive_available, archive_url) =
        check_wayback_availability(client, url_str, wayback_base).await;

    Ok(UrlVerifyResult {
        url: url_str.to_string(),
        is_live,
        status_code,
        archive_available,
        archive_url,
    })
}

async fn check_url_liveness(
    client: &reqwest::Client,
    url: &str,
) -> VerifyResult<(bool, Option<u16>)> {
    let resp = match head_with_retry(client, url).await {
        Ok(r) => r,
        Err(e) => {
            // Try GET if HEAD fails (some servers don't support HEAD)
            if e.is_connect() || e.is_timeout() {
                return Err(VerifyError::Http(e));
            }
            // HEAD not supported — try GET
            match client.get(url).send().await {
                Ok(r) => r,
                Err(e2) => return Err(VerifyError::Http(e2)),
            }
        }
    };

    let status = resp.status().as_u16();
    let is_live = resp.status().is_success();
    Ok((is_live, Some(status)))
}

/// Send a HEAD request with up to 3 retries and exponential backoff for
/// transient HTTP error status codes (429, 500, 502, 503, 504).
async fn head_with_retry(
    client: &reqwest::Client,
    url: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut resp = client.head(url).send().await?;

    for &delay_ms in &RETRY_BACKOFF_MS {
        if !TRANSIENT_STATUS_CODES.contains(&resp.status().as_u16()) {
            return Ok(resp);
        }
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        resp = client.head(url).send().await?;
    }

    Ok(resp)
}

async fn check_wayback_availability(
    client: &reqwest::Client,
    url: &str,
    wayback_base: &str,
) -> (bool, Option<String>) {
    let check_url = format!("{}?url={}", wayback_base, url);

    let resp = match client.get(&check_url).send().await {
        Ok(r) => r,
        Err(_) => return (false, None),
    };

    if !resp.status().is_success() {
        return (false, None);
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return (false, None),
    };

    // Wayback API response: { "archived_snapshots": { "closest": { "available": true, "url": "..." } } }
    let snapshot = body
        .get("archived_snapshots")
        .and_then(|s| s.get("closest"));

    match snapshot {
        Some(snap) => {
            let available = snap
                .get("available")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let archive_url = snap
                .get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (available, archive_url)
        }
        None => (false, None),
    }
}

/// Submit a URL to the Wayback Machine save API to trigger archival.
///
/// Uses `GET {save_base}/{url}` which returns a redirect to the saved snapshot
/// on success (HTTP 302/200) or indicates queued archival (HTTP 200 with job_id).
pub async fn archive_url_wayback(
    client: &reqwest::Client,
    url_str: &str,
) -> VerifyResult<ArchiveSaveResult> {
    archive_url_wayback_with_base(client, url_str, WAYBACK_SAVE_BASE).await
}

/// Same as `archive_url_wayback` but with a configurable save API base URL (for testing).
pub async fn archive_url_wayback_with_base(
    client: &reqwest::Client,
    url_str: &str,
    save_base: &str,
) -> VerifyResult<ArchiveSaveResult> {
    Url::parse(url_str).map_err(|e| VerifyError::InvalidUrl(e.to_string()))?;

    let save_url = format!("{}/{}", save_base, url_str);

    // The Wayback Machine save endpoint is triggered by a simple GET.
    // It responds with 200 (queued), 302 (already archived, redirect), or error.
    let resp = client
        .get(&save_url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(VerifyError::Http)?;

    let status = resp.status().as_u16();
    let accepted = status == 200 || status == 302 || status == 301;

    // Extract snapshot URL from Content-Location or Location header
    let snapshot_url = resp
        .headers()
        .get("Content-Location")
        .or_else(|| resp.headers().get("Location"))
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            if s.starts_with("http") {
                s.to_string()
            } else {
                format!("https://web.archive.org{}", s)
            }
        });

    // Try to extract job_id from JSON body (async save responses)
    let job_id = if accepted {
        if let Ok(body) = resp.json::<serde_json::Value>().await {
            body.get("job_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    } else {
        None
    };

    Ok(ArchiveSaveResult {
        original_url: url_str.to_string(),
        snapshot_url,
        accepted,
        job_id,
    })
}

/// Check the freshness of a URL by inspecting HTTP response headers.
///
/// Returns timing, `Last-Modified`, `ETag`, `Content-Type`, and redirect info.
pub async fn check_freshness(
    client: &reqwest::Client,
    url_str: &str,
) -> VerifyResult<FreshnessResult> {
    Url::parse(url_str).map_err(|e| VerifyError::InvalidUrl(e.to_string()))?;

    let start = Instant::now();

    let resp = match client.head(url_str).send().await {
        Ok(r) => r,
        Err(e) => {
            if e.is_connect() || e.is_timeout() {
                return Ok(FreshnessResult {
                    url: url_str.to_string(),
                    is_accessible: false,
                    last_modified: None,
                    etag: None,
                    content_type: None,
                    response_time_ms: start.elapsed().as_millis() as u64,
                    redirected_to: None,
                });
            }
            // HEAD not supported — try GET with range to minimise bandwidth
            match client
                .get(url_str)
                .header("Range", "bytes=0-0")
                .send()
                .await
            {
                Ok(r) => r,
                Err(e2) => return Err(VerifyError::Http(e2)),
            }
        }
    };

    let elapsed = start.elapsed().as_millis() as u64;
    let is_accessible = resp.status().is_success() || resp.status().as_u16() == 304;

    let header_str = |name: &str| {
        resp.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    };

    let last_modified = header_str("Last-Modified");
    let etag = header_str("ETag");
    let content_type = header_str("Content-Type");

    // Detect redirect: if the final URL differs from the input
    let redirected_to = {
        let final_url = resp.url().as_str().to_string();
        if final_url != url_str {
            Some(final_url)
        } else {
            None
        }
    };

    Ok(FreshnessResult {
        url: url_str.to_string(),
        is_accessible,
        last_modified,
        etag,
        content_type,
        response_time_ms: elapsed,
        redirected_to,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_url_live_200() {
        let server = MockServer::start().await;

        // Mock the URL liveness check
        Mock::given(method("HEAD"))
            .and(path("/some-page"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        // Mock wayback
        let wayback_server = MockServer::start().await;
        let archive_resp = serde_json::json!({
            "archived_snapshots": {
                "closest": {
                    "available": true,
                    "url": "https://web.archive.org/web/20240101/http://example.com",
                    "timestamp": "20240101000000",
                    "status": "200"
                }
            }
        });
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(archive_resp))
            .mount(&wayback_server)
            .await;

        let client = reqwest::Client::new();
        let target_url = format!("{}/some-page", server.uri());
        let result = verify_url_with_wayback_base(&client, &target_url, &wayback_server.uri())
            .await
            .unwrap();

        assert!(result.is_live);
        assert_eq!(result.status_code, Some(200));
        assert!(result.archive_available);
        assert!(result.archive_url.is_some());
    }

    #[tokio::test]
    async fn test_url_dead_404() {
        let server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(path("/dead-page"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let wayback_server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "archived_snapshots": {}
            })))
            .mount(&wayback_server)
            .await;

        let client = reqwest::Client::new();
        let target_url = format!("{}/dead-page", server.uri());
        let result = verify_url_with_wayback_base(&client, &target_url, &wayback_server.uri())
            .await
            .unwrap();

        assert!(!result.is_live);
        assert_eq!(result.status_code, Some(404));
        assert!(!result.archive_available);
    }

    #[tokio::test]
    async fn test_invalid_url() {
        let client = reqwest::Client::new();
        let result = verify_url_with_wayback_base(&client, "not-a-url", "http://wayback").await;
        assert!(matches!(result, Err(VerifyError::InvalidUrl(_))));
    }

    #[tokio::test]
    async fn test_url_no_archive() {
        let server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let wayback_server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "archived_snapshots": {
                    "closest": {
                        "available": false
                    }
                }
            })))
            .mount(&wayback_server)
            .await;

        let client = reqwest::Client::new();
        let target_url = server.uri();
        let result = verify_url_with_wayback_base(&client, &target_url, &wayback_server.uri())
            .await
            .unwrap();

        assert!(result.is_live);
        assert!(!result.archive_available);
    }

    #[tokio::test]
    async fn test_archive_url_wayback_accepted() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "job_id": "spn2-abc123",
                "status": "pending",
                "url": "https://example.com"
            })))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let result = archive_url_wayback_with_base(&client, "https://example.com", &server.uri())
            .await
            .unwrap();

        assert!(result.accepted);
        assert_eq!(result.original_url, "https://example.com");
        assert_eq!(result.job_id.as_deref(), Some("spn2-abc123"));
    }

    #[tokio::test]
    async fn test_archive_url_wayback_invalid_url() {
        let client = reqwest::Client::new();
        let result =
            archive_url_wayback_with_base(&client, "not-a-url", "http://save.example").await;
        assert!(matches!(result, Err(VerifyError::InvalidUrl(_))));
    }

    #[tokio::test]
    async fn test_check_freshness_live_with_headers() {
        let server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("Last-Modified", "Sat, 01 Jan 2024 00:00:00 GMT")
                    .append_header("ETag", "\"abc123\"")
                    .append_header("Content-Type", "text/html; charset=utf-8"),
            )
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let result = check_freshness(&client, &server.uri()).await.unwrap();

        assert!(result.is_accessible);
        assert_eq!(
            result.last_modified.as_deref(),
            Some("Sat, 01 Jan 2024 00:00:00 GMT")
        );
        assert_eq!(result.etag.as_deref(), Some("\"abc123\""));
        assert!(
            result
                .content_type
                .as_deref()
                .unwrap()
                .contains("text/html")
        );
        assert!(result.response_time_ms < 5000);
    }

    #[tokio::test]
    async fn test_check_freshness_no_headers() {
        let server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let result = check_freshness(&client, &server.uri()).await.unwrap();

        assert!(result.is_accessible);
        assert!(result.last_modified.is_none());
        assert!(result.etag.is_none());
    }

    #[tokio::test]
    async fn test_check_freshness_dead_url() {
        let server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let result = check_freshness(&client, &server.uri()).await.unwrap();

        assert!(!result.is_accessible);
    }

    #[tokio::test]
    async fn test_check_freshness_invalid_url() {
        let client = reqwest::Client::new();
        let result = check_freshness(&client, "not-a-url").await;
        assert!(matches!(result, Err(VerifyError::InvalidUrl(_))));
    }

    // --- Retry logic tests ---

    #[tokio::test]
    async fn test_retry_on_503_then_200() {
        let server = MockServer::start().await;

        // First request returns 503, second returns 200
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        // Also mock wayback for the full verify_url call
        let wayback_server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "archived_snapshots": {}
            })))
            .mount(&wayback_server)
            .await;

        let client = reqwest::Client::new();
        let result =
            verify_url_with_wayback_base(&client, &server.uri(), &wayback_server.uri())
                .await
                .unwrap();

        // After retry the URL should appear live
        assert!(result.is_live);
        assert_eq!(result.status_code, Some(200));
    }

    #[tokio::test]
    async fn test_retry_on_429_then_200() {
        let server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(429))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let wayback_server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "archived_snapshots": {}
            })))
            .mount(&wayback_server)
            .await;

        let client = reqwest::Client::new();
        let result =
            verify_url_with_wayback_base(&client, &server.uri(), &wayback_server.uri())
                .await
                .unwrap();

        assert!(result.is_live);
        assert_eq!(result.status_code, Some(200));
    }

    #[tokio::test]
    async fn test_no_retry_on_404() {
        // 404 is not transient; should NOT retry and should return immediately
        let server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1) // must be called exactly once (no retry)
            .mount(&server)
            .await;

        let wayback_server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "archived_snapshots": {}
            })))
            .mount(&wayback_server)
            .await;

        let client = reqwest::Client::new();
        let result =
            verify_url_with_wayback_base(&client, &server.uri(), &wayback_server.uri())
                .await
                .unwrap();

        assert!(!result.is_live);
        assert_eq!(result.status_code, Some(404));
    }

    #[tokio::test]
    async fn test_all_retries_exhausted_returns_last_status() {
        let server = MockServer::start().await;

        // All 4 attempts (initial + 3 retries) return 500
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let wayback_server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "archived_snapshots": {}
            })))
            .mount(&wayback_server)
            .await;

        let client = reqwest::Client::new();
        let result =
            verify_url_with_wayback_base(&client, &server.uri(), &wayback_server.uri())
                .await
                .unwrap();

        // After all retries, final 500 → not live
        assert!(!result.is_live);
        assert_eq!(result.status_code, Some(500));
    }

    #[tokio::test]
    async fn test_transient_codes_are_retried() {
        // Verify that each defined transient status code triggers a retry
        for &code in TRANSIENT_STATUS_CODES {
            let server = MockServer::start().await;

            Mock::given(method("HEAD"))
                .respond_with(ResponseTemplate::new(code))
                .up_to_n_times(1)
                .mount(&server)
                .await;
            Mock::given(method("HEAD"))
                .respond_with(ResponseTemplate::new(200))
                .mount(&server)
                .await;

            let wayback_server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "archived_snapshots": {}
                })))
                .mount(&wayback_server)
                .await;

            let client = reqwest::Client::new();
            let result =
                verify_url_with_wayback_base(&client, &server.uri(), &wayback_server.uri())
                    .await
                    .unwrap();

            assert!(
                result.is_live,
                "expected live after retry from status {code}"
            );
        }
    }
}
