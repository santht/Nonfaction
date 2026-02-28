//! Congress.gov API scraper.
//!
//! Endpoints used:
//! - GET /v3/member/            → Person entities (members of Congress)
//! - GET /v3/bill/{congress}/   → PolicyDecision entities (bills)

use std::future::Future;
use std::pin::Pin;

use chrono::NaiveDate;
use serde::Deserialize;
use tracing::debug;
use url::Url;

use nf_core::{
    ContentHash, Entity, EntityId, EntityMeta, Party, Person, PersonStatus, PolicyDecision,
    PolicyDecisionType, SourceChain, SourceRef, SourceType,
};

use crate::config::CongressConfig;
use crate::framework::{ScrapeError, ScrapeResult, ScrapeSource, Scraper, ScraperRuntime};

// ── API response shapes ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CongressMemberPage {
    members: Vec<CongressMember>,
    pagination: Option<CongressPagination>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CongressBillPage {
    bills: Vec<CongressBill>,
    pagination: Option<CongressPagination>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CongressPagination {
    count: Option<u64>,
    total: Option<u64>,
    offset: Option<u64>,
    next: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CongressMember {
    bioguide_id: Option<String>,
    name: Option<String>,
    party_name: Option<String>,
    state: Option<String>,
    district: Option<u32>,
    #[serde(rename = "terms")]
    terms: Option<serde_json::Value>,
    depiction: Option<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CongressBill {
    number: Option<String>,
    title: Option<String>,
    congress: Option<u32>,
    #[serde(rename = "type")]
    bill_type: Option<String>,
    introduced_date: Option<String>,
    sponsors: Option<Vec<BillSponsor>>,
    latest_action: Option<LatestAction>,
    policy_area: Option<PolicyArea>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BillSponsor {
    bioguide_id: Option<String>,
    full_name: Option<String>,
    party: Option<String>,
    state: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestAction {
    action_date: Option<String>,
    text: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PolicyArea {
    name: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn congress_source_ref(url: &Url, raw: &[u8]) -> SourceRef {
    SourceRef::new(
        url.clone(),
        ContentHash::compute(raw),
        SourceType::CongressionalRecord,
        "system",
    )
}

fn parse_party(party: Option<&str>) -> Option<Party> {
    match party? {
        "Democrat" | "Democratic" | "D" => Some(Party::Democrat),
        "Republican" | "R" => Some(Party::Republican),
        "Independent" | "I" => Some(Party::Independent),
        _ => Some(Party::Other),
    }
}

fn parse_date(s: Option<&str>) -> Option<NaiveDate> {
    let s = s?;
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

// ── Congress scraper ──────────────────────────────────────────────────────────

pub struct CongressScraper {
    config: CongressConfig,
}

impl CongressScraper {
    pub fn new(config: CongressConfig) -> Self {
        Self { config }
    }

    fn base_url(&self) -> &str {
        &self.config.base_url
    }

    fn api_key(&self) -> &str {
        &self.config.api_key
    }

    // ── Members ───────────────────────────────────────────────────────────────

    pub async fn scrape_members(&self, runtime: &ScraperRuntime) -> ScrapeResult<Vec<Entity>> {
        let mut entities = Vec::new();
        let mut offset = 0u32;

        loop {
            let url = Url::parse_with_params(
                &format!("{}/member/", self.base_url()),
                &[
                    ("api_key", self.api_key()),
                    ("limit", &self.config.per_page.to_string()),
                    ("offset", &offset.to_string()),
                    ("format", "json"),
                ],
            )
            .map_err(ScrapeError::UrlParse)?;

            let Some(json) = self.fetch_json(runtime, &url).await? else {
                break;
            };

            let raw = serde_json::to_vec(&json).unwrap_or_default();
            let source_ref = congress_source_ref(&url, &raw);

            let page: CongressMemberPage =
                serde_json::from_value(json).map_err(ScrapeError::Json)?;

            if page.members.is_empty() {
                break;
            }

            let total = page
                .pagination
                .as_ref()
                .and_then(|p| p.count.or(p.total))
                .unwrap_or(0);

            for member in page.members {
                let bioguide = member.bioguide_id.clone().unwrap_or_default();
                let chain = SourceChain::new(
                    source_ref
                        .clone()
                        .with_reference_detail(format!("bioguide_id={bioguide}"))
                        .with_filing_id(bioguide),
                );

                let mut person =
                    Person::new(member.name.as_deref().unwrap_or("Unknown Member"), chain);
                person.party_affiliation = parse_party(member.party_name.as_deref());
                person.jurisdiction = member.state.map(|s| nf_core::Jurisdiction::State(s));
                person.status = PersonStatus::Active;
                person.current_role = Some("Member of Congress".to_string());

                entities.push(Entity::Person(person));
            }

            debug!(
                "Congress members: offset={} total={} scraped={}",
                offset,
                total,
                entities.len()
            );

            offset += self.config.per_page;
            if offset as u64 >= total
                || entities.len() >= (self.config.max_pages * self.config.per_page) as usize
            {
                break;
            }
        }

        Ok(entities)
    }

    // ── Bills ─────────────────────────────────────────────────────────────────

    pub async fn scrape_bills(&self, runtime: &ScraperRuntime) -> ScrapeResult<Vec<Entity>> {
        let mut entities = Vec::new();
        let mut offset = 0u32;

        loop {
            let url = Url::parse_with_params(
                &format!("{}/bill/{}/", self.base_url(), self.config.congress_number),
                &[
                    ("api_key", self.api_key()),
                    ("limit", &self.config.per_page.to_string()),
                    ("offset", &offset.to_string()),
                    ("format", "json"),
                ],
            )
            .map_err(ScrapeError::UrlParse)?;

            let Some(json) = self.fetch_json(runtime, &url).await? else {
                break;
            };

            let raw = serde_json::to_vec(&json).unwrap_or_default();
            let source_ref = congress_source_ref(&url, &raw);

            let page: CongressBillPage = serde_json::from_value(json).map_err(ScrapeError::Json)?;

            if page.bills.is_empty() {
                break;
            }

            let total = page
                .pagination
                .as_ref()
                .and_then(|p| p.count.or(p.total))
                .unwrap_or(0);

            for bill in page.bills {
                let bill_number = bill.number.as_deref().unwrap_or("unknown").to_string();
                let bill_type = bill.bill_type.as_deref().unwrap_or("").to_string();
                let congress_num = bill.congress.unwrap_or(self.config.congress_number);
                let reference = format!("{bill_type}{bill_number} ({congress_num}th Congress)");

                let date = parse_date(bill.introduced_date.as_deref().or_else(|| {
                    bill.latest_action
                        .as_ref()
                        .and_then(|a| a.action_date.as_deref())
                }))
                .unwrap_or(NaiveDate::from_ymd_opt(2024, 1, 1).expect("constant date"));

                // Use first sponsor as the official; fall back to a placeholder.
                let sponsor = bill.sponsors.as_ref().and_then(|s| s.first());

                let official_id = EntityId::new();

                // If we have a sponsor, emit a Person entity too.
                if let Some(sp) = sponsor {
                    let bioguide = sp.bioguide_id.clone().unwrap_or_default();
                    let sp_chain = SourceChain::new(
                        source_ref
                            .clone()
                            .with_reference_detail(format!("sponsor={bioguide}"))
                            .with_filing_id(bioguide),
                    );
                    let mut sp_person = Person::new(
                        sp.full_name.as_deref().unwrap_or("Unknown Sponsor"),
                        sp_chain,
                    );
                    sp_person.party_affiliation = parse_party(sp.party.as_deref());
                    sp_person.jurisdiction =
                        sp.state.clone().map(|s| nf_core::Jurisdiction::State(s));
                    // Use a fresh ID for sponsor entity.
                    entities.push(Entity::Person(sp_person));
                }

                let action_text = bill
                    .latest_action
                    .as_ref()
                    .and_then(|a| a.text.clone())
                    .unwrap_or_default();

                let description = format!(
                    "{}: {}",
                    bill.title.as_deref().unwrap_or(&reference),
                    action_text,
                );

                let chain = SourceChain::new(
                    source_ref
                        .clone()
                        .with_reference_detail(reference.clone())
                        .with_filing_id(reference.clone()),
                );

                let policy = PolicyDecision {
                    meta: EntityMeta::new(chain),
                    official: official_id,
                    date,
                    description,
                    decision_type: PolicyDecisionType::LegislativeVote,
                    beneficiaries: Vec::new(),
                    reference_number: Some(reference),
                    vote: None,
                };

                entities.push(Entity::PolicyDecision(policy));
            }

            offset += self.config.per_page;
            if offset as u64 >= total
                || entities.len() >= (self.config.max_pages * self.config.per_page) as usize
            {
                break;
            }
        }

        Ok(entities)
    }
}

impl Scraper for CongressScraper {
    fn source_id(&self) -> &str {
        "congress"
    }

    fn source_config(&self) -> &crate::config::SourceConfig {
        &self.config.source
    }
}

impl ScrapeSource for CongressScraper {
    fn source_id(&self) -> &str {
        <Self as Scraper>::source_id(self)
    }

    fn scrape_all<'a>(
        &'a self,
        runtime: &'a ScraperRuntime,
    ) -> Pin<Box<dyn Future<Output = ScrapeResult<Vec<Entity>>> + Send + 'a>> {
        Box::pin(async move {
            let mut all = Vec::new();
            all.extend(self.scrape_members(runtime).await?);
            all.extend(self.scrape_bills(runtime).await?);
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

    fn make_config(base_url: String) -> CongressConfig {
        CongressConfig {
            api_key: "DEMO_KEY".to_string(),
            base_url,
            congress_number: 118,
            max_pages: 5,
            per_page: 20,
            source: crate::config::SourceConfig::default(),
        }
    }

    #[tokio::test]
    async fn test_scrape_members_returns_persons() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/member/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "members": [
                    {
                        "bioguideId": "A000001",
                        "name": "DOE, JANE",
                        "partyName": "Democrat",
                        "state": "CA",
                        "district": 12
                    },
                    {
                        "bioguideId": "B000002",
                        "name": "SMITH, JOHN",
                        "partyName": "Republican",
                        "state": "TX",
                        "district": null
                    }
                ],
                "pagination": {"count": 2, "total": 2, "offset": 0}
            })))
            .mount(&server)
            .await;

        let config = make_config(server.uri());
        let scraper = CongressScraper::new(config);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_members(&runtime).await.unwrap();
        assert_eq!(entities.len(), 2);

        for entity in &entities {
            assert_eq!(entity.type_name(), "Person");
            assert!(entity.sources().source_count() >= 1);
            // Source URL must point to the congress API.
            let url = entity.sources().primary.source_url.to_string();
            assert!(
                url.contains("/member/"),
                "URL should point to member endpoint: {url}"
            );
        }

        if let Entity::Person(p) = &entities[0] {
            assert_eq!(p.name, "DOE, JANE");
            assert_eq!(p.party_affiliation, Some(Party::Democrat));
        }
    }

    #[tokio::test]
    async fn test_scrape_bills_returns_policy_decisions() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/bill/118/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "bills": [
                    {
                        "number": "1234",
                        "title": "Transparency in Government Act",
                        "congress": 118,
                        "type": "HR",
                        "introducedDate": "2023-03-01",
                        "sponsors": [
                            {
                                "bioguideId": "A000001",
                                "fullName": "Rep. Jane Doe (D-CA)",
                                "party": "Democrat",
                                "state": "CA"
                            }
                        ],
                        "latestAction": {
                            "actionDate": "2023-05-15",
                            "text": "Passed House"
                        },
                        "policyArea": {"name": "Government Operations"}
                    }
                ],
                "pagination": {"count": 1, "total": 1, "offset": 0}
            })))
            .mount(&server)
            .await;

        let config = make_config(server.uri());
        let scraper = CongressScraper::new(config);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_bills(&runtime).await.unwrap();
        // Sponsor Person + PolicyDecision
        assert_eq!(entities.len(), 2);

        let policy = entities
            .iter()
            .find(|e| e.type_name() == "PolicyDecision")
            .unwrap();
        assert!(policy.sources().source_count() >= 1);

        if let Entity::PolicyDecision(pd) = policy {
            assert!(pd.description.contains("Transparency in Government Act"));
            assert_eq!(pd.decision_type, PolicyDecisionType::LegislativeVote);
            assert!(pd.reference_number.as_deref().unwrap().contains("HR1234"));
        }
    }

    #[tokio::test]
    async fn test_empty_members_returns_empty_vec() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/member/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "members": [],
                "pagination": {"count": 0, "total": 0, "offset": 0}
            })))
            .mount(&server)
            .await;

        let config = make_config(server.uri());
        let scraper = CongressScraper::new(config);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_members(&runtime).await.unwrap();
        assert!(entities.is_empty());
    }

    #[tokio::test]
    async fn test_source_trait_dispatch() {
        let server = MockServer::start().await;

        for path_str in ["/member/", "/bill/118/"] {
            Mock::given(method("GET"))
                .and(path(path_str))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "members": [],
                    "bills": [],
                    "pagination": {"count": 0, "total": 0, "offset": 0}
                })))
                .mount(&server)
                .await;
        }

        let config = make_config(server.uri());
        let scraper: Box<dyn ScrapeSource> = Box::new(CongressScraper::new(config));
        assert_eq!(scraper.source_id(), "congress");

        let runtime = ScraperRuntime::new_unlimited();
        let result = scraper.scrape_all(&runtime).await;
        assert!(result.is_ok());
    }
}
