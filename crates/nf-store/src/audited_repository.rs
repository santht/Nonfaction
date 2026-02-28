//! Repository wrappers that automatically append audit log entries on every mutation.
//!
//! These wrap the raw [`EntityRepository`] and [`RelationshipRepository`] and ensure
//! the tamper-evident hash chain is maintained without callers needing to remember.

use uuid::Uuid;

use crate::audit::{AuditLog, AuditOperation};
use crate::db::DbPool;
use crate::error::StoreError;
use crate::repository::{EntityRepository, Page, RelationshipRepository, Repository};

use nf_core::entities::Entity;
use nf_core::relationships::Relationship;

// ─── AuditedEntityRepository ────────────────────────────────────────────────

/// Entity repository that automatically appends audit log entries on every
/// insert, update, and delete.
pub struct AuditedEntityRepository {
    inner: EntityRepository,
    audit: AuditLog,
}

impl AuditedEntityRepository {
    pub fn new(pool: DbPool) -> Self {
        Self {
            inner: EntityRepository::new(pool.clone()),
            audit: AuditLog::new(pool),
        }
    }

    /// Access the underlying entity repository for read-only operations.
    pub fn inner(&self) -> &EntityRepository {
        &self.inner
    }

    /// Access the audit log directly (e.g. for verification).
    pub fn audit_log(&self) -> &AuditLog {
        &self.audit
    }

    pub async fn list_by_type(
        &self,
        entity_type: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Entity>, StoreError> {
        self.inner.list_by_type(entity_type, page, page_size).await
    }
}

impl Repository<Entity> for AuditedEntityRepository {
    async fn insert(&self, entity: &Entity) -> Result<Uuid, StoreError> {
        let id = self.inner.insert(entity).await?;
        let data = serde_json::to_value(entity)?;
        self.audit
            .append(AuditOperation::Insert, entity.type_name(), id, &data)
            .await?;
        Ok(id)
    }

    async fn get(&self, id: Uuid) -> Result<Option<Entity>, StoreError> {
        self.inner.get(id).await
    }

    async fn update(&self, entity: &Entity) -> Result<bool, StoreError> {
        let updated = self.inner.update(entity).await?;
        if updated {
            let id = entity.entity_id().0;
            let data = serde_json::to_value(entity)?;
            self.audit
                .append(AuditOperation::Update, entity.type_name(), id, &data)
                .await?;
        }
        Ok(updated)
    }

    async fn delete(&self, id: Uuid) -> Result<bool, StoreError> {
        // Fetch entity before deleting to get type name for audit.
        let entity = self.inner.get(id).await?;
        let deleted = self.inner.delete(id).await?;
        if deleted {
            let entity_type = entity.as_ref().map(|e| e.type_name()).unwrap_or("Unknown");
            let data = serde_json::json!({ "deleted_id": id });
            self.audit
                .append(AuditOperation::Delete, entity_type, id, &data)
                .await?;
        }
        Ok(deleted)
    }

    async fn list(&self, page: u32, page_size: u32) -> Result<Page<Entity>, StoreError> {
        self.inner.list(page, page_size).await
    }
}

// ─── AuditedRelationshipRepository ──────────────────────────────────────────

/// Relationship repository that automatically appends audit log entries.
pub struct AuditedRelationshipRepository {
    inner: RelationshipRepository,
    audit: AuditLog,
}

impl AuditedRelationshipRepository {
    pub fn new(pool: DbPool) -> Self {
        Self {
            inner: RelationshipRepository::new(pool.clone()),
            audit: AuditLog::new(pool),
        }
    }

    pub fn inner(&self) -> &RelationshipRepository {
        &self.inner
    }

    pub async fn list_from(
        &self,
        from_entity: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Relationship>, StoreError> {
        self.inner.list_from(from_entity, page, page_size).await
    }

    pub async fn list_to(
        &self,
        to_entity: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Relationship>, StoreError> {
        self.inner.list_to(to_entity, page, page_size).await
    }
}

impl Repository<Relationship> for AuditedRelationshipRepository {
    async fn insert(&self, rel: &Relationship) -> Result<Uuid, StoreError> {
        let id = self.inner.insert(rel).await?;
        let data = serde_json::to_value(rel)?;
        self.audit
            .append(
                AuditOperation::Insert,
                &format!("Relationship:{:?}", rel.rel_type),
                id,
                &data,
            )
            .await?;
        Ok(id)
    }

    async fn get(&self, id: Uuid) -> Result<Option<Relationship>, StoreError> {
        self.inner.get(id).await
    }

    async fn update(&self, rel: &Relationship) -> Result<bool, StoreError> {
        let updated = self.inner.update(rel).await?;
        if updated {
            let data = serde_json::to_value(rel)?;
            self.audit
                .append(
                    AuditOperation::Update,
                    &format!("Relationship:{:?}", rel.rel_type),
                    rel.id.0,
                    &data,
                )
                .await?;
        }
        Ok(updated)
    }

    async fn delete(&self, id: Uuid) -> Result<bool, StoreError> {
        let rel = self.inner.get(id).await?;
        let deleted = self.inner.delete(id).await?;
        if deleted {
            let rel_type = rel
                .as_ref()
                .map(|r| format!("Relationship:{:?}", r.rel_type))
                .unwrap_or_else(|| "Relationship:Unknown".to_string());
            let data = serde_json::json!({ "deleted_id": id });
            self.audit
                .append(AuditOperation::Delete, &rel_type, id, &data)
                .await?;
        }
        Ok(deleted)
    }

    async fn list(&self, page: u32, page_size: u32) -> Result<Page<Relationship>, StoreError> {
        self.inner.list(page, page_size).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::GENESIS_HASH;

    // Pure unit tests (no DB) — verify the wrappers compile and types are correct.

    #[test]
    fn test_audited_entity_repo_new() {
        // This just verifies the type compiles. Actual DB tests require DATABASE_URL.
        let _: fn(DbPool) -> AuditedEntityRepository = AuditedEntityRepository::new;
    }

    #[test]
    fn test_audited_relationship_repo_new() {
        let _: fn(DbPool) -> AuditedRelationshipRepository = AuditedRelationshipRepository::new;
    }

    // ── DB-dependent tests ──────────────────────────────────────────────────

    async fn db_pool() -> Option<DbPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = crate::db::connect(&url).await.ok()?;
        crate::migration::run(&pool).await.ok()?;
        Some(pool)
    }

    #[tokio::test]
    async fn test_audited_entity_insert_creates_audit_entry() {
        use nf_core::entities::{Entity, Person};
        use nf_core::source::{ContentHash, SourceChain, SourceRef, SourceType};
        use url::Url;

        let Some(pool) = db_pool().await else { return };
        let repo = AuditedEntityRepository::new(pool.clone());

        let source = SourceRef::new(
            Url::parse("https://example.gov/test").unwrap(),
            ContentHash::compute(b"test"),
            SourceType::FecFiling,
            "test",
        );
        let person = Person::new("Audit Test", SourceChain::new(source));
        let id = person.meta.id.0;
        let entity = Entity::Person(person);

        repo.insert(&entity).await.unwrap();

        // Verify audit entry exists
        let entries = repo.audit_log().get_for_entity(id).await.unwrap();
        assert!(!entries.is_empty());
        assert_eq!(entries[0].operation, AuditOperation::Insert);
        assert_eq!(entries[0].entity_type, "Person");

        // Clean up
        repo.delete(id).await.unwrap();

        // Delete should also be audited
        let entries = repo.audit_log().get_for_entity(id).await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].operation, AuditOperation::Delete);

        pool.close().await;
    }
}
