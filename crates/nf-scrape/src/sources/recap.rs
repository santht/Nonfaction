//! CourtListener / RECAP API scraper.
//!
//! Endpoints used:
//! - GET /api/rest/v3/dockets/   → CourtCase entities
//! - GET /api/rest/v3/opinions/  → CourtCase entities (appellate opinions)

use std::future::Future;
use std::pin::Pin;

use chrono::NaiveDate;
use serde::Deserialize;
use tracing::debug;
use url::Url;

use nf_core::{
    CaseType, ContentHash, CourtCase, Entity, EntityMeta, SourceChain, SourceRef, SourceType,
};

use crate::config::RecapConfig;
use crate::framework::{ScrapeError, ScrapeResult, ScrapeSource, ScraperRuntime};

// ── API response shapes ───────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RecapListPage<T> {
    count: Option<u64>,
    next: Option<String>,
    results: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct RecapDocket {
    id: Option<u64>,
    docket_number: Option<String>,
    case_name: Option<String>,
    court: Option<String>,
    date_filed: Option<String>,
    date_terminated: Option<String>,
    nature_of_suit: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RecapOpinion {
    id: Option<u64>,
    case_name: Option<String>,
    court: Option<String>,
    date_filed: Option<String>,
    absolute_url: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn recap_source_ref(url: &Url, raw: &[u8]) -> SourceRef {
    SourceRef::new(
        url.clone(),
        ContentHash::compute(raw),
        SourceType::CourtRecord,
        "system",
    )
}

fn parse_date(s: Option<&str>) -> Option<NaiveDate> {
    let s = s?;
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

fn infer_case_type(nature_of_suit: Option<&str>, court: Option<&str>) -> CaseType {
    let court_str = court.unwrap_or("").to_lowercase();

    // Supreme Court
    if court_str == "scotus" || court_str.contains("supreme") {
        return CaseType::SupremeCourt;
    }

    // Bankruptcy courts
    if court_str.starts_with("bankr") || court_str.contains("bankr") {
        return CaseType::Bankruptcy;
    }

    // Circuit appellate courts in CourtListener use IDs like ca1–ca11, cadc, cafc.
    // District courts use cacd, cand, caed, casd — these are NOT appellate.
    let is_circuit = matches!(
        court_str.as_str(),
        "ca1" | "ca2" | "ca3" | "ca4" | "ca5" | "ca6" | "ca7" | "ca8" | "ca9"
            | "ca10" | "ca11" | "cadc" | "cafc"
    );
    if is_circuit {
        return CaseType::Appellate;
    }

    match nature_of_suit {
        Some(s) if s.contains("Criminal") => CaseType::Criminal,
        Some(s) if s.contains("Bankruptcy") => CaseType::Bankruptcy,
        _ => CaseType::Civil,
    }
}

// ── RECAP scraper ─────────────────────────────────────────────────────────────

pub struct RecapScraper {
    config: RecapConfig,
}

impl RecapScraper {
    pub fn new(config: RecapConfig) -> Self {
        Self { config }
    }

    fn base_url(&self) -> &str {
        &self.config.base_url
    }

    fn apply_auth(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(token) = &self.config.api_token {
            request.bearer_auth(token)
        } else {
            request
        }
    }

    /// Fetch JSON from CourtListener, applying auth if configured.
    async fn fetch_cl_json(
        &self,
        url: &Url,
        runtime: &ScraperRuntime,
    ) -> ScrapeResult<Option<serde_json::Value>> {
        let url_str = url.to_string();

        if !runtime.mark_seen(&url_str).await {
            return Ok(None);
        }

        runtime.wait_for_token().await;

        let mut builder = runtime.client.get(url.clone());
        builder = self.apply_auth(builder);

        let resp = builder.send().await.map_err(ScrapeError::Http)?;
        let json = resp.json::<serde_json::Value>().await.map_err(ScrapeError::Http)?;
        Ok(Some(json))
    }

    // ── Dockets ───────────────────────────────────────────────────────────────

    pub async fn scrape_dockets(
        &self,
        runtime: &ScraperRuntime,
    ) -> ScrapeResult<Vec<Entity>> {
        let mut entities = Vec::new();
        let mut page = 1u32;

        loop {
            if page > self.config.max_pages {
                break;
            }

            let url = Url::parse_with_params(
                &format!("{}/dockets/", self.base_url()),
                &[
                    ("format", "json"),
                    ("page_size", &self.config.per_page.to_string()),
                    ("page", &page.to_string()),
                    ("order_by", "-date_filed"),
                ],
            )
            .map_err(ScrapeError::UrlParse)?;

            let Some(json) = self.fetch_cl_json(&url, runtime).await? else {
                break;
            };

            let raw = serde_json::to_vec(&json).unwrap_or_default();
            let source_ref = recap_source_ref(&url, &raw);

            let page_data: RecapListPage<RecapDocket> =
                serde_json::from_value(json).map_err(ScrapeError::Json)?;

            if page_data.results.is_empty() {
                break;
            }

            let total = page_data.count.unwrap_or(0);

            for docket in page_data.results {
                let case_id = docket
                    .id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| docket.docket_number.clone().unwrap_or_default());

                let detail = format!(
                    "docket_id={} case={}",
                    case_id,
                    docket.case_name.as_deref().unwrap_or("unknown"),
                );

                let chain = SourceChain::new(
                    source_ref
                        .clone()
                        .with_reference_detail(detail)
                        .with_filing_id(case_id.clone()),
                );

                let court_case = CourtCase {
                    meta: EntityMeta::new(chain),
                    case_id,
                    court: docket.court.clone().unwrap_or_default(),
                    case_type: infer_case_type(
                        docket.nature_of_suit.as_deref(),
                        docket.court.as_deref(),
                    ),
                    parties: Vec::new(),
                    outcome: None,
                    filing_date: parse_date(docket.date_filed.as_deref()),
                    disposition_date: parse_date(docket.date_terminated.as_deref()),
                };

                entities.push(Entity::CourtCase(court_case));
            }

            debug!(
                "RECAP dockets page {}: total={} scraped={}",
                page,
                total,
                entities.len()
            );

            // CourtListener returns 'next' URL; we use page-based pagination for simplicity.
            let fetched = (page * self.config.per_page) as u64;
            if fetched >= total {
                break;
            }
            page += 1;
        }

        Ok(entities)
    }

    // ── Opinions ──────────────────────────────────────────────────────────────

    pub async fn scrape_opinions(
        &self,
        runtime: &ScraperRuntime,
    ) -> ScrapeResult<Vec<Entity>> {
        let mut entities = Vec::new();
        let mut page = 1u32;

        loop {
            if page > self.config.max_pages {
                break;
            }

            let url = Url::parse_with_params(
                &format!("{}/opinions/", self.base_url()),
                &[
                    ("format", "json"),
                    ("page_size", &self.config.per_page.to_string()),
                    ("page", &page.to_string()),
                    ("order_by", "-date_filed"),
                ],
            )
            .map_err(ScrapeError::UrlParse)?;

            let Some(json) = self.fetch_cl_json(&url, runtime).await? else {
                break;
            };

            let raw = serde_json::to_vec(&json).unwrap_or_default();
            let _source_ref = recap_source_ref(&url, &raw);

            let page_data: RecapListPage<RecapOpinion> =
                serde_json::from_value(json).map_err(ScrapeError::Json)?;

            if page_data.results.is_empty() {
                break;
            }

            let total = page_data.count.unwrap_or(0);

            for opinion in page_data.results {
                let case_id = opinion
                    .id
                    .map(|id| format!("opinion-{id}"))
                    .unwrap_or_else(|| "unknown".to_string());

                let detail = format!(
                    "opinion_id={} case={}",
                    case_id,
                    opinion.case_name.as_deref().unwrap_or("unknown"),
                );

                // Build source URL from absolute_url if available, else use the list URL.
                let opinion_source_url = opinion
                    .absolute_url
                    .as_ref()
                    .and_then(|rel| {
                        Url::parse(&format!("https://www.courtlistener.com{rel}")).ok()
                    })
                    .unwrap_or_else(|| url.clone());

                let opinion_source_ref = SourceRef::new(
                    opinion_source_url,
                    ContentHash::compute(raw.as_slice()),
                    SourceType::CourtRecord,
                    "system",
                )
                .with_reference_detail(detail.clone())
                .with_filing_id(case_id.clone());

                let chain = SourceChain::new(opinion_source_ref);

                let court_str = opinion.court.clone().unwrap_or_default();
                let case_type = infer_case_type(None, Some(&court_str));

                let court_case = CourtCase {
                    meta: EntityMeta::new(chain),
                    case_id,
                    court: court_str,
                    case_type,
                    parties: Vec::new(),
                    outcome: None,
                    filing_date: parse_date(opinion.date_filed.as_deref()),
                    disposition_date: None,
                };

                entities.push(Entity::CourtCase(court_case));
            }

            debug!(
                "RECAP opinions page {}: total={} scraped={}",
                page,
                total,
                entities.len()
            );

            let fetched = (page * self.config.per_page) as u64;
            if fetched >= total {
                break;
            }
            page += 1;
        }

        Ok(entities)
    }
}

impl ScrapeSource for RecapScraper {
    fn source_id(&self) -> &str {
        "recap"
    }

    fn scrape_all<'a>(
        &'a self,
        runtime: &'a ScraperRuntime,
    ) -> Pin<Box<dyn Future<Output = ScrapeResult<Vec<Entity>>> + Send + 'a>> {
        Box::pin(async move {
            let mut all = Vec::new();
            all.extend(self.scrape_dockets(runtime).await?);
            all.extend(self.scrape_opinions(runtime).await?);
            Ok(all)
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_config(base_url: String) -> RecapConfig {
        RecapConfig {
            api_token: None,
            base_url,
            max_pages: 5,
            per_page: 20,
            source: crate::config::SourceConfig::default(),
        }
    }

    #[tokio::test]
    async fn test_scrape_dockets_returns_court_cases() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/dockets/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "count": 2,
                "next": null,
                "results": [
                    {
                        "id": 1001,
                        "docket_number": "1:24-cv-00001",
                        "case_name": "United States v. Acme Corp",
                        "court": "dcd",
                        "date_filed": "2024-01-15",
                        "date_terminated": null,
                        "nature_of_suit": "Civil Rights"
                    },
                    {
                        "id": 1002,
                        "docket_number": "2:24-cr-00042",
                        "case_name": "United States v. John Doe",
                        "court": "cacd",
                        "date_filed": "2024-02-01",
                        "date_terminated": "2024-06-30",
                        "nature_of_suit": "Criminal"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let config = make_config(server.uri());
        let scraper = RecapScraper::new(config);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_dockets(&runtime).await.unwrap();
        assert_eq!(entities.len(), 2);

        for entity in &entities {
            assert_eq!(entity.type_name(), "CourtCase");
            // Every entity must have a valid source ref.
            assert!(entity.sources().source_count() >= 1);
            let src_url = entity.sources().primary.source_url.to_string();
            assert!(src_url.contains("/dockets/"), "source URL: {src_url}");
        }

        if let Entity::CourtCase(cc) = &entities[0] {
            assert_eq!(cc.case_id, "1001");
            assert_eq!(cc.filing_date, Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()));
            assert!(cc.disposition_date.is_none());
        }

        if let Entity::CourtCase(cc) = &entities[1] {
            assert_eq!(cc.case_type, CaseType::Criminal); // cacd is a district court; nature_of_suit="Criminal"
            assert!(cc.disposition_date.is_some());
        }
    }

    #[tokio::test]
    async fn test_scrape_opinions_returns_court_cases() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/opinions/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "count": 1,
                "next": null,
                "results": [
                    {
                        "id": 5001,
                        "case_name": "Roe v. State",
                        "court": "scotus",
                        "date_filed": "2024-03-20",
                        "absolute_url": "/opinion/5001/roe-v-state/"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let config = make_config(server.uri());
        let scraper = RecapScraper::new(config);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_opinions(&runtime).await.unwrap();
        assert_eq!(entities.len(), 1);

        if let Entity::CourtCase(cc) = &entities[0] {
            assert_eq!(cc.case_id, "opinion-5001");
            assert_eq!(cc.case_type, CaseType::SupremeCourt);
            assert_eq!(cc.filing_date, Some(NaiveDate::from_ymd_opt(2024, 3, 20).unwrap()));
            // Source URL should be the opinion's canonical URL, not the list endpoint.
            let src_url = cc.meta.sources.primary.source_url.to_string();
            assert!(
                src_url.contains("courtlistener.com"),
                "opinion source URL: {src_url}"
            );
        }
    }

    #[tokio::test]
    async fn test_empty_dockets_returns_empty_vec() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/dockets/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "count": 0,
                "next": null,
                "results": []
            })))
            .mount(&server)
            .await;

        let config = make_config(server.uri());
        let scraper = RecapScraper::new(config);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_dockets(&runtime).await.unwrap();
        assert!(entities.is_empty());
    }

    #[tokio::test]
    async fn test_court_type_inference() {
        assert_eq!(infer_case_type(None, Some("scotus")), CaseType::SupremeCourt);
        assert_eq!(infer_case_type(None, Some("ca9")), CaseType::Appellate);
        assert_eq!(infer_case_type(None, Some("bankr-dcd")), CaseType::Bankruptcy);
        assert_eq!(infer_case_type(Some("Criminal"), Some("dcd")), CaseType::Criminal);
        assert_eq!(infer_case_type(None, Some("dcd")), CaseType::Civil);
    }

    #[tokio::test]
    async fn test_source_trait_dispatch() {
        let server = MockServer::start().await;

        for p in ["/dockets/", "/opinions/"] {
            Mock::given(method("GET"))
                .and(path(p))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "count": 0,
                    "next": null,
                    "results": []
                })))
                .mount(&server)
                .await;
        }

        let config = make_config(server.uri());
        let scraper: Box<dyn ScrapeSource> = Box::new(RecapScraper::new(config));
        assert_eq!(scraper.source_id(), "recap");

        let runtime = ScraperRuntime::new_unlimited();
        let result = scraper.scrape_all(&runtime).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
