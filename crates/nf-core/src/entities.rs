use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::source::SourceChain;

/// Every entity MUST have a source chain. This is enforced by requiring SourceChain
/// in every constructor. There is no Default impl — you cannot create an entity without sources.

/// Unique entity identifier
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntityId(pub Uuid);

impl EntityId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Common metadata shared by all entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMeta {
    pub id: EntityId,
    pub sources: SourceChain,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Wikidata QID for disambiguation (e.g., Q76 = Barack Obama)
    pub wikidata_qid: Option<String>,
    /// OpenSanctions entity ID if cross-referenced
    pub open_sanctions_id: Option<String>,
    /// Version counter — incremented on every update
    pub version: u64,
    /// Tags for categorization and filtering
    pub tags: Vec<String>,
}

impl EntityMeta {
    pub fn new(sources: SourceChain) -> Self {
        let now = Utc::now();
        Self {
            id: EntityId::new(),
            sources,
            created_at: now,
            updated_at: now,
            wikidata_qid: None,
            open_sanctions_id: None,
            version: 1,
            tags: Vec::new(),
        }
    }
}

// ─── FtM Core Entities ───────────────────────────────────────────────────────

/// A public official, donor, or other person of interest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub meta: EntityMeta,
    pub name: String,
    pub aliases: Vec<String>,
    pub current_role: Option<String>,
    pub party_affiliation: Option<Party>,
    pub jurisdiction: Option<Jurisdiction>,
    pub status: PersonStatus,
    pub birth_date: Option<NaiveDate>,
}

impl Person {
    pub fn new(name: impl Into<String>, sources: SourceChain) -> Self {
        Self {
            meta: EntityMeta::new(sources),
            name: name.into(),
            aliases: Vec::new(),
            current_role: None,
            party_affiliation: None,
            jurisdiction: None,
            status: PersonStatus::Active,
            birth_date: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Party {
    Democrat,
    Republican,
    Independent,
    Libertarian,
    Green,
    Other,
    NonPartisan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Jurisdiction {
    Federal,
    State(String),
    County(String, String),    // (state, county)
    Municipal(String, String), // (state, city)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PersonStatus {
    Active,
    Former,
    Deceased,
    Candidate,
}

/// An organization — PAC, corporation, nonprofit, government agency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub meta: EntityMeta,
    pub name: String,
    pub aliases: Vec<String>,
    pub org_type: OrganizationType,
    pub jurisdiction: Option<Jurisdiction>,
    pub known_principals: Vec<EntityId>,
    pub foreign_connection: bool,
}

impl Organization {
    pub fn new(name: impl Into<String>, org_type: OrganizationType, sources: SourceChain) -> Self {
        Self {
            meta: EntityMeta::new(sources),
            name: name.into(),
            aliases: Vec::new(),
            org_type,
            jurisdiction: None,
            known_principals: Vec::new(),
            foreign_connection: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrganizationType {
    Pac,
    SuperPac,
    Corporation,
    ForeignEntity,
    Nonprofit,
    GovernmentAgency,
    LawFirm,
    LobbyingFirm,
    Foundation,
    PoliticalParty,
    TradeAssociation,
    Other,
}

/// A document — ingested PDF, filing, or record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub meta: EntityMeta,
    pub title: String,
    pub document_type: DocumentType,
    /// Full text content (indexed in Tantivy)
    pub content: Option<String>,
    /// Original file hash for content-addressable storage
    pub file_hash: String,
    /// Original filename
    pub filename: Option<String>,
    /// MIME type
    pub mime_type: Option<String>,
    /// Page count (for PDFs)
    pub page_count: Option<u32>,
    pub date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentType {
    FecFiling,
    CourtFiling,
    FinancialDisclosure,
    LobbyingReport,
    ExecutiveOrder,
    Legislation,
    InspectorGeneralReport,
    GaoReport,
    Transcript,
    Manifest,
    Other,
}

/// A monetary payment — campaign contribution, expenditure, contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub meta: EntityMeta,
    pub amount: f64,
    pub currency: String,
    pub date: NaiveDate,
    pub donor: EntityId,
    pub recipient: EntityId,
    pub payment_type: PaymentType,
    pub filing_id: Option<String>,
    pub election_cycle: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaymentType {
    IndividualContribution,
    PacContribution,
    IndependentExpenditure,
    GovernmentContract,
    Grant,
    Loan,
    InKind,
    Other,
}

/// A court case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourtCase {
    pub meta: EntityMeta,
    pub case_id: String,
    pub court: String,
    pub case_type: CaseType,
    pub parties: Vec<CaseParty>,
    pub outcome: Option<String>,
    pub filing_date: Option<NaiveDate>,
    pub disposition_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CaseType {
    Criminal,
    Civil,
    Bankruptcy,
    Appellate,
    SupremeCourt,
    Administrative,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseParty {
    pub entity_id: EntityId,
    pub role: CasePartyRole,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CasePartyRole {
    Plaintiff,
    Defendant,
    Witness,
    Judge,
    Attorney,
    Amicus,
    Other,
}

// ─── Custom Schema Extensions ────────────────────────────────────────────────

/// A presidential pardon or commutation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pardon {
    pub meta: EntityMeta,
    pub person_pardoned: EntityId,
    pub pardoning_official: EntityId,
    pub offense: String,
    pub sentence_at_time: Option<String>,
    pub pardon_date: NaiveDate,
    pub indictment_date: Option<NaiveDate>,
    /// Calculated: days between indictment and pardon
    pub days_indictment_to_pardon: Option<i64>,
    /// Whether the pardoned person had concurrent business relationships with the official
    pub concurrent_business_relationship: bool,
}

/// A flight log entry from public manifests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightLogEntry {
    pub meta: EntityMeta,
    pub aircraft_tail_number: String,
    pub date: NaiveDate,
    pub origin: Option<String>,
    pub destination: Option<String>,
    pub passengers: Vec<EntityId>,
}

/// Automatically generated timing correlation between two events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingCorrelation {
    pub meta: EntityMeta,
    pub event_a: EntityId,
    pub event_a_description: String,
    pub event_a_date: NaiveDate,
    pub event_b: EntityId,
    pub event_b_description: String,
    pub event_b_date: NaiveDate,
    /// Days between the two events (always positive)
    pub days_between: u32,
    pub correlation_type: CorrelationType,
    /// Whether this was auto-flagged by the timing engine
    pub auto_flagged: bool,
    /// The threshold that triggered the flag (e.g., 90 days)
    pub threshold_days: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CorrelationType {
    DonationToVote,
    DonationToPolicy,
    IndictmentToPardon,
    GiftToPolicy,
    LobbyingToVote,
    ContractToVote,
    AppointmentToDecision,
    Other,
}

/// Conduct comparison — official action vs equivalent private conduct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductComparison {
    pub meta: EntityMeta,
    pub official_action: String,
    pub official: EntityId,
    pub action_date: NaiveDate,
    pub action_source: String,
    pub equivalent_private_conduct: String,
    pub documented_consequence: String,
    pub consequence_source: String,
}

/// A public statement by an official
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicStatement {
    pub meta: EntityMeta,
    pub official: EntityId,
    pub date: NaiveDate,
    pub platform: StatementPlatform,
    /// Summary only — NOT full text (copyright safety)
    pub content_summary: String,
    pub topic_tags: Vec<String>,
    pub beneficiary_tags: Vec<EntityId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StatementPlatform {
    CongressionalRecord,
    CSpan,
    PressConference,
    HearingTestimony,
    OfficialWebsite,
    SocialMedia,
    Interview,
    Other,
}

/// A policy decision or official action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub meta: EntityMeta,
    pub official: EntityId,
    pub date: NaiveDate,
    pub description: String,
    pub decision_type: PolicyDecisionType,
    pub beneficiaries: Vec<EntityId>,
    /// Legislation or regulation reference
    pub reference_number: Option<String>,
    /// How the official voted (if applicable)
    pub vote: Option<VotePosition>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyDecisionType {
    LegislativeVote,
    ExecutiveOrder,
    RegulatoryAction,
    Appointment,
    ContractAward,
    GrantDecision,
    JudicialRuling,
    Veto,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VotePosition {
    Yea,
    Nay,
    Present,
    NotVoting,
}

/// The top-level enum wrapping all entity types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Entity {
    Person(Person),
    Organization(Organization),
    Document(Document),
    Payment(Payment),
    CourtCase(CourtCase),
    Pardon(Pardon),
    FlightLogEntry(FlightLogEntry),
    TimingCorrelation(TimingCorrelation),
    ConductComparison(ConductComparison),
    PublicStatement(PublicStatement),
    PolicyDecision(PolicyDecision),
}

impl Entity {
    pub fn entity_id(&self) -> EntityId {
        match self {
            Self::Person(e) => e.meta.id,
            Self::Organization(e) => e.meta.id,
            Self::Document(e) => e.meta.id,
            Self::Payment(e) => e.meta.id,
            Self::CourtCase(e) => e.meta.id,
            Self::Pardon(e) => e.meta.id,
            Self::FlightLogEntry(e) => e.meta.id,
            Self::TimingCorrelation(e) => e.meta.id,
            Self::ConductComparison(e) => e.meta.id,
            Self::PublicStatement(e) => e.meta.id,
            Self::PolicyDecision(e) => e.meta.id,
        }
    }

    pub fn sources(&self) -> &SourceChain {
        match self {
            Self::Person(e) => &e.meta.sources,
            Self::Organization(e) => &e.meta.sources,
            Self::Document(e) => &e.meta.sources,
            Self::Payment(e) => &e.meta.sources,
            Self::CourtCase(e) => &e.meta.sources,
            Self::Pardon(e) => &e.meta.sources,
            Self::FlightLogEntry(e) => &e.meta.sources,
            Self::TimingCorrelation(e) => &e.meta.sources,
            Self::ConductComparison(e) => &e.meta.sources,
            Self::PublicStatement(e) => &e.meta.sources,
            Self::PolicyDecision(e) => &e.meta.sources,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Person(_) => "Person",
            Self::Organization(_) => "Organization",
            Self::Document(_) => "Document",
            Self::Payment(_) => "Payment",
            Self::CourtCase(_) => "CourtCase",
            Self::Pardon(_) => "Pardon",
            Self::FlightLogEntry(_) => "FlightLogEntry",
            Self::TimingCorrelation(_) => "TimingCorrelation",
            Self::ConductComparison(_) => "ConductComparison",
            Self::PublicStatement(_) => "PublicStatement",
            Self::PolicyDecision(_) => "PolicyDecision",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{ContentHash, SourceRef, SourceType};
    use url::Url;

    fn test_source_chain() -> SourceChain {
        let url = Url::parse("https://api.open.fec.gov/v1/test/").unwrap();
        let source = SourceRef::new(
            url,
            ContentHash::compute(b"test"),
            SourceType::FecFiling,
            "test",
        );
        SourceChain::new(source)
    }

    #[test]
    fn test_person_requires_source() {
        let person = Person::new("Jane Doe", test_source_chain());
        assert_eq!(person.name, "Jane Doe");
        assert_eq!(person.meta.sources.source_count(), 1);
        assert_eq!(person.meta.version, 1);
    }

    #[test]
    fn test_organization_requires_source() {
        let org = Organization::new("Test PAC", OrganizationType::Pac, test_source_chain());
        assert_eq!(org.name, "Test PAC");
        assert_eq!(org.org_type, OrganizationType::Pac);
    }

    #[test]
    fn test_entity_enum_accessors() {
        let person = Person::new("Test Person", test_source_chain());
        let entity = Entity::Person(person);
        assert_eq!(entity.type_name(), "Person");
        assert_eq!(entity.sources().source_count(), 1);
    }

    #[test]
    fn test_payment_creation() {
        let sources = test_source_chain();
        let donor = EntityId::new();
        let recipient = EntityId::new();
        let payment = Payment {
            meta: EntityMeta::new(sources),
            amount: 500_000.0,
            currency: "USD".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            donor,
            recipient,
            payment_type: PaymentType::IndividualContribution,
            filing_id: Some("FEC-2024-001".to_string()),
            election_cycle: Some("2024".to_string()),
            description: None,
        };
        assert_eq!(payment.amount, 500_000.0);
        assert_eq!(payment.payment_type, PaymentType::IndividualContribution);
    }

    #[test]
    fn test_timing_correlation() {
        let sources = test_source_chain();
        let tc = TimingCorrelation {
            meta: EntityMeta::new(sources),
            event_a: EntityId::new(),
            event_a_description: "Donation of $500K".to_string(),
            event_a_date: NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(),
            event_b: EntityId::new(),
            event_b_description: "Vote against disclosure bill".to_string(),
            event_b_date: NaiveDate::from_ymd_opt(2024, 3, 5).unwrap(),
            days_between: 55,
            correlation_type: CorrelationType::DonationToVote,
            auto_flagged: true,
            threshold_days: Some(90),
        };
        assert!(tc.auto_flagged);
        assert!(tc.days_between < tc.threshold_days.unwrap());
    }

    #[test]
    fn test_conduct_comparison() {
        let sources = test_source_chain();
        let cc = ConductComparison {
            meta: EntityMeta::new(sources),
            official_action: "Received $500K, voted against disclosure bill 47 days later"
                .to_string(),
            official: EntityId::new(),
            action_date: NaiveDate::from_ymd_opt(2024, 3, 5).unwrap(),
            action_source: "FEC + Roll Call".to_string(),
            equivalent_private_conduct: "Employee accepts payment, covers up employer misconduct"
                .to_string(),
            documented_consequence: "Termination + fraud charges".to_string(),
            consequence_source: "Case No. 2023-CV-12345".to_string(),
        };
        assert!(!cc.official_action.is_empty());
        assert!(!cc.consequence_source.is_empty());
    }

    #[test]
    fn test_pardon_days_calculation() {
        let sources = test_source_chain();
        let indictment = NaiveDate::from_ymd_opt(2020, 6, 15).unwrap();
        let pardon_date = NaiveDate::from_ymd_opt(2021, 1, 20).unwrap();
        let days = (pardon_date - indictment).num_days();

        let pardon = Pardon {
            meta: EntityMeta::new(sources),
            person_pardoned: EntityId::new(),
            pardoning_official: EntityId::new(),
            offense: "Wire fraud".to_string(),
            sentence_at_time: Some("10 years".to_string()),
            pardon_date,
            indictment_date: Some(indictment),
            days_indictment_to_pardon: Some(days),
            concurrent_business_relationship: true,
        };
        assert_eq!(pardon.days_indictment_to_pardon, Some(219));
        assert!(pardon.concurrent_business_relationship);
    }
}
