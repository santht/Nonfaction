use crate::error::{VerifyError, VerifyResult};
use url::Url;

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

const WAYBACK_API_BASE: &str = "https://archive.org/wayback/available";

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
    let resp = match client.head(url).send().await {
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
        let result = verify_url_with_wayback_base(&client, "not-a-url", "http://wayback")
            .await;
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
        let result =
            verify_url_with_wayback_base(&client, &target_url, &wayback_server.uri())
                .await
                .unwrap();

        assert!(result.is_live);
        assert!(!result.archive_available);
    }
}
