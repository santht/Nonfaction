use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::StoreError;

// ─── Types ────────────────────────────────────────────────────────────────────

/// The genesis prev_hash used for the very first audit entry.
pub const GENESIS_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditOperation {
    Insert,
    Update,
    Delete,
}

impl AuditOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Insert => "Insert",
            Self::Update => "Update",
            Self::Delete => "Delete",
        }
    }
}

impl std::fmt::Display for AuditOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single entry in the hash-chained audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub seq: i64,
    pub timestamp: DateTime<Utc>,
    pub operation: AuditOperation,
    pub entity_type: String,
    pub entity_id: Uuid,
    /// SHA-256 of the serialized entity JSON at the time of the mutation.
    pub data_hash: String,
    /// `entry_hash` of the previous entry (GENESIS_HASH for first entry).
    pub prev_hash: String,
    /// SHA-256 over all other fields — links this entry into the chain.
    pub entry_hash: String,
}

// ─── Pure hash computation ────────────────────────────────────────────────────

/// Compute `entry_hash` deterministically from the entry's fields.
///
/// This is a pure function and can be tested without a database.
pub fn compute_entry_hash(
    id: Uuid,
    timestamp: &DateTime<Utc>,
    operation: AuditOperation,
    entity_type: &str,
    entity_id: Uuid,
    data_hash: &str,
    prev_hash: &str,
) -> String {
    let mut h = Sha256::new();
    h.update(id.as_bytes());
    h.update(timestamp.to_rfc3339().as_bytes());
    h.update(operation.as_str().as_bytes());
    h.update(entity_type.as_bytes());
    h.update(entity_id.as_bytes());
    h.update(data_hash.as_bytes());
    h.update(prev_hash.as_bytes());
    format!("{:x}", h.finalize())
}

/// Compute the SHA-256 hash of a serialized entity payload.
pub fn hash_entity_data(data: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(data).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(&bytes);
    format!("{:x}", h.finalize())
}

// ─── AuditLog ────────────────────────────────────────────────────────────────

/// Append-only, hash-chained audit log backed by PostgreSQL.
pub struct AuditLog {
    pool: DbPool,
}

impl AuditLog {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Record a mutation. Returns the new `AuditEntry` (including computed hashes).
    pub async fn append(
        &self,
        operation: AuditOperation,
        entity_type: &str,
        entity_id: Uuid,
        data: &serde_json::Value,
    ) -> Result<AuditEntry, StoreError> {
        let prev_hash = self.get_last_entry_hash().await?;
        let id = Uuid::new_v4();
        let timestamp = Utc::now();
        let data_hash = hash_entity_data(data);
        let entry_hash = compute_entry_hash(
            id,
            &timestamp,
            operation,
            entity_type,
            entity_id,
            &data_hash,
            &prev_hash,
        );

        let row = sqlx::query(
            r#"
            INSERT INTO audit_log (id, timestamp, operation, entity_type, entity_id, data_hash, prev_hash, entry_hash)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING seq
            "#,
        )
        .bind(id)
        .bind(timestamp)
        .bind(operation.as_str())
        .bind(entity_type)
        .bind(entity_id)
        .bind(&data_hash)
        .bind(&prev_hash)
        .bind(&entry_hash)
        .fetch_one(&self.pool)
        .await?;

        let seq: i64 = row.try_get("seq")?;

        Ok(AuditEntry {
            id,
            seq,
            timestamp,
            operation,
            entity_type: entity_type.to_string(),
            entity_id,
            data_hash,
            prev_hash,
            entry_hash,
        })
    }

    /// Retrieve all audit entries for a specific entity, ordered by sequence.
    pub async fn get_for_entity(&self, entity_id: Uuid) -> Result<Vec<AuditEntry>, StoreError> {
        let rows = sqlx::query(
            "SELECT seq, id, timestamp, operation, entity_type, entity_id, data_hash, prev_hash, entry_hash \
             FROM audit_log WHERE entity_id = $1 ORDER BY seq ASC",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_entry).collect()
    }

    /// Retrieve all audit entries in chain order.
    pub async fn all_entries(&self) -> Result<Vec<AuditEntry>, StoreError> {
        let rows = sqlx::query(
            "SELECT seq, id, timestamp, operation, entity_type, entity_id, data_hash, prev_hash, entry_hash \
             FROM audit_log ORDER BY seq ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_entry).collect()
    }

    /// Verify the entire audit chain for tamper evidence.
    ///
    /// Returns `true` if the chain is intact, `false` if any entry has been
    /// modified or the prev_hash links are broken.
    pub async fn verify_chain(&self) -> Result<bool, StoreError> {
        let entries = self.all_entries().await?;
        let mut expected_prev = GENESIS_HASH.to_string();

        for entry in &entries {
            // Check prev_hash links to preceding entry.
            if entry.prev_hash != expected_prev {
                tracing::warn!(
                    entry_id = %entry.id,
                    "Audit chain broken: prev_hash mismatch"
                );
                return Ok(false);
            }

            // Recompute entry_hash and compare.
            let computed = compute_entry_hash(
                entry.id,
                &entry.timestamp,
                entry.operation,
                &entry.entity_type,
                entry.entity_id,
                &entry.data_hash,
                &entry.prev_hash,
            );
            if computed != entry.entry_hash {
                tracing::warn!(
                    entry_id = %entry.id,
                    "Audit chain broken: entry_hash mismatch"
                );
                return Ok(false);
            }

            expected_prev = entry.entry_hash.clone();
        }

        Ok(true)
    }

    // ── Private ──────────────────────────────────────────────────────────────

    async fn get_last_entry_hash(&self) -> Result<String, StoreError> {
        let row = sqlx::query(
            "SELECT entry_hash FROM audit_log ORDER BY seq DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(r.try_get("entry_hash")?),
            None => Ok(GENESIS_HASH.to_string()),
        }
    }
}

fn row_to_entry(row: sqlx::postgres::PgRow) -> Result<AuditEntry, StoreError> {
    let operation_str: String = row.try_get("operation")?;
    let operation = match operation_str.as_str() {
        "Insert" => AuditOperation::Insert,
        "Update" => AuditOperation::Update,
        "Delete" => AuditOperation::Delete,
        other => {
            return Err(StoreError::Integrity(format!(
                "Unknown audit operation: {other}"
            )))
        }
    };

    Ok(AuditEntry {
        seq: row.try_get("seq")?,
        id: row.try_get("id")?,
        timestamp: row.try_get("timestamp")?,
        operation,
        entity_type: row.try_get("entity_type")?,
        entity_id: row.try_get("entity_id")?,
        data_hash: row.try_get("data_hash")?,
        prev_hash: row.try_get("prev_hash")?,
        entry_hash: row.try_get("entry_hash")?,
    })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Pure / no-DB tests ───────────────────────────────────────────────────

    #[test]
    fn test_genesis_hash_length() {
        assert_eq!(GENESIS_HASH.len(), 64);
    }

    #[test]
    fn test_compute_entry_hash_deterministic() {
        let id = Uuid::new_v4();
        let ts = Utc::now();
        let entity_id = Uuid::new_v4();

        let h1 = compute_entry_hash(
            id, &ts, AuditOperation::Insert, "Person", entity_id, "data_hash", GENESIS_HASH,
        );
        let h2 = compute_entry_hash(
            id, &ts, AuditOperation::Insert, "Person", entity_id, "data_hash", GENESIS_HASH,
        );
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_compute_entry_hash_varies_by_operation() {
        let id = Uuid::new_v4();
        let ts = Utc::now();
        let entity_id = Uuid::new_v4();

        let h_insert = compute_entry_hash(
            id, &ts, AuditOperation::Insert, "Person", entity_id, "dh", GENESIS_HASH,
        );
        let h_update = compute_entry_hash(
            id, &ts, AuditOperation::Update, "Person", entity_id, "dh", GENESIS_HASH,
        );
        assert_ne!(h_insert, h_update);
    }

    #[test]
    fn test_compute_entry_hash_chaining() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let ts = Utc::now();
        let eid = Uuid::new_v4();

        let h1 = compute_entry_hash(
            id1, &ts, AuditOperation::Insert, "Person", eid, "dh1", GENESIS_HASH,
        );
        // Second entry uses h1 as prev_hash.
        let h2 = compute_entry_hash(
            id2, &ts, AuditOperation::Update, "Person", eid, "dh2", &h1,
        );
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_entity_data_deterministic() {
        let data = serde_json::json!({"name": "Jane Doe", "version": 1});
        let h1 = hash_entity_data(&data);
        let h2 = hash_entity_data(&data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_hash_entity_data_sensitive_to_content() {
        let d1 = serde_json::json!({"name": "Alice"});
        let d2 = serde_json::json!({"name": "Bob"});
        assert_ne!(hash_entity_data(&d1), hash_entity_data(&d2));
    }

    #[test]
    fn test_audit_operation_display() {
        assert_eq!(AuditOperation::Insert.to_string(), "Insert");
        assert_eq!(AuditOperation::Update.to_string(), "Update");
        assert_eq!(AuditOperation::Delete.to_string(), "Delete");
    }

    // ── DB-dependent tests (skipped without DATABASE_URL) ────────────────────

    #[tokio::test]
    async fn test_audit_log_append_and_verify() {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let pool = crate::db::connect(&url).await.expect("connect");
        crate::migration::run(&pool).await.expect("migrate");

        let log = AuditLog::new(pool.clone());
        let entity_id = Uuid::new_v4();
        let data = serde_json::json!({"name": "Test Person"});

        let entry = log
            .append(AuditOperation::Insert, "Person", entity_id, &data)
            .await
            .expect("append");

        assert_eq!(entry.entity_id, entity_id);
        assert_eq!(entry.operation, AuditOperation::Insert);
        assert_eq!(entry.prev_hash, GENESIS_HASH);

        assert!(log.verify_chain().await.expect("verify_chain"));
        pool.close().await;
    }

    #[tokio::test]
    async fn test_audit_log_chain_links() {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let pool = crate::db::connect(&url).await.expect("connect");
        crate::migration::run(&pool).await.expect("migrate");

        let log = AuditLog::new(pool.clone());
        let entity_id = Uuid::new_v4();
        let data = serde_json::json!({"test": true});

        let e1 = log.append(AuditOperation::Insert, "Payment", entity_id, &data).await.unwrap();
        let e2 = log.append(AuditOperation::Update, "Payment", entity_id, &data).await.unwrap();

        // e2's prev_hash must equal e1's entry_hash.
        assert_eq!(e2.prev_hash, e1.entry_hash);
        assert!(log.verify_chain().await.unwrap());
        pool.close().await;
    }
}
