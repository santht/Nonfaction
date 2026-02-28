use crate::crossref::{CrossRefResult, crossref_person_with_base};
use crate::error::{VerifyError, VerifyResult};
use crate::fec::{FecVerifyResult, verify_fec_filing_with_base};
use crate::result::VerificationResult;
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

/// Statistics summarising a batch verification run.
#[derive(Debug, Clone)]
pub struct BatchStats {
    /// Number of sources that were successfully verified (confidence ≥ 0.5).
    pub verified: usize,
    /// Number of sources that failed verification or returned an error.
    pub failed: usize,
    /// Number of sources where verification was not applicable.
    pub not_applicable: usize,
    /// Average confidence across all successful (non-error) results.
    pub avg_confidence: f64,
}

impl BatchStats {
    /// Fraction of processed sources that were verified, excluding not-applicable ones.
    ///
    /// Returns `0.0` if no sources were processed.
    pub fn success_rate(&self) -> f64 {
        let total = self.verified + self.failed;
        if total == 0 {
            return 0.0;
        }
        self.verified as f64 / total as f64
    }
}

/// Compute aggregate statistics from a slice of batch verification results.
pub fn compute_batch_stats(results: &[VerifyResult<VerificationResult>]) -> BatchStats {
    let mut verified = 0usize;
    let mut failed = 0usize;
    let mut not_applicable = 0usize;
    let mut confidence_sum = 0.0f64;
    let mut confidence_count = 0usize;

    for result in results {
        match result {
            Ok(r) => {
                if matches!(&r.output, VerifyOutput::NotApplicable(_)) {
                    not_applicable += 1;
                } else if r.is_verified() {
                    verified += 1;
                } else {
                    failed += 1;
                }
                confidence_sum += r.confidence;
                confidence_count += 1;
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    let avg_confidence = if confidence_count > 0 {
        confidence_sum / confidence_count as f64
    } else {
        0.0
    };

    BatchStats {
        verified,
        failed,
        not_applicable,
        avg_confidence,
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
    let result = crossref_person_with_base(client, name, &config.opensanctions_api_base).await?;
    Ok(VerifyOutput::CrossRef(result))
}

/// Run the verification pipeline and wrap the result in a `VerificationResult`
/// with evidence and confidence.
pub async fn verify_source_with_result(
    client: &reqwest::Client,
    source: &SourceRef,
    config: &VerifyConfig,
) -> VerifyResult<VerificationResult> {
    let output = verify_source(client, source, config).await?;
    Ok(VerificationResult::from_output(output))
}

/// Verify multiple sources concurrently, returning one result per source.
///
/// Each source is verified independently; errors for individual sources are
/// captured as `Err` values rather than failing the entire batch.
pub async fn verify_batch(
    client: &reqwest::Client,
    sources: &[SourceRef],
    config: &VerifyConfig,
) -> Vec<VerifyResult<VerificationResult>> {
    let handles: Vec<_> = sources
        .iter()
        .map(|source| {
            let client = client.clone();
            let config = config.clone();
            let source = source.clone();
            tokio::spawn(async move {
                verify_source(&client, &source, &config)
                    .await
                    .map(VerificationResult::from_output)
            })
        })
        .collect();

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        match handle.await {
            Ok(result) => results.push(result),
            Err(e) => results.push(Err(VerifyError::NotApplicable(format!(
                "Task panicked: {e}"
            )))),
        }
    }
    results
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

    #[tokio::test]
    async fn test_verify_source_with_result_wraps_output() {
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
            &format!("{}/doc", url_server.uri()),
        );
        let config = VerifyConfig {
            wayback_api_base: wayback_server.uri(),
            ..Default::default()
        };

        let client = reqwest::Client::new();
        let result = verify_source_with_result(&client, &source, &config)
            .await
            .unwrap();

        assert!(result.is_verified()); // live URL → confidence 0.85
        assert!(result.confidence > 0.0);
        assert!(result.verified_at > 0);
        assert!(!result.evidence.summary.is_empty());
    }

    #[tokio::test]
    async fn test_verify_batch_multiple_sources() {
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

        let sources = vec![
            make_source_ref(SourceType::GovernmentWebsite, &url_server.uri()),
            make_source_ref(SourceType::GovernmentWebsite, &url_server.uri()),
        ];
        let config = VerifyConfig {
            wayback_api_base: wayback_server.uri(),
            ..Default::default()
        };

        let client = reqwest::Client::new();
        let results = verify_batch(&client, &sources, &config).await;

        assert_eq!(results.len(), 2);
        for r in &results {
            assert!(r.is_ok());
        }
    }

    #[tokio::test]
    async fn test_verify_batch_empty_sources() {
        let client = reqwest::Client::new();
        let config = VerifyConfig::default();
        let results = verify_batch(&client, &[], &config).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_verify_batch_fec_error_captured_per_item() {
        // FEC source with no filing_id → error captured, not propagated
        let source = make_source_ref(
            SourceType::FecFiling,
            "https://api.open.fec.gov/v1/filings/",
        );
        let client = reqwest::Client::new();
        let config = VerifyConfig::default();
        let results = verify_batch(&client, &[source], &config).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    // --- BatchStats / compute_batch_stats tests ---

    fn make_ok_verified() -> VerifyResult<VerificationResult> {
        use crate::url::UrlVerifyResult;
        let url = UrlVerifyResult {
            url: "https://example.com".to_string(),
            is_live: true,
            status_code: Some(200),
            archive_available: false,
            archive_url: None,
        };
        Ok(VerificationResult::from_output(VerifyOutput::Url(url)))
    }

    fn make_ok_failed() -> VerifyResult<VerificationResult> {
        use crate::url::UrlVerifyResult;
        let url = UrlVerifyResult {
            url: "https://dead.example.com".to_string(),
            is_live: false,
            status_code: Some(404),
            archive_available: false,
            archive_url: None,
        };
        Ok(VerificationResult::from_output(VerifyOutput::Url(url)))
    }

    fn make_ok_not_applicable() -> VerifyResult<VerificationResult> {
        Ok(VerificationResult::from_output(VerifyOutput::NotApplicable(
            "test".to_string(),
        )))
    }

    fn make_err() -> VerifyResult<VerificationResult> {
        Err(VerifyError::NotApplicable("error".to_string()))
    }

    #[test]
    fn test_batch_stats_empty() {
        let stats = compute_batch_stats(&[]);
        assert_eq!(stats.verified, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.not_applicable, 0);
        assert_eq!(stats.avg_confidence, 0.0);
        assert_eq!(stats.success_rate(), 0.0);
    }

    #[test]
    fn test_batch_stats_all_verified() {
        let results = vec![make_ok_verified(), make_ok_verified()];
        let stats = compute_batch_stats(&results);
        assert_eq!(stats.verified, 2);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.not_applicable, 0);
        assert_eq!(stats.success_rate(), 1.0);
        assert!(stats.avg_confidence > 0.0);
    }

    #[test]
    fn test_batch_stats_all_failed() {
        let results = vec![make_ok_failed(), make_ok_failed()];
        let stats = compute_batch_stats(&results);
        assert_eq!(stats.verified, 0);
        assert_eq!(stats.failed, 2);
        assert_eq!(stats.success_rate(), 0.0);
    }

    #[test]
    fn test_batch_stats_error_counts_as_failed() {
        let results = vec![make_ok_verified(), make_err()];
        let stats = compute_batch_stats(&results);
        assert_eq!(stats.verified, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.success_rate(), 0.5);
    }

    #[test]
    fn test_batch_stats_not_applicable_excluded_from_success_rate() {
        let results = vec![
            make_ok_verified(),
            make_ok_not_applicable(),
            make_ok_not_applicable(),
        ];
        let stats = compute_batch_stats(&results);
        assert_eq!(stats.verified, 1);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.not_applicable, 2);
        // success_rate only looks at verified + failed (denominator = 1)
        assert_eq!(stats.success_rate(), 1.0);
    }

    #[test]
    fn test_batch_stats_mixed() {
        let results = vec![
            make_ok_verified(),
            make_ok_failed(),
            make_ok_not_applicable(),
            make_err(),
        ];
        let stats = compute_batch_stats(&results);
        assert_eq!(stats.verified, 1);
        assert_eq!(stats.failed, 2); // one ok_failed + one err
        assert_eq!(stats.not_applicable, 1);
        // success_rate = 1 / (1 + 2) = 1/3
        assert!((stats.success_rate() - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_batch_stats_avg_confidence_computed_from_ok_results_only() {
        // make_ok_verified → confidence 0.85 (live, no archive)
        // make_ok_failed   → confidence 0.0
        // make_err         → not counted in avg_confidence
        let results = vec![make_ok_verified(), make_ok_failed(), make_err()];
        let stats = compute_batch_stats(&results);
        // avg = (0.85 + 0.0) / 2
        let expected = (0.85 + 0.0) / 2.0;
        assert!((stats.avg_confidence - expected).abs() < 1e-9);
    }

    #[test]
    fn test_success_rate_no_applicable_items() {
        // Only not_applicable and errors
        let results = vec![make_ok_not_applicable(), make_err()];
        let stats = compute_batch_stats(&results);
        assert_eq!(stats.not_applicable, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.verified, 0);
        // Denominator for success_rate = verified + failed = 0 + 1 = 1
        assert_eq!(stats.success_rate(), 0.0);
    }
}
