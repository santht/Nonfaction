use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

/// Every fact in Nonfaction traces back to a SourceRef.
/// No SourceRef = entity cannot exist. Enforced at the type level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRef {
    /// Unique identifier for this source reference
    pub id: Uuid,
    /// URL of the primary source document
    pub source_url: Url,
    /// SHA-256 hash of the scraped content at time of ingestion
    pub content_hash: ContentHash,
    /// When the source was scraped/archived
    pub scrape_timestamp: DateTime<Utc>,
    /// Internet Archive Wayback Machine URL or internal archive URL
    pub archive_url: Option<Url>,
    /// Filing ID if applicable (FEC filing, PACER case ID, SEC accession number)
    pub filing_id: Option<String>,
    /// Type of source document
    pub source_type: SourceType,
    /// Specific page, section, or quote reference within the document
    pub reference_detail: Option<String>,
    /// Who submitted this source (contributor ID or "system" for automated scraping)
    pub submitted_by: String,
    /// When this source ref was created in the database
    pub created_at: DateTime<Utc>,
}

/// SHA-256 content hash — cryptographic proof of what a source said when we scraped it
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ContentHash(pub String);

impl ContentHash {
    pub fn compute(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Self(format!("{:x}", hasher.finalize()))
    }

    pub fn verify(&self, data: &[u8]) -> bool {
        let computed = Self::compute(data);
        computed.0 == self.0
    }
}

/// Classification of primary source types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SourceType {
    /// FEC campaign finance filing
    FecFiling,
    /// Federal court record via PACER/RECAP
    CourtRecord,
    /// SEC financial disclosure (EDGAR)
    SecFiling,
    /// Congressional Record entry
    CongressionalRecord,
    /// Roll call vote record
    RollCallVote,
    /// DOJ pardon record
    PardonRecord,
    /// Executive order or presidential memorandum
    ExecutiveOrder,
    /// Financial disclosure (OGE, House, Senate)
    FinancialDisclosure,
    /// Lobbying disclosure (LDA/FARA)
    LobbyingDisclosure,
    /// Government press release
    PressRelease,
    /// Official government website
    GovernmentWebsite,
    /// Flight manifest / FAA record
    FlightRecord,
    /// Property record
    PropertyRecord,
    /// Sanctions list entry
    SanctionsList,
    /// State campaign finance filing
    StateCampaignFinance,
    /// State court record
    StateCourtRecord,
    /// Legislative bill text or status
    LegislativeBill,
    /// Government contract (USASpending)
    GovernmentContract,
    /// Inspector General report
    InspectorGeneralReport,
    /// GAO audit report
    GaoReport,
    /// Public statement (C-SPAN, official transcript)
    PublicStatement,
    /// Corporate registry filing
    CorporateRegistry,
    /// Other government record
    OtherGovernment,
}

/// A chain of sources proving a connection or fact.
/// Multiple sources strengthen a claim. Zero sources = impossible to construct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceChain {
    /// Primary source — the strongest/most direct evidence
    pub primary: SourceRef,
    /// Supporting sources — additional evidence
    pub supporting: Vec<SourceRef>,
}

impl SourceChain {
    /// Create a new source chain with a single primary source
    pub fn new(primary: SourceRef) -> Self {
        Self {
            primary,
            supporting: Vec::new(),
        }
    }

    /// Add a supporting source
    pub fn add_supporting(&mut self, source: SourceRef) {
        self.supporting.push(source);
    }

    /// Total number of sources in this chain
    pub fn source_count(&self) -> usize {
        1 + self.supporting.len()
    }

    /// All source URLs in this chain
    pub fn all_urls(&self) -> Vec<&Url> {
        let mut urls = vec![&self.primary.source_url];
        for s in &self.supporting {
            urls.push(&s.source_url);
        }
        urls
    }
}

impl SourceRef {
    pub fn new(
        source_url: Url,
        content_hash: ContentHash,
        source_type: SourceType,
        submitted_by: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_url,
            content_hash,
            scrape_timestamp: Utc::now(),
            archive_url: None,
            filing_id: None,
            source_type,
            reference_detail: None,
            submitted_by: submitted_by.into(),
            created_at: Utc::now(),
        }
    }

    pub fn with_filing_id(mut self, filing_id: impl Into<String>) -> Self {
        self.filing_id = Some(filing_id.into());
        self
    }

    pub fn with_archive_url(mut self, archive_url: Url) -> Self {
        self.archive_url = Some(archive_url);
        self
    }

    pub fn with_reference_detail(mut self, detail: impl Into<String>) -> Self {
        self.reference_detail = Some(detail.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_compute_and_verify() {
        let data = b"FEC Filing 12345 - Contribution $500,000";
        let hash = ContentHash::compute(data);
        assert!(hash.verify(data));
        assert!(!hash.verify(b"tampered data"));
    }

    #[test]
    fn test_content_hash_deterministic() {
        let data = b"same input always same hash";
        let h1 = ContentHash::compute(data);
        let h2 = ContentHash::compute(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_source_ref_builder() {
        let url = Url::parse("https://api.open.fec.gov/v1/schedules/schedule_a/").unwrap();
        let hash = ContentHash::compute(b"test filing data");
        let source = SourceRef::new(url.clone(), hash, SourceType::FecFiling, "system")
            .with_filing_id("FEC-2024-001")
            .with_reference_detail("Page 3, Line 12");

        assert_eq!(source.source_url, url);
        assert_eq!(source.filing_id.as_deref(), Some("FEC-2024-001"));
        assert_eq!(source.source_type, SourceType::FecFiling);
        assert_eq!(source.submitted_by, "system");
    }

    #[test]
    fn test_source_chain() {
        let url1 = Url::parse("https://efts.sec.gov/LATEST/search-index").unwrap();
        let url2 = Url::parse("https://api.open.fec.gov/v1/candidates/").unwrap();
        let primary = SourceRef::new(
            url1,
            ContentHash::compute(b"primary"),
            SourceType::SecFiling,
            "system",
        );
        let supporting = SourceRef::new(
            url2,
            ContentHash::compute(b"supporting"),
            SourceType::FecFiling,
            "system",
        );

        let mut chain = SourceChain::new(primary);
        assert_eq!(chain.source_count(), 1);

        chain.add_supporting(supporting);
        assert_eq!(chain.source_count(), 2);
        assert_eq!(chain.all_urls().len(), 2);
    }
}
