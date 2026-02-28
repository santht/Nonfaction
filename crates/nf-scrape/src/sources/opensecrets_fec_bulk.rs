//! OpenSecrets/FEC bulk campaign-finance scraper.
//!
//! This ingests paginated contribution rows from a bulk endpoint and emits
//! donor/committee/payment entities.

use std::future::Future;
use std::pin::Pin;

use chrono::NaiveDate;
use serde::Deserialize;
use url::Url;

use nf_core::{
    ContentHash, Entity, EntityMeta, Organization, OrganizationType, Payment, PaymentType, Person,
    SourceChain, SourceRef, SourceType,
};

use crate::config::OpenSecretsFecBulkConfig;
use crate::framework::{ScrapeError, ScrapeResult, ScrapeSource, Scraper, ScraperRuntime};

#[derive(Debug, Deserialize)]
struct BulkContributionPage {
    results: Vec<BulkContribution>,
    pagination: Option<BulkPagination>,
}

#[derive(Debug, Deserialize)]
struct BulkPagination {
    pages: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct BulkContribution {
    donor_name: Option<String>,
    committee_name: Option<String>,
    amount: Option<f64>,
    date: Option<String>,
    filing_id: Option<String>,
    election_cycle: Option<String>,
    transaction_id: Option<String>,
}

fn parse_date(value: Option<&str>) -> Option<NaiveDate> {
    let date = value?;
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
}

fn source_ref(url: &Url, raw: &[u8]) -> SourceRef {
    SourceRef::new(
        url.clone(),
        ContentHash::compute(raw),
        SourceType::FecFiling,
        "system",
    )
}

pub struct OpenSecretsFecBulkScraper {
    config: OpenSecretsFecBulkConfig,
}

impl OpenSecretsFecBulkScraper {
    pub fn new(config: OpenSecretsFecBulkConfig) -> Self {
        Self { config }
    }

    fn base_url(&self) -> &str {
        &self.config.base_url
    }

    pub async fn scrape_bulk_contributions(
        &self,
        runtime: &ScraperRuntime,
    ) -> ScrapeResult<Vec<Entity>> {
        let mut entities = Vec::new();
        let mut page = 1u32;

        while page <= self.config.max_pages {
            let url = Url::parse_with_params(
                &format!("{}/bulk/contributions/", self.base_url()),
                &[
                    ("page", page.to_string()),
                    ("per_page", self.config.per_page.to_string()),
                    ("cycle", self.config.election_cycle.clone()),
                ],
            )
            .map_err(ScrapeError::UrlParse)?;

            let Some(json) = self.fetch_json(runtime, &url).await? else {
                break;
            };

            let raw = serde_json::to_vec(&json).unwrap_or_default();
            let src = source_ref(&url, &raw);

            let page_data: BulkContributionPage =
                serde_json::from_value(json).map_err(ScrapeError::Json)?;

            if page_data.results.is_empty() {
                break;
            }

            let total_pages = page_data
                .pagination
                .as_ref()
                .and_then(|p| p.pages)
                .unwrap_or(page as u64);

            for row in page_data.results {
                let amount = match row.amount {
                    Some(v) => v,
                    None => continue,
                };
                let date = match parse_date(row.date.as_deref()) {
                    Some(d) => d,
                    None => continue,
                };

                let detail = format!(
                    "bulk_tx={} donor={} committee={}",
                    row.transaction_id.as_deref().unwrap_or("unknown"),
                    row.donor_name.as_deref().unwrap_or("unknown"),
                    row.committee_name.as_deref().unwrap_or("unknown"),
                );

                let donor = Person::new(
                    row.donor_name.as_deref().unwrap_or("Unknown Bulk Donor"),
                    SourceChain::new(src.clone().with_reference_detail(detail.clone())),
                );
                let donor_id = donor.meta.id;
                entities.push(Entity::Person(donor));

                let committee = Organization::new(
                    row.committee_name
                        .as_deref()
                        .unwrap_or("Unknown Bulk Committee"),
                    OrganizationType::Pac,
                    SourceChain::new(src.clone().with_reference_detail(detail.clone())),
                );
                let committee_id = committee.meta.id;
                entities.push(Entity::Organization(committee));

                let mut payment_source = src.clone().with_reference_detail(detail);
                if let Some(filing_id) = row.filing_id.clone() {
                    payment_source = payment_source.with_filing_id(filing_id);
                }

                entities.push(Entity::Payment(Payment {
                    meta: EntityMeta::new(SourceChain::new(payment_source)),
                    amount,
                    currency: "USD".to_string(),
                    date,
                    donor: donor_id,
                    recipient: committee_id,
                    payment_type: PaymentType::IndividualContribution,
                    filing_id: row.filing_id,
                    election_cycle: row
                        .election_cycle
                        .or_else(|| Some(self.config.election_cycle.clone())),
                    description: Some("OpenSecrets bulk contribution".to_string()),
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

impl Scraper for OpenSecretsFecBulkScraper {
    fn source_id(&self) -> &str {
        "opensecrets_fec_bulk"
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

impl ScrapeSource for OpenSecretsFecBulkScraper {
    fn source_id(&self) -> &str {
        <Self as Scraper>::source_id(self)
    }

    fn scrape_all<'a>(
        &'a self,
        runtime: &'a ScraperRuntime,
    ) -> Pin<Box<dyn Future<Output = ScrapeResult<Vec<Entity>>> + Send + 'a>> {
        Box::pin(async move { self.scrape_bulk_contributions(runtime).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_config(base_url: String) -> OpenSecretsFecBulkConfig {
        OpenSecretsFecBulkConfig {
            api_token: Some("TOKEN123".to_string()),
            base_url,
            source: crate::config::SourceConfig::default(),
            max_pages: 3,
            per_page: 100,
            election_cycle: "2024".to_string(),
        }
    }

    #[tokio::test]
    async fn scrape_bulk_contributions_creates_payment_triplets() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/bulk/contributions/"))
            .and(header("authorization", "Bearer TOKEN123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [
                    {
                        "donor_name": "Acme Donor",
                        "committee_name": "Committee A",
                        "amount": 1250.0,
                        "date": "2024-02-10",
                        "filing_id": "F-100",
                        "election_cycle": "2024",
                        "transaction_id": "TX-1"
                    }
                ],
                "pagination": {"pages": 1}
            })))
            .mount(&server)
            .await;

        let scraper = OpenSecretsFecBulkScraper::new(make_config(server.uri()));
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_bulk_contributions(&runtime).await.unwrap();
        assert_eq!(entities.len(), 3);
        let payment = entities
            .iter()
            .find(|e| e.type_name() == "Payment")
            .unwrap();
        if let Entity::Payment(payment) = payment {
            assert_eq!(payment.amount, 1250.0);
            assert_eq!(payment.filing_id.as_deref(), Some("F-100"));
        }
    }

    #[tokio::test]
    async fn scrape_bulk_contributions_empty_page_stops() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/bulk/contributions/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [],
                "pagination": {"pages": 0}
            })))
            .mount(&server)
            .await;

        let mut cfg = make_config(server.uri());
        cfg.api_token = None;
        let scraper = OpenSecretsFecBulkScraper::new(cfg);
        let runtime = ScraperRuntime::new_unlimited();

        let entities = scraper.scrape_bulk_contributions(&runtime).await.unwrap();
        assert!(entities.is_empty());
    }
}
