use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::EntityId;
use crate::source::SourceChain;

/// Every relationship (edge) between entities requires a source chain.
/// No source = edge cannot exist.

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RelationshipId(pub Uuid);

impl RelationshipId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// A typed, sourced relationship between two entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: RelationshipId,
    pub from: EntityId,
    pub to: EntityId,
    pub rel_type: RelationshipType,
    pub sources: SourceChain,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub properties: RelationshipProperties,
    pub version: u64,
}

impl Relationship {
    pub fn new(
        from: EntityId,
        to: EntityId,
        rel_type: RelationshipType,
        sources: SourceChain,
    ) -> Self {
        Self {
            id: RelationshipId::new(),
            from,
            to,
            rel_type,
            sources,
            start_date: None,
            end_date: None,
            properties: RelationshipProperties::default(),
            version: 1,
        }
    }

    pub fn with_dates(mut self, start: NaiveDate, end: Option<NaiveDate>) -> Self {
        self.start_date = Some(start);
        self.end_date = end;
        self
    }

    /// Whether this relationship was active on a given date
    pub fn active_on(&self, date: NaiveDate) -> bool {
        let after_start = self.start_date.map_or(true, |s| date >= s);
        let before_end = self.end_date.map_or(true, |e| date <= e);
        after_start && before_end
    }
}

/// Enumeration of all relationship types in the system
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RelationshipType {
    /// Person/Org donated to Person (campaign contribution)
    DonatedTo,
    /// Person pardoned Person
    Pardoned,
    /// Person appeared with Person/at Location
    AppearedWith,
    /// Person made a policy decision benefiting Entity
    PolicyDecision,
    /// Person holds position at Organization
    HoldsPosition,
    /// Person named in CourtCase
    NamedIn,
    /// Person flew with Person
    FlightWith,
    /// Person has business relationship with Organization
    BusinessWith,
    /// Person lobbied for Organization/Person
    LobbiedFor,
    /// Organization registered as foreign agent for Entity
    ForeignAgentFor,
    /// Person appointed Person to position
    Appointed,
    /// Person is family member of Person
    FamilyOf,
    /// Organization is subsidiary/parent of Organization
    CorporateStructure,
    /// Person owns/controls Organization
    Owns,
    /// Person/Org received government contract
    ReceivedContract,
    /// Person made public statement about Entity
    StatedAbout,
    /// Organization/Person was lobbied by a registrant on behalf of a client
    LobbiedBy,
}

/// Additional typed properties depending on relationship type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelationshipProperties {
    /// For DonatedTo: amount in USD
    pub amount: Option<f64>,
    /// For DonatedTo: FEC filing ID
    pub filing_id: Option<String>,
    /// For DonatedTo: election cycle
    pub election_cycle: Option<String>,
    /// For HoldsPosition: role title
    pub role: Option<String>,
    /// For NamedIn: party role (plaintiff, defendant, etc.)
    pub case_role: Option<String>,
    /// For FlightWith: aircraft tail number
    pub aircraft: Option<String>,
    /// For FlightWith: manifest source URL
    pub manifest_source: Option<String>,
    /// For Pardoned: offense
    pub offense: Option<String>,
    /// For BusinessWith: concurrent with pardon?
    pub concurrent_with_pardon: Option<bool>,
    /// For PolicyDecision: days after relevant donation
    pub days_after_donation: Option<i64>,
    /// For ForeignAgentFor: FARA registration number
    pub fara_registration: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{ContentHash, SourceChain, SourceRef, SourceType};
    use url::Url;

    fn test_chain() -> SourceChain {
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
    fn test_relationship_creation() {
        let from = EntityId::new();
        let to = EntityId::new();
        let rel = Relationship::new(from, to, RelationshipType::DonatedTo, test_chain());
        assert_eq!(rel.from, from);
        assert_eq!(rel.to, to);
        assert_eq!(rel.rel_type, RelationshipType::DonatedTo);
    }

    #[test]
    fn test_relationship_active_on() {
        let from = EntityId::new();
        let to = EntityId::new();
        let start = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

        let rel = Relationship::new(from, to, RelationshipType::HoldsPosition, test_chain())
            .with_dates(start, Some(end));

        assert!(rel.active_on(NaiveDate::from_ymd_opt(2022, 6, 15).unwrap()));
        assert!(!rel.active_on(NaiveDate::from_ymd_opt(2019, 12, 31).unwrap()));
        assert!(!rel.active_on(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()));
    }

    #[test]
    fn test_relationship_no_end_date_still_active() {
        let from = EntityId::new();
        let to = EntityId::new();
        let start = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();

        let rel = Relationship::new(from, to, RelationshipType::HoldsPosition, test_chain())
            .with_dates(start, None);

        assert!(rel.active_on(NaiveDate::from_ymd_opt(2030, 1, 1).unwrap()));
    }

    #[test]
    fn test_relationship_new_initializes_accessors_and_defaults() {
        let from = EntityId::new();
        let to = EntityId::new();
        let rel = Relationship::new(from, to, RelationshipType::BusinessWith, test_chain());

        assert_eq!(rel.from, from);
        assert_eq!(rel.to, to);
        assert_eq!(rel.rel_type, RelationshipType::BusinessWith);
        assert_eq!(rel.sources.source_count(), 1);
        assert_eq!(rel.version, 1);
        assert!(rel.start_date.is_none());
        assert!(rel.end_date.is_none());
        assert!(rel.properties.amount.is_none());
        assert!(rel.properties.role.is_none());
    }

    #[test]
    fn test_relationship_all_types_constructible() {
        let from = EntityId::new();
        let to = EntityId::new();
        let all = [
            RelationshipType::DonatedTo,
            RelationshipType::Pardoned,
            RelationshipType::AppearedWith,
            RelationshipType::PolicyDecision,
            RelationshipType::HoldsPosition,
            RelationshipType::NamedIn,
            RelationshipType::FlightWith,
            RelationshipType::BusinessWith,
            RelationshipType::LobbiedFor,
            RelationshipType::ForeignAgentFor,
            RelationshipType::Appointed,
            RelationshipType::FamilyOf,
            RelationshipType::CorporateStructure,
            RelationshipType::Owns,
            RelationshipType::ReceivedContract,
            RelationshipType::StatedAbout,
            RelationshipType::LobbiedBy,
        ];

        for rel_type in all {
            let rel = Relationship::new(from, to, rel_type, test_chain());
            assert_eq!(rel.rel_type, rel_type);
            assert_eq!(rel.from, from);
            assert_eq!(rel.to, to);
        }
    }

    #[test]
    fn test_relationship_active_on_date_boundaries() {
        let from = EntityId::new();
        let to = EntityId::new();
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let rel = Relationship::new(from, to, RelationshipType::HoldsPosition, test_chain())
            .with_dates(start, Some(end));

        assert!(rel.active_on(start));
        assert!(rel.active_on(end));
    }

    #[test]
    fn test_relationship_with_dates_sets_fields() {
        let from = EntityId::new();
        let to = EntityId::new();
        let start = NaiveDate::from_ymd_opt(2023, 6, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let rel = Relationship::new(from, to, RelationshipType::NamedIn, test_chain())
            .with_dates(start, Some(end));

        assert_eq!(rel.start_date, Some(start));
        assert_eq!(rel.end_date, Some(end));
    }

    #[test]
    fn test_lobbied_by_relationship_creation() {
        let from = EntityId::new();
        let to = EntityId::new();
        let rel = Relationship::new(from, to, RelationshipType::LobbiedBy, test_chain());
        assert_eq!(rel.rel_type, RelationshipType::LobbiedBy);
        assert_eq!(rel.from, from);
        assert_eq!(rel.to, to);
        assert_eq!(rel.version, 1);
        assert_eq!(rel.sources.source_count(), 1);
    }

    #[test]
    fn test_lobbied_by_serialization_roundtrip() {
        let from = EntityId::new();
        let to = EntityId::new();
        let rel = Relationship::new(from, to, RelationshipType::LobbiedBy, test_chain());
        let json = serde_json::to_string(&rel).expect("serialize should succeed");
        assert!(
            json.contains("LobbiedBy"),
            "serialized JSON should contain 'LobbiedBy'"
        );
        let deserialized: Relationship =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deserialized.rel_type, RelationshipType::LobbiedBy);
        assert_eq!(deserialized.from, from);
        assert_eq!(deserialized.to, to);
    }

    #[test]
    fn test_lobbied_by_with_dates() {
        let from = EntityId::new();
        let to = EntityId::new();
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let rel = Relationship::new(from, to, RelationshipType::LobbiedBy, test_chain())
            .with_dates(start, Some(end));

        assert!(rel.active_on(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()));
        assert!(!rel.active_on(NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()));
        assert!(!rel.active_on(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()));
        assert_eq!(rel.start_date, Some(start));
        assert_eq!(rel.end_date, Some(end));
    }
}
