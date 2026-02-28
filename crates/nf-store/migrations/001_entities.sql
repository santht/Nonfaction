-- Entities table: stores all entity types as JSONB blobs with type discriminator.
-- One table for all entity variants keeps the schema simple and uniform.
CREATE TABLE IF NOT EXISTS entities (
    id          UUID PRIMARY KEY,
    entity_type TEXT      NOT NULL,
    version     BIGINT    NOT NULL DEFAULT 1,
    data        JSONB     NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS entities_type_idx       ON entities (entity_type);
CREATE INDEX IF NOT EXISTS entities_created_at_idx ON entities (created_at DESC);
CREATE INDEX IF NOT EXISTS entities_updated_at_idx ON entities (updated_at DESC);
