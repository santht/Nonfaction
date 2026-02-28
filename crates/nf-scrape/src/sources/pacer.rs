//! PACER-compatible court records scraper.

use std::future::Future;
use std::pin::Pin;

use chrono::NaiveDate;
use serde::Deserialize;
use url::Url;

use nf_core::{
    CaseType, ContentHash, CourtCase, Entity, EntityMeta, SourceChain, SourceRef, SourceType,
};

use crate::config::PacerConfig;
use crate::framework::{ScrapeError, ScrapeResult, ScrapeSource, Scraper, ScraperRuntime};

#[derive(Debug, Deserialize)]
struct PacerCasePage {
    results: Vec<PacerCaseRecord>,
    pagination: Option<PacerPagination>,
}

#[derive(Debug, Deserialize)]
struct PacerPagination {
    pages: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct PacerCaseRecord {
    case_id: Option<String>,
    docket_number: Option<String>,
    case_name: Option<String>,
    court: Option<String>,
    case_type: Option<String>,
    filed_date: Option<String>,
    terminated_date: Option<String>,
}

fn parse_date(value: Option<&str>) -> Option<NaiveDate> {
    let date = value?;
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
}

fn parse_case_type(case_type: Option<&str>) -> CaseType {
    match case_type.map(|s| s.to_ascii_lowercase()) {
        Some(kind) if kind.contains("criminal") => CaseType::Criminal,
        Some(kind) if kind.contains("bankruptcy") => CaseType::Bankruptcy,
        Some(kind) if kind.contains("appeal") || kind.contains("appellate") => CaseType::Appellate,
        Some(kind) if kind.contains("supreme") => CaseType::SupremeCourt,
        Some(kind) if kind.contains("administrative") => CaseType::Administrative,
        Some(kind) if kind.contains("civil") => CaseType::Civil,
        _ => CaseType::Other,
    }
}

fn source_ref(url: &Url, raw: &[u8]) -> SourceRef {
    SourceRef::new(
        url.clone(),
        ContentHash::compute(raw),
        SourceType::CourtRecord,
        "system",
    )
}

pub struct PacerScraper {
    config: PacerConfig,
}

impl PacerScraper {
    pub fn new(config: PacerConfig) -> Self {
        Self { config }
    }

    fn base_url(&self) -> &str {
        &self.config.base_url
    }

    pub async fn scrape_cases(&self, runtime: &ScraperRuntime) -> ScrapeResult<Vec<Entity>> {
        let mut entities = Vec::new();
        let mut page = 1u32;

        while page <= self.config.max_pages {
            let url = Url::parse_with_params(
                &format!("{}/cases/", self.base_url()),
                &[
                    ("page", page.to_string()),
                    ("per_page", self.config.per_page.to_string()),
                    ("format", "json".to_string()),
                ],
            )
            .map_err(ScrapeError::UrlParse)?;

            let Some(json) = self.fetch_json(runtime, &url).await? else {
                break;
            };

            let raw = serde_json::to_vec(&json).unwrap_or_default();
            let src = source_ref(&url, &raw);
            let page_data: PacerCasePage =
                serde_json::from_value(json).map_err(ScrapeError::Json)?;

            if page_data.results.is_empty() {
                break;
            }

            let total_pages = page_data
                .pagination
                .as_ref()
                .and_then(|p| p.pages)
                .unwrap_or(page as u64);

            for case in page_data.results {
                let case_id = case
                    .case_id
                    .or(case.docket_number)
                    .unwrap_or_else(|| "unknown-case".to_string());
                let detail = format!(
                    "case_id={} case={}",
                    case_id,
                    case.case_name.as_deref().unwrap_or("unknown"),
                );

                let chain = SourceChain::new(
                    src.clone()
                        .with_reference_detail(detail)
                        .with_filing_id(case_id.clone()),
                );

                entities.push(Entity::CourtCase(CourtCase {
                    meta: EntityMeta::new(chain),
                    case_id,
                    court: case.court.unwrap_or_else(|| "unknown".to_string()),
                    case_type: parse_case_type(case.case_type.as_deref()),
                    parties: Vec::new(),
                    outcome: None,
                    filing_date: parse_date(case.filed_date.as_deref()),
                    disposition_date: parse_date(case.terminated_date.as_deref()),
                }));
            }

            if page as u64 >= total_pages {
                break;
            }
            page += 1;
        }

        Ok(entities)
    }
}

impl Scraper for PacerScraper {
    fn source_id(&self) -> &str {
        "pacer"
    }

    fn source_config(&self) -> &crate::config::SourceConfig {
        &self.config.source
    }

    fn prepare_request(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.config.api_token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }
}

impl ScrapeSource for PacerScraper {
    fn source_id(&self) -> &str {
        <Self as Scraper>::source_id(self)
    }

    fn scrape_all<'a>(
        &'a self,
        runtime: &'a ScraperRuntime,
    ) -> Pin<Box<dyn Future<Output = ScrapeResult<Vec<Entity>>> + Send + 'a>> {
        Box::pin(async move { self.scrape_cases(runtime).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_config(base_url: String) -> PacerConfig {
        PacerConfig {
            api_token: Some("PACER_TOKEN".to_string()),
            base_url,
            source: crate::config::SourceConfig::default(),
            max_pages: 3,
            per_page: 25,
        }
    }

    #[tokio::test]
    async fn scrape_cases_returns_court_cases() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/cases/"))
            .and(header("authorization", "Bearer PACER_TOKEN"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [
                    {
                        "case_id": "24-cv-1001",
                        "docket_number": "1:24-cv-1001",
                        "case_name": "United States v. Acme",
                        "court": "dcd",
                        "case_type": "Civil",
                        "filed_date": "2024-01-20",
                        "terminated_date": null
                    }
                ],
                "pagination": {"pages": 1}
            })))
            .mount(&server)
            .await;

        let scraper = PacerScraper::new(make_config(server.uri()));
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_cases(&runtime).await.unwrap();
        assert_eq!(entities.len(), 1);

        match &entities[0] {
            Entity::CourtCase(case) => {
                assert_eq!(case.case_id, "24-cv-1001");
                assert_eq!(case.case_type, CaseType::Civil);
                assert_eq!(case.filing_date, NaiveDate::from_ymd_opt(2024, 1, 20));
            }
            other => panic!("unexpected entity type: {}", other.type_name()),
        }
    }

    #[tokio::test]
    async fn scrape_cases_stops_on_empty_page() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/cases/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [],
                "pagination": {"pages": 0}
            })))
            .mount(&server)
            .await;

        let mut cfg = make_config(server.uri());
        cfg.api_token = None;
        let scraper = PacerScraper::new(cfg);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_cases(&runtime).await.unwrap();
        assert!(entities.is_empty());
    }
}
