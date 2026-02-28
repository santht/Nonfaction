use crate::error::{VerifyError, VerifyResult};
use serde::{Deserialize, Serialize};

/// FEC filing metadata returned by the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FecFilingMetadata {
    pub filing_id: String,
    pub committee_id: Option<String>,
    pub committee_name: Option<String>,
    pub report_type: Option<String>,
    pub report_year: Option<i32>,
    pub form_type: Option<String>,
    pub is_amended: Option<bool>,
    pub receipt_date: Option<String>,
}

/// Result of verifying a FEC filing ID
#[derive(Debug, Clone)]
pub struct FecVerifyResult {
    pub filing_id: String,
    /// Whether the filing was found in the FEC API
    pub found: bool,
    /// Metadata if found
    pub metadata: Option<FecFilingMetadata>,
}

const FEC_API_BASE: &str = "https://api.open.fec.gov/v1";
const FEC_API_KEY: &str = "DEMO_KEY";

/// Verify a FEC filing ID against the FEC API.
///
/// Returns `FecVerifyResult { found: false }` if the filing is not found (404).
/// Returns `Err` for network or API errors.
pub async fn verify_fec_filing(
    client: &reqwest::Client,
    filing_id: &str,
) -> VerifyResult<FecVerifyResult> {
    verify_fec_filing_with_base(client, filing_id, FEC_API_BASE).await
}

/// Same as `verify_fec_filing` but with a configurable base URL (for testing).
pub async fn verify_fec_filing_with_base(
    client: &reqwest::Client,
    filing_id: &str,
    base_url: &str,
) -> VerifyResult<FecVerifyResult> {
    let url = format!(
        "{}/filings/?filing_id={}&api_key={}",
        base_url, filing_id, FEC_API_KEY
    );

    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(VerifyError::Http)?;

    let status = resp.status().as_u16();

    if status == 404 {
        return Ok(FecVerifyResult {
            filing_id: filing_id.to_string(),
            found: false,
            metadata: None,
        });
    }

    if !resp.status().is_success() {
        return Err(VerifyError::FecApiError(format!(
            "FEC API returned HTTP {status}"
        )));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| VerifyError::JsonParse(e.to_string()))?;

    // FEC API response shape: { "results": [...], "pagination": {...} }
    let results = body.get("results").and_then(|r| r.as_array());

    let (found, metadata) = match results {
        Some(arr) if !arr.is_empty() => {
            let first = &arr[0];
            let meta = FecFilingMetadata {
                filing_id: filing_id.to_string(),
                committee_id: first
                    .get("committee_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                committee_name: first
                    .get("committee")
                    .and_then(|c| c.get("name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                report_type: first
                    .get("report_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                report_year: first
                    .get("report_year")
                    .and_then(|v| v.as_i64())
                    .map(|y| y as i32),
                form_type: first
                    .get("form_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                is_amended: first
                    .get("is_amended")
                    .and_then(|v| v.as_bool()),
                receipt_date: first
                    .get("receipt_date")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            };
            (true, Some(meta))
        }
        _ => (false, None),
    };

    Ok(FecVerifyResult {
        filing_id: filing_id.to_string(),
        found,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn make_client() -> reqwest::Client {
        reqwest::Client::new()
    }

    #[tokio::test]
    async fn test_fec_filing_found() {
        let server = MockServer::start().await;

        let response_body = serde_json::json!({
            "results": [{
                "filing_id": 12345,
                "committee_id": "C00001234",
                "report_type": "Q1",
                "report_year": 2024,
                "form_type": "F3",
                "is_amended": false,
                "receipt_date": "2024-04-15"
            }],
            "pagination": { "count": 1 }
        });

        Mock::given(method("GET"))
            .and(path_regex("/filings/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&server)
            .await;

        let client = make_client().await;

        let result = verify_fec_filing_with_base(&client, "12345", &server.uri()).await.unwrap();
        assert!(result.found);
        assert_eq!(result.filing_id, "12345");
        let meta = result.metadata.unwrap();
        assert_eq!(meta.committee_id.as_deref(), Some("C00001234"));
        assert_eq!(meta.report_year, Some(2024));
    }

    #[tokio::test]
    async fn test_fec_filing_not_found() {
        let server = MockServer::start().await;

        let response_body = serde_json::json!({
            "results": [],
            "pagination": { "count": 0 }
        });

        Mock::given(method("GET"))
            .and(path_regex("/filings/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&server)
            .await;

        let client = make_client().await;
        let result = verify_fec_filing_with_base(&client, "99999", &server.uri())
            .await
            .unwrap();
        assert!(!result.found);
        assert!(result.metadata.is_none());
    }

    #[tokio::test]
    async fn test_fec_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex("/filings/"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = make_client().await;
        let result = verify_fec_filing_with_base(&client, "12345", &server.uri()).await;
        assert!(matches!(result, Err(VerifyError::FecApiError(_))));
    }
}
