use crate::error::{VerifyError, VerifyResult};
use serde::{Deserialize, Serialize};

const OPENSANCTIONS_BASE: &str = "https://api.opensanctions.org";

/// Sanction/PEP status flags for a matched entity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SanctionFlags {
    /// Whether the entity is on a sanctions list
    pub is_sanctioned: bool,
    /// Whether the entity is a Politically Exposed Person (PEP)
    pub is_pep: bool,
    /// Which datasets the entity appears in
    pub datasets: Vec<String>,
    /// The OpenSanctions entity ID
    pub entity_id: Option<String>,
    /// The matched name in OpenSanctions
    pub matched_name: Option<String>,
    /// Match score (0.0 - 1.0)
    pub match_score: Option<f64>,
}

/// Result of cross-referencing a person against OpenSanctions
#[derive(Debug, Clone)]
pub struct CrossRefResult {
    pub query_name: String,
    /// Whether any match was found
    pub found: bool,
    pub flags: SanctionFlags,
}

/// Cross-reference a person by name against the OpenSanctions API.
pub async fn crossref_person(
    client: &reqwest::Client,
    name: &str,
) -> VerifyResult<CrossRefResult> {
    crossref_person_with_base(client, name, OPENSANCTIONS_BASE).await
}

/// Same as `crossref_person` but with a configurable base URL (for testing).
pub async fn crossref_person_with_base(
    client: &reqwest::Client,
    name: &str,
    base_url: &str,
) -> VerifyResult<CrossRefResult> {
    if name.trim().is_empty() {
        return Ok(CrossRefResult {
            query_name: name.to_string(),
            found: false,
            flags: SanctionFlags::default(),
        });
    }

    // Use the /match/ endpoint with a Person schema query
    // POST /match/default with { queries: { q: { schema: "Person", properties: { name: [...] } } } }
    let url = format!("{}/match/default", base_url);
    let payload = serde_json::json!({
        "queries": {
            "q": {
                "schema": "Person",
                "properties": {
                    "name": [name]
                }
            }
        }
    });

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(VerifyError::Http)?;

    if !resp.status().is_success() {
        return Err(VerifyError::OpenSanctionsError(format!(
            "OpenSanctions API returned HTTP {}",
            resp.status().as_u16()
        )));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| VerifyError::JsonParse(e.to_string()))?;

    // Response shape: { "responses": { "q": { "results": [...] } } }
    let results = body
        .get("responses")
        .and_then(|r| r.get("q"))
        .and_then(|q| q.get("results"))
        .and_then(|r| r.as_array());

    let Some(results) = results else {
        return Ok(CrossRefResult {
            query_name: name.to_string(),
            found: false,
            flags: SanctionFlags::default(),
        });
    };

    if results.is_empty() {
        return Ok(CrossRefResult {
            query_name: name.to_string(),
            found: false,
            flags: SanctionFlags::default(),
        });
    }

    // Take the best (first) result
    let best = &results[0];
    let flags = parse_flags(best);

    Ok(CrossRefResult {
        query_name: name.to_string(),
        found: true,
        flags,
    })
}

fn parse_flags(result: &serde_json::Value) -> SanctionFlags {
    let entity_id = result
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let match_score = result.get("score").and_then(|v| v.as_f64());

    // Matched name: first value in properties.name array
    let matched_name = result
        .get("properties")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Datasets
    let datasets: Vec<String> = result
        .get("datasets")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    // Determine PEP/sanction flags from dataset names and schema
    let is_sanctioned = datasets
        .iter()
        .any(|d| d.contains("sanction") || d.contains("ofac") || d.contains("eu_fsf"));
    let is_pep = datasets.iter().any(|d| d.contains("pep"))
        || result
            .get("schema")
            .and_then(|s| s.as_str())
            .map(|s| s == "Person")
            .unwrap_or(false)
            && result
                .get("properties")
                .and_then(|p| p.get("topics"))
                .and_then(|t| t.as_array())
                .map(|arr| {
                    arr.iter()
                        .any(|v| v.as_str().map(|s| s.contains("pep")).unwrap_or(false))
                })
                .unwrap_or(false);

    SanctionFlags {
        is_sanctioned,
        is_pep,
        datasets,
        entity_id,
        matched_name,
        match_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_crossref_person_found_pep() {
        let server = MockServer::start().await;

        let response_body = serde_json::json!({
            "responses": {
                "q": {
                    "results": [{
                        "id": "NK-abc123",
                        "schema": "Person",
                        "score": 0.95,
                        "datasets": ["pep_positions", "eu_meps"],
                        "properties": {
                            "name": ["John Doe"],
                            "topics": ["pep"]
                        }
                    }]
                }
            }
        });

        Mock::given(method("POST"))
            .and(path("/match/default"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let result = crossref_person_with_base(&client, "John Doe", &server.uri())
            .await
            .unwrap();

        assert!(result.found);
        assert_eq!(result.query_name, "John Doe");
        assert!(result.flags.is_pep);
        assert!(!result.flags.is_sanctioned);
        assert_eq!(result.flags.entity_id.as_deref(), Some("NK-abc123"));
        assert_eq!(result.flags.matched_name.as_deref(), Some("John Doe"));
        assert!((result.flags.match_score.unwrap() - 0.95).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_crossref_person_sanctioned() {
        let server = MockServer::start().await;

        let response_body = serde_json::json!({
            "responses": {
                "q": {
                    "results": [{
                        "id": "NK-sanc456",
                        "schema": "Person",
                        "score": 0.88,
                        "datasets": ["ofac_sdn", "un_sc_sanctions"],
                        "properties": {
                            "name": ["Bad Actor"]
                        }
                    }]
                }
            }
        });

        Mock::given(method("POST"))
            .and(path("/match/default"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let result = crossref_person_with_base(&client, "Bad Actor", &server.uri())
            .await
            .unwrap();

        assert!(result.found);
        assert!(result.flags.is_sanctioned);
    }

    #[tokio::test]
    async fn test_crossref_person_not_found() {
        let server = MockServer::start().await;

        let response_body = serde_json::json!({
            "responses": {
                "q": {
                    "results": []
                }
            }
        });

        Mock::given(method("POST"))
            .and(path("/match/default"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let result = crossref_person_with_base(&client, "Unknown Person", &server.uri())
            .await
            .unwrap();

        assert!(!result.found);
        assert!(!result.flags.is_pep);
        assert!(!result.flags.is_sanctioned);
    }

    #[tokio::test]
    async fn test_crossref_empty_name_returns_not_found() {
        let client = reqwest::Client::new();
        let result = crossref_person_with_base(&client, "  ", "http://unused")
            .await
            .unwrap();
        assert!(!result.found);
    }

    #[tokio::test]
    async fn test_crossref_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/match/default"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let result = crossref_person_with_base(&client, "Test Person", &server.uri()).await;
        assert!(matches!(result, Err(VerifyError::OpenSanctionsError(_))));
    }

    #[tokio::test]
    async fn test_crossref_person_no_datasets() {
        let server = MockServer::start().await;

        let response_body = serde_json::json!({
            "responses": {
                "q": {
                    "results": [{
                        "id": "NK-xyz",
                        "schema": "Person",
                        "score": 0.7,
                        "datasets": [],
                        "properties": {
                            "name": ["Jane Smith"]
                        }
                    }]
                }
            }
        });

        Mock::given(method("POST"))
            .and(path("/match/default"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let result = crossref_person_with_base(&client, "Jane Smith", &server.uri())
            .await
            .unwrap();

        assert!(result.found);
        assert!(!result.flags.is_sanctioned);
        assert!(!result.flags.is_pep);
    }
}
