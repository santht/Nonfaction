use crate::crossref::CrossRefResult;
use crate::fec::FecVerifyResult;
use crate::pipeline::VerifyOutput;
use crate::url::UrlVerifyResult;
use std::time::{SystemTime, UNIX_EPOCH};

/// Raw evidence backing a verification result, with a human-readable summary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Evidence {
    /// Raw JSON data captured from the verification source
    pub raw: serde_json::Value,
    /// Human-readable summary of the finding
    pub summary: String,
    /// Which system produced this evidence (e.g. "fec-api", "wayback-api")
    pub source: String,
}

/// A structured, evidence-backed result from running the verification pipeline.
///
/// Wraps a `VerifyOutput` with provenance metadata: when it was checked,
/// how confident we are in the result, and the raw evidence JSON.
#[derive(Debug)]
pub struct VerificationResult {
    /// The underlying verification output
    pub output: VerifyOutput,
    /// Unix timestamp (seconds since epoch) when verification was performed
    pub verified_at: u64,
    /// Confidence in the result: 0.0 = unknown/not found, 1.0 = confirmed
    pub confidence: f64,
    /// Structured evidence backing this result
    pub evidence: Evidence,
}

impl VerificationResult {
    /// Build a `VerificationResult` from a `VerifyOutput`, computing
    /// confidence and evidence automatically.
    pub fn from_output(output: VerifyOutput) -> Self {
        let verified_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let (confidence, evidence) = compute_evidence(&output);

        Self {
            output,
            verified_at,
            confidence,
            evidence,
        }
    }

    /// Returns `true` if the result is considered verified (confidence ≥ 0.5).
    pub fn is_verified(&self) -> bool {
        self.confidence >= 0.5
    }
}

fn compute_evidence(output: &VerifyOutput) -> (f64, Evidence) {
    match output {
        VerifyOutput::Fec(result) => evidence_from_fec(result),
        VerifyOutput::Url(result) => evidence_from_url(result),
        VerifyOutput::CrossRef(result) => evidence_from_crossref(result),
        VerifyOutput::NotApplicable(reason) => {
            let ev = Evidence {
                raw: serde_json::Value::Null,
                summary: format!("Not applicable: {reason}"),
                source: "pipeline".to_string(),
            };
            (0.0, ev)
        }
    }
}

fn evidence_from_fec(r: &FecVerifyResult) -> (f64, Evidence) {
    let confidence = if r.found { 1.0 } else { 0.0 };
    let summary = if r.found {
        let committee = r
            .metadata
            .as_ref()
            .and_then(|m| m.committee_id.as_deref())
            .unwrap_or("unknown");
        format!(
            "FEC filing {} confirmed (committee: {committee})",
            r.filing_id
        )
    } else {
        format!("FEC filing {} not found in FEC database", r.filing_id)
    };
    let raw = serde_json::json!({
        "filing_id": r.filing_id,
        "found": r.found,
        "metadata": r.metadata,
    });
    (
        confidence,
        Evidence {
            raw,
            summary,
            source: "fec-api".to_string(),
        },
    )
}

fn evidence_from_url(r: &UrlVerifyResult) -> (f64, Evidence) {
    let confidence = match (r.is_live, r.archive_available) {
        (true, true) => 1.0,
        (true, false) => 0.85,
        (false, true) => 0.5,
        (false, false) => 0.0,
    };
    let status = r.status_code.unwrap_or(0);
    let summary = if r.is_live {
        format!(
            "URL is live (HTTP {status}); archive available: {}",
            r.archive_available
        )
    } else {
        let archive_note = if r.archive_available {
            "archive available"
        } else {
            "no archive"
        };
        format!("URL is dead (HTTP {status}); {archive_note}")
    };
    let raw = serde_json::json!({
        "url": r.url,
        "is_live": r.is_live,
        "status_code": r.status_code,
        "archive_available": r.archive_available,
        "archive_url": r.archive_url,
    });
    (
        confidence,
        Evidence {
            raw,
            summary,
            source: "wayback-api".to_string(),
        },
    )
}

fn evidence_from_crossref(r: &CrossRefResult) -> (f64, Evidence) {
    let confidence = if r.found {
        r.flags.match_score.unwrap_or(0.5).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let summary = if r.found {
        format!(
            "Entity '{}' matched in OpenSanctions (score: {:.2}; PEP: {}; sanctioned: {})",
            r.query_name,
            r.flags.match_score.unwrap_or(0.0),
            r.flags.is_pep,
            r.flags.is_sanctioned
        )
    } else {
        format!("Entity '{}' not found in OpenSanctions", r.query_name)
    };
    let raw = serde_json::json!({
        "query_name": r.query_name,
        "found": r.found,
        "flags": {
            "is_pep": r.flags.is_pep,
            "is_sanctioned": r.flags.is_sanctioned,
            "datasets": r.flags.datasets,
            "entity_id": r.flags.entity_id,
            "matched_name": r.flags.matched_name,
            "match_score": r.flags.match_score,
        },
    });
    (
        confidence,
        Evidence {
            raw,
            summary,
            source: "opensanctions-api".to_string(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fec::FecVerifyResult;
    use crate::url::UrlVerifyResult;

    #[test]
    fn test_verification_result_from_fec_found() {
        let fec = FecVerifyResult {
            filing_id: "FEC-001".to_string(),
            found: true,
            metadata: Some(crate::fec::FecFilingMetadata {
                filing_id: "FEC-001".to_string(),
                committee_id: Some("C00001234".to_string()),
                committee_name: None,
                report_type: Some("Q1".to_string()),
                report_year: Some(2024),
                form_type: Some("F3".to_string()),
                is_amended: Some(false),
                receipt_date: None,
            }),
        };
        let result = VerificationResult::from_output(VerifyOutput::Fec(fec));
        assert_eq!(result.confidence, 1.0);
        assert!(result.is_verified());
        assert!(result.evidence.summary.contains("FEC-001"));
        assert_eq!(result.evidence.source, "fec-api");
    }

    #[test]
    fn test_verification_result_from_fec_not_found() {
        let fec = FecVerifyResult {
            filing_id: "FEC-999".to_string(),
            found: false,
            metadata: None,
        };
        let result = VerificationResult::from_output(VerifyOutput::Fec(fec));
        assert_eq!(result.confidence, 0.0);
        assert!(!result.is_verified());
    }

    #[test]
    fn test_verification_result_from_url_live_with_archive() {
        let url = UrlVerifyResult {
            url: "https://example.com".to_string(),
            is_live: true,
            status_code: Some(200),
            archive_available: true,
            archive_url: Some(
                "https://web.archive.org/web/20240101/https://example.com".to_string(),
            ),
        };
        let result = VerificationResult::from_output(VerifyOutput::Url(url));
        assert_eq!(result.confidence, 1.0);
        assert!(result.is_verified());
        assert_eq!(result.evidence.source, "wayback-api");
    }

    #[test]
    fn test_verification_result_from_url_dead_with_archive() {
        let url = UrlVerifyResult {
            url: "https://dead.example.com".to_string(),
            is_live: false,
            status_code: Some(404),
            archive_available: true,
            archive_url: Some("https://web.archive.org/old".to_string()),
        };
        let result = VerificationResult::from_output(VerifyOutput::Url(url));
        assert_eq!(result.confidence, 0.5);
        assert!(result.is_verified());
    }

    #[test]
    fn test_verification_result_from_url_dead_no_archive() {
        let url = UrlVerifyResult {
            url: "https://gone.example.com".to_string(),
            is_live: false,
            status_code: Some(404),
            archive_available: false,
            archive_url: None,
        };
        let result = VerificationResult::from_output(VerifyOutput::Url(url));
        assert_eq!(result.confidence, 0.0);
        assert!(!result.is_verified());
    }

    #[test]
    fn test_verification_result_not_applicable() {
        let result =
            VerificationResult::from_output(VerifyOutput::NotApplicable("no source".to_string()));
        assert_eq!(result.confidence, 0.0);
        assert!(!result.is_verified());
        assert!(result.evidence.summary.contains("Not applicable"));
    }

    #[test]
    fn test_verified_at_is_recent() {
        let result = VerificationResult::from_output(VerifyOutput::NotApplicable("x".to_string()));
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(result.verified_at <= now);
        assert!(result.verified_at > now - 5);
    }

    #[test]
    fn test_evidence_raw_has_url_fields() {
        let url = UrlVerifyResult {
            url: "https://example.com".to_string(),
            is_live: true,
            status_code: Some(200),
            archive_available: false,
            archive_url: None,
        };
        let result = VerificationResult::from_output(VerifyOutput::Url(url));
        assert_eq!(result.evidence.raw["url"], "https://example.com");
        assert_eq!(result.evidence.raw["is_live"], true);
        assert_eq!(result.evidence.raw["status_code"], 200);
    }
}
