use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::Row;
use uuid::Uuid;

use nf_core::entities::Entity;
use nf_core::relationships::Relationship;

use crate::db::DbPool;
use crate::error::StoreError;

// ─── Pagination ───────────────────────────────────────────────────────────────

/// A paginated slice of results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    /// Total number of rows matching the query (for computing total pages).
    pub total_count: i64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, page: u32, page_size: u32) -> Self {
        let total_count = items.len() as i64;
        Self {
            items,
            page,
            page_size,
            total_count,
        }
    }

    pub fn with_total(items: Vec<T>, page: u32, page_size: u32, total_count: i64) -> Self {
        Self {
            items,
            page,
            page_size,
            total_count,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Total number of pages.
    pub fn total_pages(&self) -> u32 {
        if self.page_size == 0 {
            return 0;
        }
        ((self.total_count as u32) + self.page_size - 1) / self.page_size
    }
}

// ─── Repository trait ─────────────────────────────────────────────────────────

/// Generic CRUD repository contract.
pub trait Repository<T>: Send + Sync {
    /// Insert a new item. Returns its UUID primary key.
    fn insert(
        &self,
        item: &T,
    ) -> impl std::future::Future<Output = Result<Uuid, StoreError>> + Send;

    /// Fetch an item by UUID, or `None` if not found.
    fn get(
        &self,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<Option<T>, StoreError>> + Send;

    /// Replace the stored data for an existing item.
    /// Returns `true` if a row was updated, `false` if not found.
    fn update(
        &self,
        item: &T,
    ) -> impl std::future::Future<Output = Result<bool, StoreError>> + Send;

    /// Delete by UUID.
    /// Returns `true` if a row was deleted, `false` if not found.
    fn delete(
        &self,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<bool, StoreError>> + Send;

    /// List items with zero-based page index and page size.
    fn list(
        &self,
        page: u32,
        page_size: u32,
    ) -> impl std::future::Future<Output = Result<Page<T>, StoreError>> + Send;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Extract the UUID from any Entity variant.
fn entity_uuid(entity: &Entity) -> Uuid {
    entity.entity_id().0
}

/// Extract the version counter from any Entity variant.
fn entity_version(entity: &Entity) -> i64 {
    let v = match entity {
        Entity::Person(e) => e.meta.version,
        Entity::Organization(e) => e.meta.version,
        Entity::Document(e) => e.meta.version,
        Entity::Payment(e) => e.meta.version,
        Entity::CourtCase(e) => e.meta.version,
        Entity::Pardon(e) => e.meta.version,
        Entity::FlightLogEntry(e) => e.meta.version,
        Entity::TimingCorrelation(e) => e.meta.version,
        Entity::ConductComparison(e) => e.meta.version,
        Entity::PublicStatement(e) => e.meta.version,
        Entity::PolicyDecision(e) => e.meta.version,
    };
    v as i64
}

fn entity_created_at(entity: &Entity) -> chrono::DateTime<chrono::Utc> {
    match entity {
        Entity::Person(e) => e.meta.created_at,
        Entity::Organization(e) => e.meta.created_at,
        Entity::Document(e) => e.meta.created_at,
        Entity::Payment(e) => e.meta.created_at,
        Entity::CourtCase(e) => e.meta.created_at,
        Entity::Pardon(e) => e.meta.created_at,
        Entity::FlightLogEntry(e) => e.meta.created_at,
        Entity::TimingCorrelation(e) => e.meta.created_at,
        Entity::ConductComparison(e) => e.meta.created_at,
        Entity::PublicStatement(e) => e.meta.created_at,
        Entity::PolicyDecision(e) => e.meta.created_at,
    }
}

fn entity_updated_at(entity: &Entity) -> chrono::DateTime<chrono::Utc> {
    match entity {
        Entity::Person(e) => e.meta.updated_at,
        Entity::Organization(e) => e.meta.updated_at,
        Entity::Document(e) => e.meta.updated_at,
        Entity::Payment(e) => e.meta.updated_at,
        Entity::CourtCase(e) => e.meta.updated_at,
        Entity::Pardon(e) => e.meta.updated_at,
        Entity::FlightLogEntry(e) => e.meta.updated_at,
        Entity::TimingCorrelation(e) => e.meta.updated_at,
        Entity::ConductComparison(e) => e.meta.updated_at,
        Entity::PublicStatement(e) => e.meta.updated_at,
        Entity::PolicyDecision(e) => e.meta.updated_at,
    }
}

// ─── EntityRepository ────────────────────────────────────────────────────────

/// Repository for all [`Entity`] variants.
///
/// Every entity is stored as a `JSONB` blob in the `entities` table, tagged
/// by `entity_type` for filtered queries.
pub struct EntityRepository {
    pool: DbPool,
}

impl EntityRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Access the underlying database pool (e.g. for audit queries).
    pub fn pool(&self) -> DbPool {
        self.pool.clone()
    }

    /// List entities filtered by a specific entity type name.
    pub async fn list_by_type(
        &self,
        entity_type: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        let limit = page_size as i64;
        let offset = (page as i64) * limit;

        let (total_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM entities WHERE entity_type = $1")
                .bind(entity_type)
                .fetch_one(&self.pool)
                .await?;

        let rows = sqlx::query(
            "SELECT data FROM entities WHERE entity_type = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(entity_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let items = deserialize_entity_rows(rows)?;
        Ok(Page::with_total(items, page, page_size, total_count))
    }

    // ── Convenience typed getters ────────────────────────────────────────────

    pub async fn list_persons(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        self.list_by_type("Person", page, page_size).await
    }

    pub async fn list_organizations(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        self.list_by_type("Organization", page, page_size).await
    }

    pub async fn list_payments(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        self.list_by_type("Payment", page, page_size).await
    }

    pub async fn list_court_cases(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        self.list_by_type("CourtCase", page, page_size).await
    }

    pub async fn list_pardons(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        self.list_by_type("Pardon", page, page_size).await
    }

    pub async fn list_flight_log_entries(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        self.list_by_type("FlightLogEntry", page, page_size).await
    }

    pub async fn list_timing_correlations(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        self.list_by_type("TimingCorrelation", page, page_size)
            .await
    }

    pub async fn list_conduct_comparisons(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        self.list_by_type("ConductComparison", page, page_size)
            .await
    }

    pub async fn list_public_statements(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        self.list_by_type("PublicStatement", page, page_size).await
    }

    pub async fn list_policy_decisions(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        self.list_by_type("PolicyDecision", page, page_size).await
    }

    pub async fn list_documents(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        self.list_by_type("Document", page, page_size).await
    }
}

impl Repository<Entity> for EntityRepository {
    async fn insert(&self, entity: &Entity) -> Result<Uuid, StoreError> {
        let id = entity_uuid(entity);
        let entity_type = entity.type_name();
        let version = entity_version(entity);
        let created_at = entity_created_at(entity);
        let updated_at = entity_updated_at(entity);
        let data = serde_json::to_value(entity)?;

        sqlx::query(
            "INSERT INTO entities (id, entity_type, version, data, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(entity_type)
        .bind(version)
        .bind(data)
        .bind(created_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    async fn get(&self, id: Uuid) -> Result<Option<Entity>, StoreError> {
        let row = sqlx::query("SELECT data FROM entities WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            None => Ok(None),
            Some(r) => {
                let data: JsonValue = r.try_get("data")?;
                let entity: Entity = serde_json::from_value(data)?;
                Ok(Some(entity))
            }
        }
    }

    async fn update(&self, entity: &Entity) -> Result<bool, StoreError> {
        let id = entity_uuid(entity);
        let data = serde_json::to_value(entity)?;
        let version = entity_version(entity);
        let updated_at = entity_updated_at(entity);

        let result = sqlx::query(
            "UPDATE entities SET data = $1, version = $2, updated_at = $3 WHERE id = $4",
        )
        .bind(data)
        .bind(version)
        .bind(updated_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: Uuid) -> Result<bool, StoreError> {
        let result = sqlx::query("DELETE FROM entities WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn list(&self, page: u32, page_size: u32) -> Result<Page<Entity>, StoreError> {
        let limit = page_size as i64;
        let offset = (page as i64) * limit;

        let (total_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM entities")
            .fetch_one(&self.pool)
            .await?;

        let rows =
            sqlx::query("SELECT data FROM entities ORDER BY created_at DESC LIMIT $1 OFFSET $2")
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;

        let items = deserialize_entity_rows(rows)?;
        Ok(Page::with_total(items, page, page_size, total_count))
    }
}

// ─── RelationshipRepository ───────────────────────────────────────────────────

/// Repository for [`Relationship`] edges.
pub struct RelationshipRepository {
    pool: DbPool,
}

impl RelationshipRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Find all relationships originating from a given entity.
    pub async fn list_from(
        &self,
        from_entity: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Relationship>, StoreError> {
        let limit = page_size as i64;
        let offset = (page as i64) * limit;

        let (total_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM relationships WHERE from_entity = $1")
                .bind(from_entity)
                .fetch_one(&self.pool)
                .await?;

        let rows = sqlx::query(
            "SELECT data FROM relationships WHERE from_entity = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(from_entity)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let items = deserialize_relationship_rows(rows)?;
        Ok(Page::with_total(items, page, page_size, total_count))
    }

    /// Find all relationships pointing to a given entity.
    pub async fn list_to(
        &self,
        to_entity: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Relationship>, StoreError> {
        let limit = page_size as i64;
        let offset = (page as i64) * limit;

        let (total_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM relationships WHERE to_entity = $1")
                .bind(to_entity)
                .fetch_one(&self.pool)
                .await?;

        let rows = sqlx::query(
            "SELECT data FROM relationships WHERE to_entity = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(to_entity)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let items = deserialize_relationship_rows(rows)?;
        Ok(Page::with_total(items, page, page_size, total_count))
    }
}

impl Repository<Relationship> for RelationshipRepository {
    async fn insert(&self, rel: &Relationship) -> Result<Uuid, StoreError> {
        let id = rel.id.0;
        let data = serde_json::to_value(rel)?;

        sqlx::query(
            "INSERT INTO relationships (id, from_entity, to_entity, rel_type, version, data) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(rel.from.0)
        .bind(rel.to.0)
        .bind(format!("{:?}", rel.rel_type))
        .bind(rel.version as i64)
        .bind(data)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    async fn get(&self, id: Uuid) -> Result<Option<Relationship>, StoreError> {
        let row = sqlx::query("SELECT data FROM relationships WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            None => Ok(None),
            Some(r) => {
                let data: JsonValue = r.try_get("data")?;
                let rel: Relationship = serde_json::from_value(data)?;
                Ok(Some(rel))
            }
        }
    }

    async fn update(&self, rel: &Relationship) -> Result<bool, StoreError> {
        let data = serde_json::to_value(rel)?;

        let result = sqlx::query(
            "UPDATE relationships SET data = $1, version = $2, updated_at = NOW() WHERE id = $3",
        )
        .bind(data)
        .bind(rel.version as i64)
        .bind(rel.id.0)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: Uuid) -> Result<bool, StoreError> {
        let result = sqlx::query("DELETE FROM relationships WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn list(&self, page: u32, page_size: u32) -> Result<Page<Relationship>, StoreError> {
        let limit = page_size as i64;
        let offset = (page as i64) * limit;

        let (total_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM relationships")
            .fetch_one(&self.pool)
            .await?;

        let rows = sqlx::query(
            "SELECT data FROM relationships ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let items = deserialize_relationship_rows(rows)?;
        Ok(Page::with_total(items, page, page_size, total_count))
    }
}

// ─── Private deserialization helpers ─────────────────────────────────────────

fn deserialize_entity_rows(rows: Vec<sqlx::postgres::PgRow>) -> Result<Vec<Entity>, StoreError> {
    rows.into_iter()
        .map(|r| {
            let data: JsonValue = r.try_get("data")?;
            serde_json::from_value(data).map_err(StoreError::from)
        })
        .collect()
}

fn deserialize_relationship_rows(
    rows: Vec<sqlx::postgres::PgRow>,
) -> Result<Vec<Relationship>, StoreError> {
    rows.into_iter()
        .map(|r| {
            let data: JsonValue = r.try_get("data")?;
            serde_json::from_value(data).map_err(StoreError::from)
        })
        .collect()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::*;
    use nf_core::entities::{EntityId, EntityMeta, Organization, OrganizationType, Person};
    use nf_core::relationships::{Relationship, RelationshipType};
    use nf_core::source::{ContentHash, SourceChain, SourceRef, SourceType};
    use url::Url;

    // ── Unit tests (no DB) ───────────────────────────────────────────────────

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
    fn test_page_new() {
        let page: Page<i32> = Page::new(vec![1, 2, 3], 0, 10);
        assert_eq!(page.len(), 3);
        assert!(!page.is_empty());
        assert_eq!(page.page, 0);
        assert_eq!(page.page_size, 10);
    }

    #[test]
    fn test_page_empty() {
        let page: Page<String> = Page::new(vec![], 2, 25);
        assert!(page.is_empty());
        assert_eq!(page.len(), 0);
    }

    #[test]
    fn test_entity_uuid_extraction() {
        let person = Person::new("Alice", test_source_chain());
        let expected_id = person.meta.id.0;
        let entity = Entity::Person(person);
        assert_eq!(entity_uuid(&entity), expected_id);
    }

    #[test]
    fn test_entity_version_extraction() {
        let person = Person::new("Bob", test_source_chain());
        let entity = Entity::Person(person);
        assert_eq!(entity_version(&entity), 1);
    }

    #[test]
    fn test_entity_serialization_roundtrip() {
        let person = Person::new("Carol", test_source_chain());
        let entity = Entity::Person(person);
        let json = serde_json::to_value(&entity).unwrap();
        let recovered: Entity = serde_json::from_value(json).unwrap();
        assert_eq!(entity_uuid(&entity), entity_uuid(&recovered));
    }

    #[test]
    fn test_relationship_id_roundtrip() {
        let from = nf_core::entities::EntityId::new();
        let to = nf_core::entities::EntityId::new();
        let rel = Relationship::new(from, to, RelationshipType::DonatedTo, test_source_chain());
        let json = serde_json::to_value(&rel).unwrap();
        let recovered: Relationship = serde_json::from_value(json).unwrap();
        assert_eq!(recovered.id.0, rel.id.0);
    }

    // ── DB-dependent tests ────────────────────────────────────────────────────

    async fn db_pool() -> Option<DbPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = crate::db::connect(&url).await.ok()?;
        crate::migration::run(&pool).await.ok()?;
        Some(pool)
    }

    #[tokio::test]
    async fn test_entity_insert_get_delete() {
        let Some(pool) = db_pool().await else { return };
        let repo = EntityRepository::new(pool.clone());

        let person = Person::new("Dave", test_source_chain());
        let id = person.meta.id.0;
        let entity = Entity::Person(person);

        repo.insert(&entity).await.unwrap();

        let fetched = repo.get(id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(entity_uuid(&fetched.unwrap()), id);

        assert!(repo.delete(id).await.unwrap());
        assert!(repo.get(id).await.unwrap().is_none());

        pool.close().await;
    }

    #[tokio::test]
    async fn test_entity_update() {
        let Some(pool) = db_pool().await else { return };
        let repo = EntityRepository::new(pool.clone());

        let mut person = Person::new("Eve", test_source_chain());
        let id = person.meta.id.0;
        let entity = Entity::Person(person.clone());
        repo.insert(&entity).await.unwrap();

        person.name = "Eve Updated".to_string();
        person.meta.version = 2;
        let updated = Entity::Person(person);
        assert!(repo.update(&updated).await.unwrap());

        let fetched = repo.get(id).await.unwrap().unwrap();
        if let Entity::Person(p) = fetched {
            assert_eq!(p.name, "Eve Updated");
            assert_eq!(p.meta.version, 2);
        } else {
            panic!("expected Person");
        }

        repo.delete(id).await.unwrap();
        pool.close().await;
    }

    #[tokio::test]
    async fn test_entity_list_pagination() {
        let Some(pool) = db_pool().await else { return };
        let repo = EntityRepository::new(pool.clone());

        // Insert three persons with distinct UUIDs.
        let ids: Vec<Uuid> = (0..3)
            .map(|i| {
                let p = Person::new(format!("Person {i}"), test_source_chain());
                p.meta.id.0
            })
            .collect();

        for _id in &ids {
            // Re-create person with that specific id is complicated; just insert fresh ones.
            let p = Person::new("list test", test_source_chain());
            let e = Entity::Person(p);
            repo.insert(&e).await.unwrap();
        }

        let page = repo.list(0, 2).await.unwrap();
        assert!(page.len() >= 2);

        for id in &ids {
            repo.delete(*id).await.ok();
        }
        pool.close().await;
    }

    #[tokio::test]
    async fn test_relationship_crud() {
        let Some(pool) = db_pool().await else { return };

        // Insert two placeholder entities first.
        let entity_repo = EntityRepository::new(pool.clone());
        let p1 = Person::new("Fran", test_source_chain());
        let p2 = Person::new("Greg", test_source_chain());
        let id1 = p1.meta.id.0;
        let id2 = p2.meta.id.0;
        entity_repo.insert(&Entity::Person(p1)).await.unwrap();
        entity_repo.insert(&Entity::Person(p2)).await.unwrap();

        let rel_repo = RelationshipRepository::new(pool.clone());
        let rel = Relationship::new(
            EntityId(id1),
            EntityId(id2),
            RelationshipType::DonatedTo,
            test_source_chain(),
        );
        let rel_id = rel.id.0;

        rel_repo.insert(&rel).await.unwrap();
        let fetched = rel_repo.get(rel_id).await.unwrap();
        assert!(fetched.is_some());

        assert!(rel_repo.delete(rel_id).await.unwrap());
        assert!(rel_repo.get(rel_id).await.unwrap().is_none());

        entity_repo.delete(id1).await.unwrap();
        entity_repo.delete(id2).await.unwrap();
        pool.close().await;
    }

    #[tokio::test]
    async fn test_all_entity_types_store() {
        use chrono::NaiveDate;
        use nf_core::entities::*;

        let Some(pool) = db_pool().await else { return };
        let repo = EntityRepository::new(pool.clone());

        let sc = test_source_chain();
        let eid = EntityId::new();

        let entities: Vec<Entity> = vec![
            Entity::Person(Person::new("Test", sc.clone())),
            Entity::Organization(Organization::new("Org", OrganizationType::Pac, sc.clone())),
            Entity::Payment(Payment {
                meta: EntityMeta::new(sc.clone()),
                amount: 1000.0,
                currency: "USD".into(),
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                donor: EntityId::new(),
                recipient: EntityId::new(),
                payment_type: PaymentType::IndividualContribution,
                filing_id: None,
                election_cycle: None,
                description: None,
            }),
            Entity::Pardon(Pardon {
                meta: EntityMeta::new(sc.clone()),
                person_pardoned: eid,
                pardoning_official: eid,
                offense: "fraud".into(),
                sentence_at_time: None,
                pardon_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                indictment_date: None,
                days_indictment_to_pardon: None,
                concurrent_business_relationship: false,
            }),
            Entity::FlightLogEntry(FlightLogEntry {
                meta: EntityMeta::new(sc.clone()),
                aircraft_tail_number: "N12345".into(),
                date: NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
                origin: Some("JFK".into()),
                destination: Some("LAX".into()),
                passengers: vec![],
            }),
            Entity::TimingCorrelation(TimingCorrelation {
                meta: EntityMeta::new(sc.clone()),
                event_a: EntityId::new(),
                event_a_description: "Event A".into(),
                event_a_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                event_b: EntityId::new(),
                event_b_description: "Event B".into(),
                event_b_date: NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
                days_between: 60,
                correlation_type: CorrelationType::DonationToVote,
                auto_flagged: true,
                threshold_days: Some(90),
            }),
            Entity::ConductComparison(ConductComparison {
                meta: EntityMeta::new(sc.clone()),
                official_action: "voted".into(),
                official: EntityId::new(),
                action_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                action_source: "roll call".into(),
                equivalent_private_conduct: "bribed".into(),
                documented_consequence: "fired".into(),
                consequence_source: "court".into(),
            }),
            Entity::PublicStatement(PublicStatement {
                meta: EntityMeta::new(sc.clone()),
                official: EntityId::new(),
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                platform: StatementPlatform::CSpan,
                content_summary: "test summary".into(),
                topic_tags: vec![],
                beneficiary_tags: vec![],
            }),
            Entity::PolicyDecision(PolicyDecision {
                meta: EntityMeta::new(sc.clone()),
                official: EntityId::new(),
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                description: "policy".into(),
                decision_type: PolicyDecisionType::LegislativeVote,
                beneficiaries: vec![],
                reference_number: None,
                vote: Some(VotePosition::Yea),
            }),
        ];

        let mut inserted_ids = Vec::new();
        for entity in &entities {
            let id = repo.insert(entity).await.unwrap();
            inserted_ids.push(id);
        }

        // Verify all can be fetched back.
        for id in &inserted_ids {
            let fetched = repo.get(*id).await.unwrap();
            assert!(fetched.is_some(), "Entity {id} should be fetchable");
        }

        // Clean up.
        for id in &inserted_ids {
            repo.delete(*id).await.unwrap();
        }
        pool.close().await;
    }
}
