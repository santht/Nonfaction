-- Add foreign key constraints to relationships table for referential integrity.
-- These enforce that relationship endpoints must reference existing entities.
ALTER TABLE relationships
    ADD CONSTRAINT fk_relationships_from_entity
        FOREIGN KEY (from_entity) REFERENCES entities(id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_relationships_to_entity
        FOREIGN KEY (to_entity) REFERENCES entities(id) ON DELETE CASCADE;

-- GIN index on entities.data for efficient JSONB queries (e.g. filtering by name, amount).
CREATE INDEX IF NOT EXISTS idx_entities_data_gin ON entities USING gin (data jsonb_path_ops);

-- Partial index on audit_log for quick lookups of recent entries.
CREATE INDEX IF NOT EXISTS idx_audit_log_recent
    ON audit_log (timestamp DESC)
    WHERE timestamp > NOW() - INTERVAL '30 days';

-- Unique constraint on audit_log.seq to guarantee chain ordering.
ALTER TABLE audit_log
    ADD CONSTRAINT audit_log_seq_unique UNIQUE (seq);

-- Add entity_type index to submissions for filtering by submission type.
CREATE INDEX IF NOT EXISTS idx_submissions_type ON submissions (submission_type);
