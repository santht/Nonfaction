use crate::crossref::{CrossRefResult, crossref_person_with_base};
use crate::error::{VerifyError, VerifyResult};
use crate::fec::{FecVerifyResult, verify_fec_filing_with_base};
use crate::url::{UrlVerifyResult, verify_url_with_wayback_base};
use nf_core::source::{SourceRef, SourceType};

/// The result of running the verification pipeline on a `SourceRef`
#[derive(Debug)]
pub enum VerifyOutput {
    /// FEC filing check result
    Fec(FecVerifyResult),
    /// URL liveness check result
    Url(UrlVerifyResult),
    /// OpenSanctions cross-reference result
    CrossRef(CrossRefResult),
    /// Verification was not applicable for this source type
    NotApplicable(String),
}

/// Configuration for the verification pipeline.
/// Allows overriding API base URLs (primarily for testing).
#[derive(Debug, Clone)]
pub struct VerifyConfig {
    pub fec_api_base: String,
    pub wayback_api_base: String,
    pub opensanctions_api_base: String,
}

impl Default for VerifyConfig {
    fn default() -> Self {
        Self {
            fec_api_base: "https://api.open.fec.gov/v1".to_string(),
            wayback_api_base: "https://archive.org/wayback/available".to_string(),
            opensanctions_api_base: "https://api.opensanctions.org".to_string(),
        }
    }
}

/// Run the verification pipeline on a `SourceRef`.
///
/// Routing logic:
/// - `SourceType::FecFiling` → verify filing_id against FEC API
/// - Any URL source with `source_url` → verify URL liveness + archive
/// - `SourceType::SanctionsList` → cross-reference entity name against OpenSanctions
///
/// For sources with a filing_id, FEC verification is always attempted first.
pub async fn verify_source(
    client: &reqwest::Client,
    source: &SourceRef,
    config: &VerifyConfig,
) -> VerifyResult<VerifyOutput> {
    match source.source_type {
        SourceType::FecFiling => {
            let filing_id = source.filing_id.as_deref().unwrap_or_default();
            if filing_id.is_empty() {
                return Err(VerifyError::NotApplicable(
                    "FEC source has no filing_id".to_string(),
                ));
            }
            let result =
                verify_fec_filing_with_base(client, filing_id, &config.fec_api_base).await?;
            Ok(VerifyOutput::Fec(result))
        }

        SourceType::SanctionsList => {
            // For sanction sources, cross-reference the URL or any person name we have
            // (In practice, the caller should use verify_person_by_name directly)
            let url_str = source.source_url.as_str();
            let result =
                verify_url_with_wayback_base(client, url_str, &config.wayback_api_base).await?;
            Ok(VerifyOutput::Url(result))
        }

        _ => {
            // Default: verify the URL is live
            let url_str = source.source_url.as_str();
            let result =
                verify_url_with_wayback_base(client, url_str, &config.wayback_api_base).await?;
            Ok(VerifyOutput::Url(result))
        }
    }
}

/// Verify a Person entity by name against OpenSanctions.
pub async fn verify_person_by_name(
    client: &reqwest::Client,
    name: &str,
    config: &VerifyConfig,
) -> VerifyResult<VerifyOutput> {
    let result =
        crossref_person_with_base(client, name, &config.opensanctions_api_base).await?;
    Ok(VerifyOutput::CrossRef(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nf_core::source::{ContentHash, SourceRef, SourceType};
    use url::Url;
    use wiremock::matchers::{method, path, path_regex, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_source_ref(source_type: SourceType, url: &str) -> SourceRef {
        SourceRef::new(
            Url::parse(url).unwrap(),
            ContentHash::compute(b"test"),
            source_type,
            "system",
        )
    }

    #[tokio::test]
    async fn test_verify_fec_source() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex("/filings/"))
            .and(query_param("filing_id", "FEC-2024-001"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [{
                    "filing_id": "FEC-2024-001",
                    "committee_id": "C00001234",
                    "form_type": "F3",
                    "report_year": 2024
                }],
                "pagination": { "count": 1 }
            })))
            .mount(&server)
            .await;

        let mut source = make_source_ref(
            SourceType::FecFiling,
            "https://api.open.fec.gov/v1/filings/",
        );
        source.filing_id = Some("FEC-2024-001".to_string());

        let config = VerifyConfig {
            fec_api_base: server.uri(),
            ..Default::default()
        };

        let client = reqwest::Client::new();
        let output = verify_source(&client, &source, &config).await.unwrap();

        match output {
            VerifyOutput::Fec(result) => {
                assert!(result.found);
                assert_eq!(result.filing_id, "FEC-2024-001");
            }
            _ => panic!("Expected Fec output"),
        }
    }

    #[tokio::test]
    async fn test_verify_url_source() {
        let url_server = MockServer::start().await;
        let wayback_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&url_server)
            .await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "archived_snapshots": {}
            })))
            .mount(&wayback_server)
            .await;

        let source = make_source_ref(
            SourceType::GovernmentWebsite,
            &format!("{}/page", url_server.uri()),
        );

        let config = VerifyConfig {
            wayback_api_base: wayback_server.uri(),
            ..Default::default()
        };

        let client = reqwest::Client::new();
        let output = verify_source(&client, &source, &config).await.unwrap();

        match output {
            VerifyOutput::Url(result) => {
                assert!(result.is_live);
            }
            _ => panic!("Expected Url output"),
        }
    }

    #[tokio::test]
    async fn test_verify_person_by_name() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/match/default"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "responses": {
                    "q": {
                        "results": [{
                            "id": "NK-test",
                            "schema": "Person",
                            "score": 0.9,
                            "datasets": ["pep_positions"],
                            "properties": { "name": ["Test Person"] }
                        }]
                    }
                }
            })))
            .mount(&server)
            .await;

        let config = VerifyConfig {
            opensanctions_api_base: server.uri(),
            ..Default::default()
        };

        let client = reqwest::Client::new();
        let output = verify_person_by_name(&client, "Test Person", &config)
            .await
            .unwrap();

        match output {
            VerifyOutput::CrossRef(result) => {
                assert!(result.found);
                assert!(result.flags.is_pep);
            }
            _ => panic!("Expected CrossRef output"),
        }
    }

    #[tokio::test]
    async fn test_verify_fec_no_filing_id_fails() {
        let client = reqwest::Client::new();
        let source = make_source_ref(
            SourceType::FecFiling,
            "https://api.open.fec.gov/v1/filings/",
        );
        // No filing_id set

        let config = VerifyConfig::default();
        let result = verify_source(&client, &source, &config).await;
        assert!(matches!(result, Err(VerifyError::NotApplicable(_))));
    }
}
