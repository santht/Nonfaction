//! FEC Open Data API scraper.
//!
//! Endpoints used:
//! - GET /v1/candidates/         → Person entities
//! - GET /v1/committees/         → Organization entities
//! - GET /v1/schedules/schedule_a/ → Payment entities (contributions)
//! - GET /v1/schedules/schedule_b/ → Payment entities (disbursements)

use std::pin::Pin;
use std::future::Future;

use chrono::NaiveDate;
use serde::Deserialize;
use tracing::{debug, warn};
use url::Url;

use nf_core::{
    ContentHash, Entity, EntityMeta, Organization, OrganizationType, Party, Payment,
    PaymentType, Person, SourceChain, SourceRef, SourceType,
};

use crate::config::FecConfig;
use crate::framework::{ScrapeError, ScrapeResult, ScrapeSource, ScraperRuntime};

// ── API response shapes ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct FecPage<T> {
    results: Vec<T>,
    pagination: Option<FecPagination>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct FecPagination {
    count: Option<u64>,
    pages: Option<u64>,
    page: Option<u64>,
    #[serde(rename = "last_indexes")]
    last_indexes: Option<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct FecCandidate {
    candidate_id: Option<String>,
    name: Option<String>,
    party: Option<String>,
    state: Option<String>,
    office: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct FecCommittee {
    committee_id: Option<String>,
    name: Option<String>,
    committee_type_full: Option<String>,
    state: Option<String>,
    party_full: Option<String>,
    designation_full: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct FecScheduleA {
    contribution_receipt_amount: Option<f64>,
    contribution_receipt_date: Option<String>,
    contributor_name: Option<String>,
    committee_id: Option<String>,
    file_number: Option<serde_json::Value>,
    transaction_id: Option<String>,
    election_type_full: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct FecScheduleB {
    disbursement_amount: Option<f64>,
    disbursement_date: Option<String>,
    disbursement_description: Option<String>,
    committee_id: Option<String>,
    recipient_name: Option<String>,
    file_number: Option<serde_json::Value>,
    transaction_id: Option<String>,
}

// ── Helper: build SourceRef from an API URL + raw JSON bytes ─────────────────

fn fec_source_ref(url: &Url, raw: &[u8]) -> SourceRef {
    SourceRef::new(
        url.clone(),
        ContentHash::compute(raw),
        SourceType::FecFiling,
        "system",
    )
}

fn parse_party(party: Option<&str>) -> Option<Party> {
    match party? {
        "DEM" | "Democrat" => Some(Party::Democrat),
        "REP" | "Republican" => Some(Party::Republican),
        "IND" | "Independent" => Some(Party::Independent),
        "LIB" | "Libertarian" => Some(Party::Libertarian),
        "GRE" | "Green" => Some(Party::Green),
        _ => Some(Party::Other),
    }
}

fn parse_date(s: Option<&str>) -> Option<NaiveDate> {
    let s = s?;
    // FEC uses YYYY-MM-DD
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

fn org_type_from_committee_type(ct: Option<&str>) -> OrganizationType {
    match ct {
        Some(s) if s.contains("Super PAC") || s.contains("Super Pac") => OrganizationType::SuperPac,
        Some(s) if s.contains("PAC") || s.contains("Pac") => OrganizationType::Pac,
        Some(s) if s.contains("Party") => OrganizationType::PoliticalParty,
        _ => OrganizationType::Other,
    }
}

fn filing_id_from_value(v: Option<&serde_json::Value>) -> Option<String> {
    v?.as_i64().map(|n| n.to_string())
        .or_else(|| v?.as_str().map(|s| s.to_string()))
}

// ── FEC scraper ───────────────────────────────────────────────────────────────

pub struct FecScraper {
    config: FecConfig,
}

impl FecScraper {
    pub fn new(config: FecConfig) -> Self {
        Self { config }
    }

    fn base_url(&self) -> &str {
        &self.config.base_url
    }

    fn api_key(&self) -> &str {
        &self.config.api_key
    }

    // ── Candidates ────────────────────────────────────────────────────────────

    pub async fn scrape_candidates(
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
                &format!("{}/candidates/", self.base_url()),
                &[
                    ("api_key", self.api_key()),
                    ("per_page", &self.config.per_page.to_string()),
                    ("page", &page.to_string()),
                    ("election_year", &self.config.election_cycle),
                ],
            )
            .map_err(ScrapeError::UrlParse)?;

            let Some(json) = runtime.fetch_json(&url).await? else {
                break;
            };

            let raw = serde_json::to_vec(&json).unwrap_or_default();
            let source_ref = fec_source_ref(&url, &raw);

            let page_data: FecPage<FecCandidate> = serde_json::from_value(json)
                .map_err(ScrapeError::Json)?;

            if page_data.results.is_empty() {
                break;
            }

            let total_pages = page_data
                .pagination
                .as_ref()
                .and_then(|p| p.pages)
                .unwrap_or(1);

            for candidate in page_data.results {
                let chain = SourceChain::new(
                    source_ref
                        .clone()
                        .with_reference_detail(format!(
                            "candidate_id={}",
                            candidate.candidate_id.as_deref().unwrap_or("unknown")
                        ))
                        .with_filing_id(
                            candidate.candidate_id.clone().unwrap_or_default(),
                        ),
                );

                let mut person = Person::new(
                    candidate.name.as_deref().unwrap_or("Unknown"),
                    chain,
                );
                person.party_affiliation = parse_party(candidate.party.as_deref());
                person.current_role = candidate.office.clone();
                person.status = nf_core::PersonStatus::Candidate;

                entities.push(Entity::Person(person));
            }

            debug!(
                "FEC candidates page {}/{} scraped ({} entities so far)",
                page,
                total_pages,
                entities.len()
            );

            if page >= total_pages as u32 {
                break;
            }
            page += 1;
        }

        Ok(entities)
    }

    // ── Committees ────────────────────────────────────────────────────────────

    pub async fn scrape_committees(
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
                &format!("{}/committees/", self.base_url()),
                &[
                    ("api_key", self.api_key()),
                    ("per_page", &self.config.per_page.to_string()),
                    ("page", &page.to_string()),
                    ("election_year", &self.config.election_cycle),
                ],
            )
            .map_err(ScrapeError::UrlParse)?;

            let Some(json) = runtime.fetch_json(&url).await? else {
                break;
            };

            let raw = serde_json::to_vec(&json).unwrap_or_default();
            let source_ref = fec_source_ref(&url, &raw);

            let page_data: FecPage<FecCommittee> = serde_json::from_value(json)
                .map_err(ScrapeError::Json)?;

            if page_data.results.is_empty() {
                break;
            }

            let total_pages = page_data
                .pagination
                .as_ref()
                .and_then(|p| p.pages)
                .unwrap_or(1);

            for committee in page_data.results {
                let chain = SourceChain::new(
                    source_ref
                        .clone()
                        .with_reference_detail(format!(
                            "committee_id={}",
                            committee.committee_id.as_deref().unwrap_or("unknown")
                        ))
                        .with_filing_id(
                            committee.committee_id.clone().unwrap_or_default(),
                        ),
                );

                let org_type =
                    org_type_from_committee_type(committee.committee_type_full.as_deref());
                let mut org = Organization::new(
                    committee.name.as_deref().unwrap_or("Unknown Committee"),
                    org_type,
                    chain,
                );
                org.jurisdiction = committee.state.map(|s| nf_core::Jurisdiction::State(s));

                entities.push(Entity::Organization(org));
            }

            if page >= total_pages as u32 {
                break;
            }
            page += 1;
        }

        Ok(entities)
    }

    // ── Schedule A (contributions / receipts) ─────────────────────────────────

    pub async fn scrape_schedule_a(
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
                &format!("{}/schedules/schedule_a/", self.base_url()),
                &[
                    ("api_key", self.api_key()),
                    ("per_page", &self.config.per_page.to_string()),
                    ("page", &page.to_string()),
                    ("two_year_transaction_period", &self.config.election_cycle),
                ],
            )
            .map_err(ScrapeError::UrlParse)?;

            let Some(json) = runtime.fetch_json(&url).await? else {
                break;
            };

            let raw = serde_json::to_vec(&json).unwrap_or_default();
            let source_ref = fec_source_ref(&url, &raw);

            let page_data: FecPage<FecScheduleA> = serde_json::from_value(json)
                .map_err(ScrapeError::Json)?;

            if page_data.results.is_empty() {
                break;
            }

            let total_pages = page_data
                .pagination
                .as_ref()
                .and_then(|p| p.pages)
                .unwrap_or(1);

            for record in page_data.results {
                let amount = match record.contribution_receipt_amount {
                    Some(a) => a,
                    None => {
                        warn!("skipping schedule_a record with no amount");
                        continue;
                    }
                };
                let date = match parse_date(record.contribution_receipt_date.as_deref()) {
                    Some(d) => d,
                    None => {
                        warn!("skipping schedule_a record with unparseable date");
                        continue;
                    }
                };

                let detail = format!(
                    "contributor={} committee={} amount={}",
                    record.contributor_name.as_deref().unwrap_or("unknown"),
                    record.committee_id.as_deref().unwrap_or("unknown"),
                    amount,
                );

                let contributor_chain = SourceChain::new(
                    source_ref.clone().with_reference_detail(detail.clone()),
                );
                let contributor = Person::new(
                    record.contributor_name.as_deref().unwrap_or("Unknown Contributor"),
                    contributor_chain,
                );
                let contributor_id = contributor.meta.id;
                entities.push(Entity::Person(contributor));

                let committee_name = record
                    .committee_id
                    .as_deref()
                    .unwrap_or("Unknown Committee")
                    .to_string();
                let committee_chain = SourceChain::new(
                    source_ref.clone().with_reference_detail(detail.clone()),
                );
                let committee =
                    Organization::new(&committee_name, OrganizationType::Pac, committee_chain);
                let committee_id = committee.meta.id;
                entities.push(Entity::Organization(committee));

                let filing_id = filing_id_from_value(record.file_number.as_ref());
                let mut payment_source = source_ref.clone().with_reference_detail(detail);
                if let Some(fid) = &filing_id {
                    payment_source = payment_source.with_filing_id(fid.clone());
                }
                let payment = Payment {
                    meta: EntityMeta::new(SourceChain::new(payment_source)),
                    amount,
                    currency: "USD".to_string(),
                    date,
                    donor: contributor_id,
                    recipient: committee_id,
                    payment_type: PaymentType::IndividualContribution,
                    filing_id,
                    election_cycle: Some(self.config.election_cycle.clone()),
                    description: record.election_type_full,
                };
                entities.push(Entity::Payment(payment));
            }

            if page >= total_pages as u32 {
                break;
            }
            page += 1;
        }

        Ok(entities)
    }

    // ── Schedule B (disbursements) ────────────────────────────────────────────

    pub async fn scrape_schedule_b(
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
                &format!("{}/schedules/schedule_b/", self.base_url()),
                &[
                    ("api_key", self.api_key()),
                    ("per_page", &self.config.per_page.to_string()),
                    ("page", &page.to_string()),
                    ("two_year_transaction_period", &self.config.election_cycle),
                ],
            )
            .map_err(ScrapeError::UrlParse)?;

            let Some(json) = runtime.fetch_json(&url).await? else {
                break;
            };

            let raw = serde_json::to_vec(&json).unwrap_or_default();
            let source_ref = fec_source_ref(&url, &raw);

            let page_data: FecPage<FecScheduleB> = serde_json::from_value(json)
                .map_err(ScrapeError::Json)?;

            if page_data.results.is_empty() {
                break;
            }

            let total_pages = page_data
                .pagination
                .as_ref()
                .and_then(|p| p.pages)
                .unwrap_or(1);

            for record in page_data.results {
                let amount = match record.disbursement_amount {
                    Some(a) => a,
                    None => continue,
                };
                let date = match parse_date(record.disbursement_date.as_deref()) {
                    Some(d) => d,
                    None => continue,
                };

                let detail = format!(
                    "committee={} recipient={} amount={}",
                    record.committee_id.as_deref().unwrap_or("unknown"),
                    record.recipient_name.as_deref().unwrap_or("unknown"),
                    amount,
                );

                let payer_name = record
                    .committee_id
                    .as_deref()
                    .unwrap_or("Unknown Committee")
                    .to_string();
                let payer_chain =
                    SourceChain::new(source_ref.clone().with_reference_detail(detail.clone()));
                let payer = Organization::new(&payer_name, OrganizationType::Pac, payer_chain);
                let payer_id = payer.meta.id;
                entities.push(Entity::Organization(payer));

                let recipient_chain =
                    SourceChain::new(source_ref.clone().with_reference_detail(detail.clone()));
                let recipient = Organization::new(
                    record.recipient_name.as_deref().unwrap_or("Unknown Recipient"),
                    OrganizationType::Other,
                    recipient_chain,
                );
                let recipient_id = recipient.meta.id;
                entities.push(Entity::Organization(recipient));

                let filing_id = filing_id_from_value(record.file_number.as_ref());
                let mut payment_source = source_ref.clone().with_reference_detail(detail);
                if let Some(fid) = &filing_id {
                    payment_source = payment_source.with_filing_id(fid.clone());
                }
                let payment = Payment {
                    meta: EntityMeta::new(SourceChain::new(payment_source)),
                    amount,
                    currency: "USD".to_string(),
                    date,
                    donor: payer_id,
                    recipient: recipient_id,
                    payment_type: PaymentType::PacContribution,
                    filing_id,
                    election_cycle: Some(self.config.election_cycle.clone()),
                    description: record.disbursement_description,
                };
                entities.push(Entity::Payment(payment));
            }

            if page >= total_pages as u32 {
                break;
            }
            page += 1;
        }

        Ok(entities)
    }
}

impl ScrapeSource for FecScraper {
    fn source_id(&self) -> &str {
        "fec"
    }

    fn scrape_all<'a>(
        &'a self,
        runtime: &'a ScraperRuntime,
    ) -> Pin<Box<dyn Future<Output = ScrapeResult<Vec<Entity>>> + Send + 'a>> {
        Box::pin(async move {
            let mut all = Vec::new();
            all.extend(self.scrape_candidates(runtime).await?);
            all.extend(self.scrape_committees(runtime).await?);
            all.extend(self.scrape_schedule_a(runtime).await?);
            all.extend(self.scrape_schedule_b(runtime).await?);
            Ok(all)
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_config(base_url: String) -> FecConfig {
        FecConfig {
            api_key: "DEMO_KEY".to_string(),
            base_url,
            election_cycle: "2024".to_string(),
            max_pages: 5,
            per_page: 20,
            source: crate::config::SourceConfig::default(),
        }
    }

    fn candidate_page(count: u64, results: serde_json::Value) -> serde_json::Value {
        json!({
            "results": results,
            "pagination": {
                "count": count,
                "pages": 1,
                "page": 1,
                "per_page": 20,
                "last_indexes": null
            }
        })
    }

    #[tokio::test]
    async fn test_scrape_candidates_returns_persons() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/candidates/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(candidate_page(
                2,
                json!([
                    {
                        "candidate_id": "P00000001",
                        "name": "JANE DOE",
                        "party": "DEM",
                        "state": "CA",
                        "office": "P"
                    },
                    {
                        "candidate_id": "P00000002",
                        "name": "JOHN SMITH",
                        "party": "REP",
                        "state": "TX",
                        "office": "S"
                    }
                ]),
            )))
            .mount(&server)
            .await;

        let config = make_config(server.uri());
        let scraper = FecScraper::new(config);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_candidates(&runtime).await.unwrap();
        assert_eq!(entities.len(), 2);

        for entity in &entities {
            // Every entity must have at least one source ref.
            assert!(entity.sources().source_count() >= 1);
            assert_eq!(entity.type_name(), "Person");
        }

        // Check party parsing.
        if let Entity::Person(p) = &entities[0] {
            assert_eq!(p.party_affiliation, Some(Party::Democrat));
            assert_eq!(p.name, "JANE DOE");
        }
        if let Entity::Person(p) = &entities[1] {
            assert_eq!(p.party_affiliation, Some(Party::Republican));
        }
    }

    #[tokio::test]
    async fn test_scrape_committees_returns_organizations() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/committees/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [
                    {
                        "committee_id": "C00000001",
                        "name": "FRIENDS OF JANE DOE",
                        "committee_type_full": "PAC - Separate Segregated Fund",
                        "state": "CA",
                        "party_full": "Democratic Party",
                        "designation_full": "Principal campaign committee"
                    }
                ],
                "pagination": {"count": 1, "pages": 1, "page": 1, "per_page": 20}
            })))
            .mount(&server)
            .await;

        let config = make_config(server.uri());
        let scraper = FecScraper::new(config);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_committees(&runtime).await.unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].type_name(), "Organization");
        assert!(entities[0].sources().source_count() >= 1);
    }

    #[tokio::test]
    async fn test_scrape_schedule_a_creates_payment_with_source() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/schedules/schedule_a/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [
                    {
                        "contribution_receipt_amount": 500000.0,
                        "contribution_receipt_date": "2024-03-15",
                        "contributor_name": "BIG DONOR LLC",
                        "committee_id": "C00000001",
                        "file_number": 12345,
                        "transaction_id": "TXN001",
                        "election_type_full": "General"
                    }
                ],
                "pagination": {"count": 1, "pages": 1, "page": 1, "per_page": 20}
            })))
            .mount(&server)
            .await;

        let config = make_config(server.uri());
        let scraper = FecScraper::new(config);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_schedule_a(&runtime).await.unwrap();
        // One contribution → Person (donor) + Organization (committee) + Payment
        assert_eq!(entities.len(), 3);

        let payment = entities.iter().find(|e| e.type_name() == "Payment").unwrap();
        assert!(payment.sources().source_count() >= 1);
        assert!(!payment.sources().primary.source_url.to_string().is_empty());

        if let Entity::Payment(p) = payment {
            assert_eq!(p.amount, 500_000.0);
            assert_eq!(p.currency, "USD");
            assert_eq!(p.filing_id.as_deref(), Some("12345"));
            assert_eq!(p.election_cycle.as_deref(), Some("2024"));
        }
    }

    #[tokio::test]
    async fn test_scrape_schedule_b_creates_disbursement_payment() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/schedules/schedule_b/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [
                    {
                        "disbursement_amount": 75000.0,
                        "disbursement_date": "2024-04-01",
                        "disbursement_description": "Media buy",
                        "committee_id": "C00000001",
                        "recipient_name": "AD AGENCY INC",
                        "file_number": 99999,
                        "transaction_id": "TXN002"
                    }
                ],
                "pagination": {"count": 1, "pages": 1, "page": 1, "per_page": 20}
            })))
            .mount(&server)
            .await;

        let config = make_config(server.uri());
        let scraper = FecScraper::new(config);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_schedule_b(&runtime).await.unwrap();
        assert_eq!(entities.len(), 3); // payer org + recipient org + payment

        if let Some(Entity::Payment(p)) = entities.iter().find(|e| e.type_name() == "Payment") {
            assert_eq!(p.amount, 75_000.0);
            assert_eq!(p.payment_type, PaymentType::PacContribution);
            assert_eq!(p.description.as_deref(), Some("Media buy"));
        }
    }

    #[tokio::test]
    async fn test_scrape_empty_results_returns_empty_vec() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/candidates/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [],
                "pagination": {"count": 0, "pages": 0, "page": 1, "per_page": 20}
            })))
            .mount(&server)
            .await;

        let config = make_config(server.uri());
        let scraper = FecScraper::new(config);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_candidates(&runtime).await.unwrap();
        assert!(entities.is_empty());
    }

    #[tokio::test]
    async fn test_schedule_a_skips_records_with_no_date() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/schedules/schedule_a/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [
                    {
                        "contribution_receipt_amount": 1000.0,
                        "contribution_receipt_date": null,
                        "contributor_name": "ANON",
                        "committee_id": "C00000001",
                        "file_number": null,
                        "transaction_id": null
                    }
                ],
                "pagination": {"count": 1, "pages": 1, "page": 1, "per_page": 20}
            })))
            .mount(&server)
            .await;

        let config = make_config(server.uri());
        let scraper = FecScraper::new(config);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_schedule_a(&runtime).await.unwrap();
        assert!(entities.is_empty(), "records without dates should be skipped");
    }

    #[tokio::test]
    async fn test_source_trait_dispatch() {
        let server = MockServer::start().await;

        // Register all four endpoints with empty results so scrape_all completes.
        for path_str in ["/candidates/", "/committees/", "/schedules/schedule_a/", "/schedules/schedule_b/"] {
            Mock::given(method("GET"))
                .and(path(path_str))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "results": [],
                    "pagination": {"count": 0, "pages": 0, "page": 1, "per_page": 20}
                })))
                .mount(&server)
                .await;
        }

        let config = make_config(server.uri());
        let scraper: Box<dyn ScrapeSource> = Box::new(FecScraper::new(config));
        assert_eq!(scraper.source_id(), "fec");

        let runtime = ScraperRuntime::new_unlimited();
        let entities = scraper.scrape_all(&runtime).await.unwrap();
        assert!(entities.is_empty());
    }
}
