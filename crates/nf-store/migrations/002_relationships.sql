-- Relationships table: directed, typed edges between entities.
CREATE TABLE IF NOT EXISTS relationships (
    id          UUID PRIMARY KEY,
    from_entity UUID      NOT NULL,
    to_entity   UUID      NOT NULL,
    rel_type    TEXT      NOT NULL,
    version     BIGINT    NOT NULL DEFAULT 1,
    data        JSONB     NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS relationships_from_idx ON relationships (from_entity);
CREATE INDEX IF NOT EXISTS relationships_to_idx   ON relationships (to_entity);
CREATE INDEX IF NOT EXISTS relationships_type_idx ON relationships (rel_type);
