// nf-search: Tantivy full-text search engine replacing Elasticsearch

pub mod error;
pub mod index;
pub mod indexer;
pub mod query;
pub mod searcher;

pub use error::SearchError;
pub use index::{IndexDirectory, NfSchema, open_or_create_index};
pub use indexer::EntityIndexer;
pub use query::QueryBuilder;
pub use searcher::{NfSearcher, SearchOptions, SearchResult};

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::NaiveDate;
    use nf_core::{
        entities::{
            CaseType, ConductComparison, CorrelationType, CourtCase, Document, DocumentType,
            Entity, EntityId, EntityMeta, FlightLogEntry, Organization, OrganizationType, Pardon,
            Payment, PaymentType, Person, PolicyDecision, PolicyDecisionType, PublicStatement,
            StatementPlatform, TimingCorrelation,
        },
        source::{ContentHash, SourceChain, SourceRef, SourceType},
    };
    use url::Url;

    use super::*;

    // ─── Test helpers ─────────────────────────────────────────────────────────

    fn source_chain() -> SourceChain {
        let url = Url::parse("https://example.gov/source/1").unwrap();
        let src = SourceRef::new(
            url,
            ContentHash::compute(b"test"),
            SourceType::FecFiling,
            "test",
        );
        SourceChain::new(src)
    }

    fn ram_index() -> (tantivy::Index, Arc<NfSchema>) {
        let schema = Arc::new(NfSchema::build());
        let index = open_or_create_index(&schema, IndexDirectory::Ram).unwrap();
        (index, schema)
    }

    /// Index entities and commit, then return a searcher.
    fn setup(entities: &[Entity]) -> (tantivy::Index, Arc<NfSchema>) {
        let (index, schema) = ram_index();
        let mut indexer = EntityIndexer::new(&index, schema.clone()).unwrap();
        indexer.index_entities(entities).unwrap();
        indexer.commit().unwrap();
        (index, schema)
    }

    // ─── Entity construction helpers ──────────────────────────────────────────

    fn make_person(name: &str) -> Entity {
        Entity::Person(Person::new(name, source_chain()))
    }

    fn make_org(name: &str) -> Entity {
        Entity::Organization(Organization::new(
            name,
            OrganizationType::Pac,
            source_chain(),
        ))
    }

    fn make_document(title: &str, content: &str, date: NaiveDate) -> Entity {
        Entity::Document(Document {
            meta: EntityMeta::new(source_chain()),
            title: title.to_owned(),
            document_type: DocumentType::FecFiling,
            content: Some(content.to_owned()),
            file_hash: "abc123".to_owned(),
            filename: None,
            mime_type: None,
            page_count: None,
            date: Some(date),
        })
    }

    fn make_payment(amount: f64, date: NaiveDate, desc: &str) -> Entity {
        Entity::Payment(Payment {
            meta: EntityMeta::new(source_chain()),
            amount,
            currency: "USD".to_owned(),
            date,
            donor: EntityId::new(),
            recipient: EntityId::new(),
            payment_type: PaymentType::IndividualContribution,
            filing_id: None,
            election_cycle: None,
            description: Some(desc.to_owned()),
        })
    }

    fn make_court_case(case_id: &str, court: &str, date: NaiveDate) -> Entity {
        Entity::CourtCase(CourtCase {
            meta: EntityMeta::new(source_chain()),
            case_id: case_id.to_owned(),
            court: court.to_owned(),
            case_type: CaseType::Civil,
            parties: vec![],
            outcome: Some("settled".to_owned()),
            filing_date: Some(date),
            disposition_date: None,
        })
    }

    fn make_pardon(offense: &str, date: NaiveDate) -> Entity {
        Entity::Pardon(Pardon {
            meta: EntityMeta::new(source_chain()),
            person_pardoned: EntityId::new(),
            pardoning_official: EntityId::new(),
            offense: offense.to_owned(),
            sentence_at_time: None,
            pardon_date: date,
            indictment_date: None,
            days_indictment_to_pardon: None,
            concurrent_business_relationship: false,
        })
    }

    fn make_flight(tail: &str, date: NaiveDate) -> Entity {
        Entity::FlightLogEntry(FlightLogEntry {
            meta: EntityMeta::new(source_chain()),
            aircraft_tail_number: tail.to_owned(),
            date,
            origin: Some("DCA".to_owned()),
            destination: Some("LGA".to_owned()),
            passengers: vec![],
        })
    }

    fn make_timing_correlation(desc_a: &str, desc_b: &str, date: NaiveDate) -> Entity {
        Entity::TimingCorrelation(TimingCorrelation {
            meta: EntityMeta::new(source_chain()),
            event_a: EntityId::new(),
            event_a_description: desc_a.to_owned(),
            event_a_date: date,
            event_b: EntityId::new(),
            event_b_description: desc_b.to_owned(),
            event_b_date: date,
            days_between: 30,
            correlation_type: CorrelationType::DonationToVote,
            auto_flagged: true,
            threshold_days: Some(90),
        })
    }

    fn make_conduct_comparison(action: &str, date: NaiveDate) -> Entity {
        Entity::ConductComparison(ConductComparison {
            meta: EntityMeta::new(source_chain()),
            official_action: action.to_owned(),
            official: EntityId::new(),
            action_date: date,
            action_source: "FEC".to_owned(),
            equivalent_private_conduct: "Bribery".to_owned(),
            documented_consequence: "Termination".to_owned(),
            consequence_source: "Court record".to_owned(),
        })
    }

    fn make_public_statement(summary: &str, date: NaiveDate) -> Entity {
        Entity::PublicStatement(PublicStatement {
            meta: EntityMeta::new(source_chain()),
            official: EntityId::new(),
            date,
            platform: StatementPlatform::CongressionalRecord,
            content_summary: summary.to_owned(),
            topic_tags: vec!["taxation".to_owned()],
            beneficiary_tags: vec![],
        })
    }

    fn make_policy_decision(desc: &str, date: NaiveDate) -> Entity {
        Entity::PolicyDecision(PolicyDecision {
            meta: EntityMeta::new(source_chain()),
            official: EntityId::new(),
            date,
            description: desc.to_owned(),
            decision_type: PolicyDecisionType::LegislativeVote,
            beneficiaries: vec![],
            reference_number: Some("HR-1234".to_owned()),
            vote: None,
        })
    }

    // ─── Index / schema tests ─────────────────────────────────────────────────

    #[test]
    fn test_schema_has_all_fields() {
        let s = NfSchema::build();
        // Just verifying fields are distinct (Field is Copy).
        assert_ne!(s.entity_id, s.entity_type);
        assert_ne!(s.name, s.content);
        assert_ne!(s.tags, s.date);
    }

    #[test]
    fn test_open_or_create_ram() {
        let schema = NfSchema::build();
        let index = open_or_create_index(&schema, IndexDirectory::Ram);
        assert!(index.is_ok());
    }

    #[test]
    fn test_open_or_create_mmap() {
        let dir = tempfile::tempdir().unwrap();
        let schema = NfSchema::build();
        let index = open_or_create_index(&schema, IndexDirectory::Mmap(dir.path().to_path_buf()));
        assert!(index.is_ok());
        // Opening again (already exists) should also work.
        let schema2 = NfSchema::build();
        let index2 = open_or_create_index(&schema2, IndexDirectory::Mmap(dir.path().to_path_buf()));
        assert!(index2.is_ok());
    }

    // ─── Indexer tests ────────────────────────────────────────────────────────

    #[test]
    fn test_index_all_entity_types() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let entities = vec![
            make_person("Alice Doe"),
            make_org("Super PAC Alpha"),
            make_document("FEC Report 2024", "campaign finance disclosure", date),
            make_payment(500_000.0, date, "direct contribution"),
            make_court_case("2024-CV-001", "D.C. Circuit", date),
            make_pardon("Wire fraud conviction", date),
            make_flight("N12345", date),
            make_timing_correlation("Donation received", "Vote against disclosure", date),
            make_conduct_comparison("Accepted $500K then voted Nay", date),
            make_public_statement("Voted to cut corporate taxes", date),
            make_policy_decision("Voted against H.R.1234 transparency bill", date),
        ];
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema.clone()).unwrap();
        // Each entity type should be individually retrievable.
        let opts = SearchOptions::new()
            .with_limit(5)
            .with_entity_type("Person");
        let r = searcher.search("Alice", &opts).unwrap();
        assert_eq!(r.len(), 1);
        let opts = SearchOptions::new()
            .with_limit(5)
            .with_entity_type("Organization");
        let r = searcher.search("Alpha", &opts).unwrap();
        assert_eq!(r.len(), 1);
        let opts = SearchOptions::new()
            .with_limit(5)
            .with_entity_type("Document");
        let r = searcher.search("campaign", &opts).unwrap();
        assert_eq!(r.len(), 1);
        let opts = SearchOptions::new()
            .with_limit(5)
            .with_entity_type("Payment");
        let r = searcher.search("contribution", &opts).unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn test_index_person_searchable_by_name() {
        let entities = vec![make_person("Jane Smith")];
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        let results = searcher
            .search("Jane", &SearchOptions::new().with_limit(10))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_type, "Person");
        assert!(results[0].name.contains("Jane Smith"));
    }

    #[test]
    fn test_index_organization_searchable() {
        let entities = vec![make_org("Influence Capital PAC")];
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        let results = searcher
            .search("Influence", &SearchOptions::new().with_limit(10))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_type, "Organization");
    }

    #[test]
    fn test_index_document_content_searchable() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let entities = vec![make_document(
            "FEC Filing Q1",
            "lobbying expenditure PAC disclosure",
            date,
        )];
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        let results = searcher
            .search("lobbying", &SearchOptions::new().with_limit(10))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_type, "Document");
    }

    #[test]
    fn test_incremental_update_replaces_old_doc() {
        let (index, schema) = ram_index();
        let mut indexer = EntityIndexer::new(&index, schema.clone()).unwrap();

        let mut person = Person::new("Bob Jones", source_chain());
        let _entity_id = person.meta.id;
        indexer
            .index_entity(&Entity::Person(person.clone()))
            .unwrap();
        indexer.commit().unwrap();

        // Update the name and re-index.
        person.name = "Bob Jones Jr.".to_owned();
        indexer.index_entity(&Entity::Person(person)).unwrap();
        indexer.commit().unwrap();

        let searcher = NfSearcher::new(&index, schema).unwrap();
        let results = searcher
            .search("Bob", &SearchOptions::new().with_limit(10))
            .unwrap();

        // Only one document should exist.
        assert_eq!(results.len(), 1);
        assert!(results[0].name.contains("Jr."));
    }

    #[test]
    fn test_delete_entity() {
        let (index, schema) = ram_index();
        let mut indexer = EntityIndexer::new(&index, schema.clone()).unwrap();

        let entity = make_person("Carol White");
        let id = entity.entity_id();
        indexer.index_entity(&entity).unwrap();
        indexer.commit().unwrap();

        indexer.delete_entity(&id).unwrap();
        indexer.commit().unwrap();

        let searcher = NfSearcher::new(&index, schema).unwrap();
        let results = searcher
            .search("Carol", &SearchOptions::new().with_limit(10))
            .unwrap();
        assert!(results.is_empty());
    }

    // ─── Search tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_search_returns_entity_id_and_type() {
        let entities = vec![make_person("David Lee")];
        let entity_uuid = entities[0].entity_id().0.to_string();
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        let results = searcher
            .search("David", &SearchOptions::new().with_limit(10))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, entity_uuid);
        assert_eq!(results[0].entity_type, "Person");
    }

    #[test]
    fn test_search_no_results_for_unknown_term() {
        let entities = vec![make_person("Eve Brown")];
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        let results = searcher
            .search("xyzzy", &SearchOptions::new().with_limit(10))
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_scores_are_positive() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let entities = vec![
            make_document("Donation Report", "large corporate donation", date),
            make_payment(100.0, date, "small personal donation"),
        ];
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        let results = searcher
            .search("donation", &SearchOptions::new().with_limit(10))
            .unwrap();
        assert!(!results.is_empty());
        for r in &results {
            assert!(r.score > 0.0, "score should be positive");
        }
    }

    // ─── Faceted search tests ─────────────────────────────────────────────────

    #[test]
    fn test_facet_by_type() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let entities = vec![
            make_person("Frank Green"),
            make_org("Green Energy PAC"),
            make_document("Green Report", "environmental policy green", date),
        ];
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        let facets = searcher.facet_by_type("green").unwrap();
        let type_names: Vec<&str> = facets.iter().map(|(t, _)| t.as_str()).collect();
        assert!(type_names.contains(&"Person"));
        assert!(type_names.contains(&"Organization"));
        assert!(type_names.contains(&"Document"));
    }

    #[test]
    fn test_facet_counts_correct() {
        let entities = vec![
            make_person("George Black"),
            make_person("Harry Black"),
            make_org("Black Rock PAC"),
        ];
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        let facets = searcher.facet_by_type("black").unwrap();
        let person_count = facets
            .iter()
            .find(|(t, _)| t == "Person")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        let org_count = facets
            .iter()
            .find(|(t, _)| t == "Organization")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        assert_eq!(person_count, 2);
        assert_eq!(org_count, 1);
    }

    // ─── Pagination tests ─────────────────────────────────────────────────────

    #[test]
    fn test_pagination_limit() {
        let entities: Vec<Entity> = (0..10)
            .map(|i| make_person(&format!("Person {i}")))
            .collect();
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        let results = searcher
            .search("Person", &SearchOptions::new().with_limit(3))
            .unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_pagination_offset() {
        let entities: Vec<Entity> = (0..10)
            .map(|i| make_person(&format!("Person {i}")))
            .collect();
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        let page1 = searcher
            .search("Person", &SearchOptions::new().with_limit(5))
            .unwrap();
        let page2 = searcher
            .search("Person", &SearchOptions::new().with_limit(5).with_offset(5))
            .unwrap();

        assert_eq!(page1.len(), 5);
        assert_eq!(page2.len(), 5);

        let ids1: std::collections::HashSet<_> = page1.iter().map(|r| &r.entity_id).collect();
        let ids2: std::collections::HashSet<_> = page2.iter().map(|r| &r.entity_id).collect();
        // Pages should not overlap.
        assert!(ids1.is_disjoint(&ids2));
    }

    // ─── Entity-type filter tests ─────────────────────────────────────────────

    #[test]
    fn test_filter_by_entity_type() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let entities = vec![
            make_person("Ivan Gray"),
            make_org("Gray Matter LLC"),
            make_document("Gray Report", "gray area regulation", date),
        ];
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        let opts = SearchOptions::new()
            .with_limit(10)
            .with_entity_type("Person");
        let results = searcher.search("gray", &opts).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_type, "Person");
    }

    // ─── Date range filter tests ──────────────────────────────────────────────

    #[test]
    fn test_date_range_filter() {
        let early = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let late = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let entities = vec![
            make_document("Old Filing", "old budget appropriation", early),
            make_document("New Filing", "new budget appropriation", late),
        ];
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        // Only documents from 2023 onwards.
        let cutoff = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        use chrono::TimeZone;
        let cutoff_ts = chrono::Utc
            .from_utc_datetime(&cutoff.and_hms_opt(0, 0, 0).unwrap())
            .timestamp();
        let opts = SearchOptions::new()
            .with_limit(10)
            .with_date_range(Some(cutoff_ts), None);

        let results = searcher.search("budget", &opts).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].name.contains("New"));
    }

    // ─── QueryBuilder tests ───────────────────────────────────────────────────

    #[test]
    fn test_query_builder_by_type_and_text() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let entities = vec![
            make_person("Julia White"),
            make_document("White Paper", "policy white paper analysis", date),
        ];
        let (index, schema) = setup(&entities);

        let query =
            QueryBuilder::search_by_type_and_text(&schema, &index, "Person", "white").unwrap();

        let searcher_obj = index.reader().unwrap().searcher();
        let top_docs = searcher_obj
            .search(&*query, &tantivy::collector::TopDocs::with_limit(10))
            .unwrap();
        assert_eq!(top_docs.len(), 1);

        let doc: tantivy::TantivyDocument = searcher_obj.doc(top_docs[0].1).unwrap();
        let entity_type_val = doc
            .get_first(schema.entity_type)
            .and_then(|v| {
                if let tantivy::schema::OwnedValue::Str(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        assert_eq!(entity_type_val, "Person");
    }

    #[test]
    fn test_query_builder_by_type_and_date() {
        let early = NaiveDate::from_ymd_opt(2018, 1, 1).unwrap();
        let mid = NaiveDate::from_ymd_opt(2022, 6, 1).unwrap();
        let late = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let entities = vec![
            make_payment(100.0, early, "early contribution"),
            make_payment(200.0, mid, "mid contribution"),
            make_payment(300.0, late, "late contribution"),
        ];
        let (index, schema) = setup(&entities);

        let from = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        let query =
            QueryBuilder::search_by_type_and_date(&schema, &index, "Payment", Some(from), Some(to));

        let searcher_obj = index.reader().unwrap().searcher();
        let top_docs = searcher_obj
            .search(&*query, &tantivy::collector::TopDocs::with_limit(10))
            .unwrap();
        assert_eq!(top_docs.len(), 1);
    }

    #[test]
    fn test_query_builder_entity_id_lookup() {
        let entities = vec![make_person("Kevin Orange")];
        let target_id = entities[0].entity_id().0.to_string();
        let (index, schema) = setup(&entities);

        let query = QueryBuilder::new(&schema, &index)
            .with_entity_id(&target_id)
            .build();

        let searcher_obj = index.reader().unwrap().searcher();
        let top_docs = searcher_obj
            .search(&*query, &tantivy::collector::TopDocs::with_limit(10))
            .unwrap();
        assert_eq!(top_docs.len(), 1);
    }

    #[test]
    fn test_query_builder_all_query_when_empty() {
        let entities = vec![make_person("Laura Blue"), make_org("Blue Wave PAC")];
        let (index, schema) = setup(&entities);

        let query = QueryBuilder::new(&schema, &index).build();
        let searcher_obj = index.reader().unwrap().searcher();
        let top_docs = searcher_obj
            .search(&*query, &tantivy::collector::TopDocs::with_limit(100))
            .unwrap();
        assert_eq!(top_docs.len(), 2);
    }

    // ─── Source URL storage tests ─────────────────────────────────────────────

    #[test]
    fn test_source_urls_stored_in_result() {
        let entities = vec![make_person("Mike Teal")];
        let (index, schema) = setup(&entities);
        let searcher = NfSearcher::new(&index, schema).unwrap();

        let results = searcher
            .search("Mike", &SearchOptions::new().with_limit(10))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].source_urls.is_empty());
        assert!(results[0].source_urls[0].starts_with("https://"));
    }

    // ─── Tag indexing tests ───────────────────────────────────────────────────

    #[test]
    fn test_tags_indexed_and_searchable() {
        let (index, schema) = ram_index();
        let mut indexer = EntityIndexer::new(&index, schema.clone()).unwrap();

        let mut person = Person::new("Nancy Gold", source_chain());
        person.meta.tags = vec!["senator".to_owned(), "finance-committee".to_owned()];
        indexer.index_entity(&Entity::Person(person)).unwrap();
        indexer.commit().unwrap();

        let searcher = NfSearcher::new(&index, schema).unwrap();
        let results = searcher
            .search("senator", &SearchOptions::new().with_limit(10))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].tags.contains(&"senator".to_owned()));
    }
}
